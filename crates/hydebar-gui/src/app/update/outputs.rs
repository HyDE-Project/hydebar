//! Compositor output hotplug handling.

use hydebar_core::outputs::auto_metrics;
use iced::{Task, event::wayland::OutputEvent};
use log::info;

use super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_outputs(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OutputEvent((event, wl_output)) => match event {
                OutputEvent::Created(info) => {
                    info!("Output created: {info:?}");
                    if let Some(info) = info.as_ref() {
                        let mode = info
                            .modes
                            .iter()
                            .find(|mode| mode.current)
                            .or_else(|| info.modes.first())
                            .map(|mode| mode.dimensions);

                        self.adopt_output_metrics(mode, info.scale_factor);
                    }
                    let name = info
                        .as_ref()
                        .and_then(|info| info.name.as_deref())
                        .unwrap_or("");

                    self.outputs.add(
                        self.config.appearance.style,
                        &self.config.outputs,
                        self.config.position,
                        name,
                        wl_output,
                        &self.config
                    )
                }
                OutputEvent::Removed => {
                    info!("Output destroyed");
                    self.outputs.remove(
                        self.config.appearance.style,
                        self.config.position,
                        wl_output,
                        &self.config
                    )
                }
                OutputEvent::InfoUpdate(info) => {
                    let mode = info
                        .modes
                        .iter()
                        .find(|mode| mode.current)
                        .or_else(|| info.modes.first())
                        .map(|mode| mode.dimensions);

                    self.adopt_output_metrics(mode, info.scale_factor);
                    Task::none()
                }
            },
            _ => Task::none()
        }
    }

    /// Records the sizes a screen of `dimensions` at `scale_factor` calls for.
    ///
    /// The compositor scale is handed on so the sizes are not scaled a second
    /// time by a compositor that already scales the surface, and the scale the
    /// bar applies to its own surface is divided out for the same reason.
    fn adopt_output_metrics(&mut self, dimensions: Option<(i32, i32)>, scale_factor: i32) {
        let Some(dimensions) = dimensions else {
            return;
        };

        let compositor_scale = scale_factor.max(1) as f32;
        let surface_scale = self.config.appearance.scale_factor as f32;

        let metrics = auto_metrics(
            dimensions.0 as f32,
            dimensions.1 as f32,
            compositor_scale * surface_scale.max(f32::EPSILON)
        );

        if self.auto_metrics != Some(metrics) {
            info!(
                "screen calls for a text size of {} and a bar height of {}",
                metrics.font_size, metrics.height
            );
            self.auto_metrics = Some(metrics);
            self.refresh_appearance();
        }
    }
}
