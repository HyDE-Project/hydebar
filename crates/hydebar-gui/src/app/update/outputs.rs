//! Compositor output hotplug handling.

use hydebar_core::outputs::{auto_metrics, scaling::screen_geometry};
use iced::{OutputEvent, Task};
use log::info;

use super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_outputs(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OutputEvent(event) => match event {
                OutputEvent::Added(info) => {
                    info!("Output created: {info:?}");

                    self.adopt_metrics(&info.name);

                    let appearance = self.scaled_appearance();

                    self.outputs.add(
                        appearance.style,
                        &self.config.outputs,
                        self.config.position,
                        &info.name,
                        info.id,
                        &self.config,
                        appearance.scale_factor,
                        appearance.height
                    )
                }
                OutputEvent::Removed(id) => {
                    info!("Output destroyed");
                    self.outputs.remove(
                        self.config.appearance.style,
                        self.config.position,
                        id,
                        &self.config
                    )
                }
                OutputEvent::InfoChanged(info) => {
                    if self.adopt_metrics(&info.name) {
                        self.refresh_appearance()
                    } else {
                        Task::none()
                    }
                }
                OutputEvent::SurfaceEnteredOutput {
                    ..
                }
                | OutputEvent::SurfaceLeftOutput {
                    ..
                } => Task::none()
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
    fn adopt_metrics(&mut self, name: &str) -> bool {
        let Some(geometry) = screen_geometry(name) else {
            return false;
        };

        let surface_scale = self.config.appearance.scale_factor as f32;

        self.screen_height = Some(geometry.pixels.1 / geometry.scale.max(f32::EPSILON));

        let metrics = auto_metrics(
            geometry.pixels.0,
            geometry.pixels.1,
            geometry.scale * surface_scale.max(f32::EPSILON),
            geometry.physical
        );

        if self.auto_metrics == Some(metrics) {
            return false;
        }

        info!("screen calls for a bar magnified {} times", metrics.scale);
        self.auto_metrics = Some(metrics);

        true
    }
}
