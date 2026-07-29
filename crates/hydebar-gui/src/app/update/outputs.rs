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
                    let name = info
                        .as_ref()
                        .and_then(|info| info.name.as_deref())
                        .unwrap_or("")
                        .to_owned();

                    if let Some(info) = info.as_ref() {
                        let mode = info
                            .modes
                            .iter()
                            .find(|mode| mode.current)
                            .or_else(|| info.modes.first())
                            .map(|mode| mode.dimensions);

                        self.adopt_metrics(mode, info.scale_factor, info.physical_size);
                    }

                    let appearance = self.scaled_appearance();

                    self.outputs.add(
                        appearance.style,
                        &self.config.outputs,
                        self.config.position,
                        &name,
                        wl_output,
                        &self.config,
                        appearance.scale_factor,
                        appearance.height
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

                    if self.adopt_metrics(mode, info.scale_factor, info.physical_size) {
                        self.refresh_appearance()
                    } else {
                        Task::none()
                    }
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
    /// Reports whether the sizes changed.
    fn adopt_metrics(
        &mut self,
        dimensions: Option<(i32, i32)>,
        scale_factor: i32,
        physical: (i32, i32)
    ) -> bool {
        let Some(dimensions) = dimensions else {
            return false;
        };

        let compositor_scale = scale_factor.max(1) as f32;
        let surface_scale = self.config.appearance.scale_factor as f32;

        let metrics = auto_metrics(
            dimensions.0 as f32,
            dimensions.1 as f32,
            compositor_scale * surface_scale.max(f32::EPSILON),
            (physical.0 as f32, physical.1 as f32)
        );

        if self.auto_metrics == Some(metrics) {
            return false;
        }

        info!("screen calls for a bar magnified {} times", metrics.scale);
        self.auto_metrics = Some(metrics);

        true
    }
}
