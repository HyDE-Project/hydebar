//! Assembly of the iced application around the bar.

use std::{borrow::Cow, num::NonZeroUsize, path::PathBuf, sync::Arc};

use clap::Parser;
use hydebar_core::{
    adapters::hyprland_client::HyprlandClient,
    config::{Config, ConfigManager, RendererChoice, config},
    event_bus::EventBus
};
use hydebar_gui::{App, log_spec};
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

/// Sizes the bar to the screen it boots on: percentages, magnification and
/// the compositor's own look, all settled before the first frame.
fn apply_screen_look(raw_config: &mut Config) {
    let screen = startup_scale::focused_screen();

    if let Some(geo) = screen {
        let logical_height = geo.pixels.1 / geo.scale;
        raw_config.appearance.resolve_percentages(logical_height);
    }

    let magnification = if raw_config.appearance.auto_scale {
        screen.map_or(1.0, startup_scale::ScreenGeometry::magnification)
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
}

/// Points the renderer at the backend the configuration asks for.
///
/// The renderer reads this choice once, from the environment, before it
/// brings up its first frame; `auto` leaves the variable alone and lets the
/// renderer try the GPU and fall back to the CPU on its own. An
/// `ICED_BACKEND` already in the environment wins: that is what a packager
/// or a debugging user sets to overrule every configuration on the machine.
fn apply_renderer_choice(renderer: RendererChoice) {
    let Some(name) = renderer.backend_name() else {
        return;
    };

    if std::env::var_os("ICED_BACKEND").is_some() {
        return;
    }

    #[expect(
        unsafe_code,
        reason = "the choice is applied before the renderer reads it; the live runtime workers never touch the environment"
    )]
    unsafe {
        std::env::set_var("ICED_BACKEND", name);
    }
}

/// Whether a device type names real graphics hardware rather than a rasterizer.
///
/// `Cpu` is `lavapipe` or `SwiftShader`: a Vulkan device that exists but draws
/// on the processor. Naming Vulkan for such a device would trade the OpenGL
/// driver for software rasterization, which costs more, so only a real GPU
/// counts.
#[must_use]
const fn is_hardware_vulkan(device_type: wgpu::DeviceType) -> bool {
    !matches!(device_type, wgpu::DeviceType::Cpu)
}

/// Reports whether the machine exposes a hardware Vulkan device.
///
/// Builds a wgpu instance limited to the Vulkan backend and asks it what it
/// finds. The answer is the driver's own, not a guess from files on disk, so a
/// `true` here guarantees the renderer will find a Vulkan adapter moments
/// later. Any failure — no Vulkan, a driver that will not load — reads as
/// `false`, which leaves the bar on the backend it would have chosen anyway.
fn hardware_vulkan_present() -> bool {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    instance
        .enumerate_adapters(wgpu::Backends::VULKAN)
        .iter()
        .any(|adapter| is_hardware_vulkan(adapter.get_info().device_type))
}

/// Points the GPU renderer at Vulkan so it stops loading OpenGL alongside it.
///
/// wgpu's automatic backend initializes every backend it was compiled with to
/// enumerate adapters, which maps the Mesa OpenGL driver and starts its threads
/// even when the bar ends up drawing on Vulkan. Naming Vulkan when a hardware
/// Vulkan device is present leaves that driver unmapped — about nineteen MiB of
/// resident memory and two threads on a typical machine. When no hardware
/// Vulkan device answers, the variable is left alone and wgpu falls back to
/// OpenGL as before, so a machine without Vulkan loses nothing. A
/// `WGPU_BACKEND` already in the environment wins: it is what a packager or a
/// debugging user sets to overrule the probe.
fn prefer_vulkan_backend(renderer: RendererChoice) {
    if matches!(renderer, RendererChoice::Cpu) {
        return;
    }

    if std::env::var_os("WGPU_BACKEND").is_some() {
        return;
    }

    if hardware_vulkan_present() {
        #[expect(
            unsafe_code,
            reason = "applied before the renderer reads it; the live runtime workers never touch the environment"
        )]
        unsafe {
            std::env::set_var("WGPU_BACKEND", "vulkan");
        }
    }
}

/// Reads the configuration, becomes the single instance and runs the bar.
///
/// The bridge's own first surface is not one of the bar's: it is parked as an
/// off-screen pixel in the background, and the bar's real surfaces are created
/// by the output handling.
pub fn run(runtime_handle: Handle) -> Result<(), MainError> {
    let args = Args::parse();
    debug!("args: {args:?}");

    let logger = logging::init();

    let (raw_config, config_path) = config(args.config_path)?;

    let instance_lock = instance::acquire()?;
    debug!("instance lock held at {}", instance_lock.path().display());

    housekeeping::reap_and_guard_children();

    let mut raw_config = raw_config;
    apply_screen_look(&mut raw_config);
    apply_renderer_choice(raw_config.renderer);
    prefer_vulkan_backend(raw_config.renderer);

    let config = Arc::new(raw_config);
    let config_manager = Arc::new(ConfigManager::new((*config).clone()));

    let _ = logger.reload(log_spec(&config.log_level));

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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::is_hardware_vulkan;

    #[test]
    fn a_software_rasterizer_is_not_counted_as_hardware() {
        assert!(!is_hardware_vulkan(wgpu::DeviceType::Cpu));
    }

    #[test]
    fn every_real_device_type_counts_as_hardware() {
        for device_type in [
            wgpu::DeviceType::DiscreteGpu,
            wgpu::DeviceType::IntegratedGpu,
            wgpu::DeviceType::VirtualGpu,
            wgpu::DeviceType::Other
        ] {
            assert!(
                is_hardware_vulkan(device_type),
                "{device_type:?} is hardware"
            );
        }
    }
}
