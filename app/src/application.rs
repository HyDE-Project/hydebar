//! Assembly of the iced application around the bar.

use std::{borrow::Cow, num::NonZeroUsize, path::PathBuf, sync::Arc};

use clap::Parser;
use hydebar_core::{
    adapters::hyprland_client::HyprlandClient,
    config::{ConfigManager, get_config},
    event_bus::EventBus
};
use hydebar_gui::{App, get_log_spec};
use hydebar_proto::ports::hyprland::HyprlandPort;
use iced::{Anchor, Font, KeyboardInteractivity, Layer, LayerShellSettings, SurfaceId};
use log::debug;
use tokio::runtime::Handle;

use crate::{error::MainError, housekeeping, instance, logging, startup_scale};

const ICON_FONT: &[u8] = include_bytes!("../../assets/SymbolsNerdFont-Regular.ttf");

/// The notice the licence asks a program to state where it can be read.
const LICENCE_NOTICE: &str = "hydebar is free software under the GNU General Public License, \
                              version 3 or later, and comes with absolutely no warranty. See \
                              the LICENSE file shipped with it, or \
                              <https://www.gnu.org/licenses/gpl-3.0.html>.";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, after_help = LICENCE_NOTICE)]
struct Args {
    #[arg(short, long, value_parser = clap::value_parser!(PathBuf))]
    config_path: Option<PathBuf>
}

/// Surrenders `deps` to the first boot call and refuses a second.
///
/// iced's boot contract offers no error path, so the closure can only die
/// loudly on the programming error of being called twice.
fn boot_once<D, A>(deps: D, build: fn(D) -> A) -> impl Fn() -> A {
    let deps = std::cell::Cell::new(Some(deps));

    #[expect(
        clippy::expect_used,
        reason = "iced's boot contract offers no error path for a repeated call"
    )]
    move || deps.take().map(build).expect("boot called more than once")
}

/// Reads the configuration, becomes the single instance and runs the bar.
///
/// The bridge's own first surface is not one of the bar's: it is parked as an
/// off-screen pixel in the background, and the bar's real surfaces are created
/// by the output handling.
pub fn run(runtime_handle: Handle) -> Result<(), MainError> {
    let args = Args::parse();
    debug!("args: {args:?}");

    let logger = logging::init()?;

    let (raw_config, config_path) = get_config(args.config_path)?;

    let instance_lock = instance::acquire()?;
    debug!("instance lock held at {}", instance_lock.path().display());

    housekeeping::reap_and_guard_children();

    let mut raw_config = raw_config;

    let magnification = if raw_config.appearance.auto_scale {
        startup_scale::focused_screen().map_or(1.0, startup_scale::ScreenGeometry::magnification)
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

    let font = config
        .appearance
        .font_name
        .as_ref()
        .map_or(Font::DEFAULT, |font_name| {
            Font::with_name(Box::leak(font_name.clone().into_boxed_str()))
        });

    let hyprland: Arc<dyn HyprlandPort> = Arc::new(HyprlandClient::new());

    let bus_capacity = NonZeroUsize::new(64).ok_or(MainError::BusCapacity)?;
    let event_bus = EventBus::new(bus_capacity);
    let event_sender = event_bus.sender();
    let bus_receiver = event_bus.receiver();

    let boot = boot_once(
        (
            logger,
            config,
            config_manager,
            config_path,
            hyprland,
            event_sender,
            runtime_handle,
            bus_receiver
        ),
        App::new
    );

    let outcome = iced::application(
        boot,
        |app: &mut App, message| app.update(message),
        |app: &App, id| app.view(id)
    )
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
