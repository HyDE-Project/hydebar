#![allow(mismatched_lifetime_syntaxes)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::double_ended_iterator_last)]

mod error;
mod executor;
mod instance;

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
use iced::{Font, Pixels, Settings};
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

    executor::install(runtime_handle.clone());

    run(runtime_handle)
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

    let config = Arc::new(raw_config);
    let config_manager = Arc::new(ConfigManager::new((*config).clone()));

    logger.set_new_spec(get_log_spec(&config.log_level));

    let font = match config.appearance.font_name {
        Some(ref font_name) => Font::with_name(Box::leak(font_name.clone().into_boxed_str())),
        None => Font::DEFAULT
    };

    let settings = Settings {
        default_text_size: config
            .appearance
            .font_size
            .map_or_else(|| Settings::default().default_text_size, Pixels::from),
        ..Settings::default()
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

    let outcome = iced::daemon(boot, App::update, App::view)
        .executor::<executor::SharedRuntime>()
        .settings(settings)
        .subscription(App::subscription)
        .theme(App::theme)
        .style(App::style)
        .scale_factor(App::scale_factor)
        .font(Cow::from(ICON_FONT))
        .default_font(font)
        .run()
        .map_err(MainError::from);

    drop(instance_lock);

    outcome
}
