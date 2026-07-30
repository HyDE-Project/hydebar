#![allow(mismatched_lifetime_syntaxes)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::double_ended_iterator_last)]

mod error;
mod instance;
mod startup_scale;

use std::{
    backtrace::Backtrace, borrow::Cow, num::NonZeroUsize, panic, path::PathBuf, process::ExitCode,
    sync::Arc
};

use clap::Parser;
use flexi_logger::{Age, Cleanup, Criterion, FileSpec, LogSpecBuilder, Logger, Naming};
use hydebar_core::{
    adapters::hyprland_client::HyprlandClient,
    config::{ConfigManager, get_config},
    event_bus::EventBus
};
use hydebar_gui::{App, get_log_spec};
use hydebar_proto::ports::hyprland::HyprlandPort;
use iced::{Anchor, Font, KeyboardInteractivity, Layer, LayerShellSettings, SurfaceId};
use log::{debug, error};
use tokio::runtime::Handle;

use crate::error::MainError;

const ICON_FONT: &[u8] = include_bytes!("../../../assets/SymbolsNerdFont-Regular.ttf");

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_parser = clap::value_parser!(PathBuf))]
    config_path: Option<PathBuf>
}

/// Starts the async runtime and hands its handle to the bar.
///
/// The event loop must not run inside the runtime: the graphics layer blocks
/// the calling thread while creating the compositor, and blocking a thread that
/// is already driving tasks aborts the process.
fn main() -> ExitCode {
    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err}");
            eprintln!("hydebar: {err}");

            ExitCode::FAILURE
        }
    }
}

/// Worker threads backing every asynchronous source the bar owns.
///
/// The default pool is sized to the CPU count, which on a desktop machine means
/// dozens of workers for a process whose entire workload is parking on D-Bus,
/// Wayland, Hyprland and child-process pipes. A fixed handful covers the only
/// tasks that ever hold a worker for longer than a poll — the synchronous
/// Hyprland round-trips issued from the listener handlers and the `sysinfo`
/// sampler reading `/proc` — while leaving spare capacity so none of them can
/// starve the others.
const RUNTIME_WORKER_THREADS: usize = 4;

/// Builds the runtime and runs the bar on it.
fn start() -> Result<(), MainError> {
    let workers = std::env::var("HYDEBAR_RUNTIME_THREADS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(RUNTIME_WORKER_THREADS);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .thread_name("hydebar-rt")
        .enable_all()
        .build()
        .map_err(MainError::Runtime)?;
    let runtime_handle = runtime.handle().clone();

    run(runtime_handle)
}

/// Clears the strays of earlier runs and arms the reaper for this one.
///
/// Ordered after the instance lock on purpose: the bar that was running has
/// already been asked to quit and released the slot, so everything still
/// wearing an older launch stamp is genuinely abandoned. Both steps are best
/// effort — a bar that cannot sweep or cannot install its handler is still a
/// working bar, and its children are covered by the parent death signal the
/// kernel enforces on each of them.
fn reap_and_guard_children() {
    let swept = hydebar_core::utils::process_group::sweep_orphans();

    if swept > 0 {
        debug!("ended {swept} processes left behind by an earlier run");
    }

    if let Err(err) = hydebar_core::utils::process_group::claim_orphans() {
        error!("failed to claim orphaned children, some may escape the bar: {err}");
    }

    if let Err(err) = hydebar_core::utils::process_group::install_termination_handler() {
        error!("failed to arm the process reaper, a signalled exit may leave children: {err}");
    }
}

fn run(runtime_handle: Handle) -> Result<(), MainError> {
    let args = Args::parse();
    debug!("args: {args:?}");

    let logger = Logger::with(
        LogSpecBuilder::new()
            .default(log::LevelFilter::Info)
            .build()
    )
    .log_to_file(FileSpec::default().directory("/tmp/hydebar"))
    .duplicate_to_stdout(flexi_logger::Duplicate::All)
    .rotate(
        Criterion::Age(Age::Day),
        Naming::Timestamps,
        Cleanup::KeepLogFiles(7)
    );
    let logger = if cfg!(debug_assertions) {
        logger.duplicate_to_stdout(flexi_logger::Duplicate::All)
    } else {
        logger
    };
    let logger = logger.start()?;
    panic::set_hook(Box::new(|info| {
        let b = Backtrace::capture();
        error!("Panic: {info} \n {b}");
    }));

    let (raw_config, config_path) = get_config(args.config_path)?;

    let instance_lock = instance::acquire()?;
    debug!("instance lock held at {:?}", instance_lock.path());

    reap_and_guard_children();

    let mut raw_config = raw_config;

    let magnification = if raw_config.appearance.auto_scale {
        startup_scale::focused_screen().map_or(1.0, |screen| screen.magnification())
    } else {
        1.0
    };

    if magnification > 1.0 {
        debug!("magnifying the bar {magnification} times for this screen");
        hydebar_core::components::scale::set_screen_factor(magnification);
    }

    raw_config.appearance.adopt_screen(
        magnification,
        &hydebar_proto::compositor_look::CompositorLook::read()
    );

    hydebar_core::components::scale::set_base(raw_config.appearance.font_size_px());

    let config = Arc::new(raw_config);
    let config_manager = Arc::new(ConfigManager::new((*config).clone()));

    logger.set_new_spec(get_log_spec(&config.log_level));

    let font = match config.appearance.font_name {
        Some(ref font_name) => Font::with_name(Box::leak(font_name.clone().into_boxed_str())),
        None => Font::DEFAULT
    };

    let hyprland: Arc<dyn HyprlandPort> = Arc::new(HyprlandClient::new());

    let bus_capacity = NonZeroUsize::new(64).ok_or(MainError::BusCapacity)?;
    let event_bus = EventBus::new(bus_capacity);
    let event_sender = event_bus.sender();
    let bus_receiver = event_bus.receiver();

    let boot = {
        let deps = std::cell::Cell::new(Some((
            logger,
            config,
            config_manager,
            config_path,
            hyprland,
            event_sender,
            runtime_handle,
            bus_receiver
        )));
        move || {
            deps.take()
                .map(App::new)
                .expect("boot called more than once")
        }
    };

    let outcome = iced::application(
        boot,
        |app: &mut App, message| app.update(message),
        |app: &App, id| app.view(id)
    )
    // The bridge's own first surface is not one of the bar's: it is
    // parked as an off-screen pixel in the background, and the bar's
    // real surfaces are created by the output handling.
    .layer_shell(LayerShellSettings {
        namespace: "hydebar-boot-layer".to_owned(),
        size: Some((1, 1)),
        layer: Layer::Background,
        keyboard_interactivity: KeyboardInteractivity::None,
        exclusive_zone: 0,
        anchor: Anchor::TOP | Anchor::LEFT,
        margin: (-2, 0, 0, -2),
        ..Default::default()
    })
    .subscription(|app: &App| app.subscription())
    .theme(|app: &App| app.theme(SurfaceId::MAIN))
    .scale_factor(|app: &App| app.scale_factor(SurfaceId::MAIN))
    .font(Cow::from(ICON_FONT))
    .default_font(font)
    .run()
    .map_err(MainError::from);

    hydebar_core::utils::process_group::terminate_all();

    drop(instance_lock);

    outcome
}
