//! Compositor output hotplug handling.

use hydebar_core::outputs::{
    auto_metrics,
    scaling::{ScreenGeometry, screen_geometry}
};
use iced::{OutputEvent, Task};
use log::info;

use super::super::state::{App, Message};

/// Asks the compositor for the screen's geometry off the drawing thread.
///
/// The question costs a process spawn and the compositor may answer slowly;
/// the drawing thread must never stand waiting on it, so the answer comes
/// back as a message of its own.
fn measure_screen(name: String) -> Task<Message> {
    Task::perform(
        async move {
            let answer = tokio::task::spawn_blocking({
                let name = name.clone();
                move || screen_geometry(&name)
            })
            .await
            .ok()
            .flatten();

            (name, answer)
        },
        |(name, geometry)| Message::ScreenMeasured {
            name,
            geometry
        }
    )
}

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_outputs(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScreenMeasured {
                name,
                geometry
            } => {
                let Some(geometry) = geometry else {
                    return Task::none();
                };

                if self.outputs.has_name(&name) && self.adopt_measured(geometry) {
                    self.refresh_appearance()
                } else {
                    Task::none()
                }
            }
            Message::OutputEvent(event) => match event {
                OutputEvent::Added(info) => {
                    info!("Output created: {info:?}");

                    let measure = measure_screen(info.name.clone());

                    let appearance = self.scaled_appearance();

                    let add = self.outputs.add(
                        appearance.style,
                        &self.config.outputs,
                        self.config.position,
                        &info.name,
                        info.id,
                        &self.config,
                        appearance.scale_factor,
                        appearance.height
                    );

                    Task::batch([measure, add])
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
                OutputEvent::InfoChanged(info) => measure_screen(info.name),
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

    /// Records the sizes a screen of `geometry` calls for.
    ///
    /// The compositor scale is handed on so the sizes are not scaled a second
    /// time by a compositor that already scales the surface, and the scale the
    /// bar applies to its own surface is divided out for the same reason.
    /// Reports whether the sizes changed.
    fn adopt_measured(&mut self, geometry: ScreenGeometry) -> bool {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "surface scale factors are small values that fit f32"
        )]
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
