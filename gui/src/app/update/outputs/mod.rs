//! Compositor output hotplug handling.

pub mod reads;

use hydebar_core::outputs::{auto_metrics, scaling::ScreenGeometry};
use iced::{OutputEvent, Task};
use log::info;
pub use reads::{measure_screen, read_looks, read_wallpaper};

use super::super::state::{App, Message};

impl App {
    /// Handles the messages this module owns.
    pub(super) fn update_outputs(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScreenMeasured {
                name,
                geometry
            } => {
                let Some(geometry) = geometry else {
                    log::debug!("frame: screen {name} measured as gone");

                    if self.outputs.has_name(&name) {
                        return Task::perform(
                            async move {
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                name
                            },
                            |name| Message::RemeasureScreen {
                                name
                            }
                        );
                    }

                    return Task::none();
                };

                if self.outputs.has_name(&name) {
                    log::debug!("frame: screen {name} measured");

                    let looked = if self.adopt_measured(name, geometry) {
                        self.refresh_appearance()
                    } else {
                        Task::none()
                    };

                    let rows = Task::perform(
                        async {
                            for _ in 0..10 {
                                let rows = hydebar_core::outputs::perch::strip_rows().await;

                                if !rows.is_empty() {
                                    return rows;
                                }

                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }

                            std::collections::HashMap::new()
                        },
                        Message::StripRowsMeasured,
                    );

                    Task::batch([looked, rows])
                } else {
                    Task::none()
                }
            }
            Message::WallpaperRead(read) => {
                self.wallpaper_preview = read;

                Task::none()
            }
            Message::LooksRead(looks) => {
                self.looks = *looks;

                Task::none()
            }
            Message::StripRowsMeasured(rows) => {
                if !rows.is_empty() && self.strip_rows != rows {
                    self.strip_rows = rows;
                }

                self.unfold_desk()
            }
            Message::RemeasureScreen {
                name
            } => {
                if self.outputs.has_name(&name) {
                    measure_screen(name)
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
                        appearance.height_px()
                    );

                    let unfold = self.unfold_desk();

                    Task::batch([measure, add, unfold])
                }
                OutputEvent::Removed(id) => {
                    info!("Output destroyed");

                    let name = self.outputs.output_name(id).map(str::to_owned);

                    let removed = self.outputs.remove(
                        self.config.appearance.style,
                        self.config.position,
                        id,
                        &self.config
                    );

                    if let Some(name) = name {
                        self.screen_sizes.remove(&name);
                    }

                    Task::batch([removed, self.unfold_desk()])
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
    /// The sizes are kept per monitor as well, because each output rings
    /// itself in its own numbers. Reports whether the shared metrics changed.
    fn adopt_measured(&mut self, name: String, geometry: ScreenGeometry) -> bool {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "surface scale factors are small values that fit f32"
        )]
        let surface_scale = self.config.appearance.scale_factor as f32;

        let height = geometry.pixels.1 / geometry.scale.max(f32::EPSILON);
        let width = geometry.pixels.0 / geometry.scale.max(f32::EPSILON);

        self.screen_height = Some(height);
        self.screen_width = Some(width);
        self.screen_sizes.insert(name, (width, height));

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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use hydebar_core::outputs::scaling::ScreenGeometry;

    use super::{
        super::super::state::{Message, test_support::test_app_with},
        *
    };

    fn measured(pixels: (f32, f32), scale: f32) -> ScreenGeometry {
        ScreenGeometry {
            pixels,
            physical: (0.0, 0.0),
            scale
        }
    }

    fn tracked(app: &mut App, name: &str) -> iced::OutputId {
        let id = iced::OutputId::for_tests();
        let task = app.outputs.add::<Message>(
            app.config.appearance.style,
            &app.config.outputs,
            app.config.position,
            name,
            id,
            &app.config,
            1.0,
            None
        );
        let _ = task;

        id
    }

    #[test]
    fn each_output_adopts_its_own_measured_size() {
        let mut app = test_app_with(|config| {
            config.outputs = hydebar_core::config::Outputs::Targets(vec![
                "DP-1".to_owned(),
                "HDMI-1".to_owned(),
            ]);
            config.appearance.frame.enabled = true;
        });

        tracked(&mut app, "DP-1");
        tracked(&mut app, "HDMI-1");

        let task = app.update_outputs(Message::ScreenMeasured {
            name:     "DP-1".to_owned(),
            geometry: Some(measured((1920.0, 1080.0), 1.0))
        });
        let _ = task;
        let task = app.update_outputs(Message::ScreenMeasured {
            name:     "HDMI-1".to_owned(),
            geometry: Some(measured((3840.0, 2160.0), 2.0))
        });
        let _ = task;

        // the last report does not overwrite the first: every output rings
        // itself in its own numbers
        assert_eq!(app.screen_sizes.get("DP-1"), Some(&(1920.0, 1080.0)));
        assert_eq!(app.screen_sizes.get("HDMI-1"), Some(&(1920.0, 1080.0)));
        assert_eq!(app.screen_sizes.len(), 2);
    }

    #[test]
    fn a_departed_output_leaves_no_size_behind() {
        let mut app = test_app_with(|config| {
            config.outputs = hydebar_core::config::Outputs::Targets(vec![
                "DP-1".to_owned(),
                "HDMI-1".to_owned(),
            ]);
        });

        let dp = tracked(&mut app, "DP-1");
        tracked(&mut app, "HDMI-1");

        let task = app.update_outputs(Message::ScreenMeasured {
            name:     "DP-1".to_owned(),
            geometry: Some(measured((1920.0, 1080.0), 1.0))
        });
        let _ = task;

        assert!(app.screen_sizes.contains_key("DP-1"));

        let _ = app.update_outputs(Message::OutputEvent(iced::OutputEvent::Removed(dp)));

        assert!(!app.screen_sizes.contains_key("DP-1"));
    }

    #[test]
    fn an_arriving_output_is_tracked() {
        let mut app = test_app_with(|config| {
            config.outputs = hydebar_core::config::Outputs::Targets(vec!["DP-1".to_owned()]);
        });

        let info = iced::OutputInfo {
            id:           iced::OutputId::for_tests(),
            name:         "DP-1".to_owned(),
            scale_factor: 1,
            logical_size: Some((1920, 1080)),
            make:         String::new(),
            model:        String::new()
        };
        let task = app.update_outputs(Message::OutputEvent(iced::OutputEvent::Added(info)));
        let _ = task;

        assert!(app.outputs.has_name("DP-1"));
    }

    #[test]
    fn a_gone_output_schedules_a_remeasure() {
        let mut app = test_app_with(|config| {
            config.outputs = hydebar_core::config::Outputs::Targets(vec!["DP-1".to_owned()]);
        });

        tracked(&mut app, "DP-1");

        // the compositor reports the screen as gone: no size is adopted,
        // and the bar asks again later instead of ringing a ghost
        let task = app.update_outputs(Message::ScreenMeasured {
            name:     "DP-1".to_owned(),
            geometry: None
        });
        let _ = task;

        assert!(!app.screen_sizes.contains_key("DP-1"));
    }

    #[test]
    fn an_untracked_gone_screen_schedules_nothing() {
        let mut app = test_app_with(|config| {
            config.outputs = hydebar_core::config::Outputs::Targets(vec!["DP-1".to_owned()]);
        });

        // no group answers to the name: the answer is dropped instead of
        // remeasuring a screen the bar never stood on
        let task = app.update_outputs(Message::ScreenMeasured {
            name:     "HDMI-1".to_owned(),
            geometry: None
        });
        let _ = task;

        assert!(app.screen_sizes.is_empty());
    }

    #[test]
    fn a_remeasure_only_fires_for_a_tracked_output() {
        let mut app = test_app_with(|config| {
            config.outputs = hydebar_core::config::Outputs::Targets(vec!["DP-1".to_owned()]);
        });

        tracked(&mut app, "DP-1");

        let task = app.update_outputs(Message::RemeasureScreen {
            name: "DP-1".to_owned()
        });
        let _ = task;

        let task = app.update_outputs(Message::RemeasureScreen {
            name: "HDMI-1".to_owned()
        });
        let _ = task;
    }

    #[test]
    fn surface_traffic_leaves_the_outputs_alone() {
        let mut app = test_app_with(|_| {});
        let id = iced::OutputId::for_tests();

        for event in [
            iced::OutputEvent::SurfaceEnteredOutput {
                surface: iced::SurfaceId::unique(),
                output:  id
            },
            iced::OutputEvent::SurfaceLeftOutput {
                surface: iced::SurfaceId::unique(),
                output:  id
            }
        ] {
            let task = app.update_outputs(Message::OutputEvent(event));
            let _ = task;
        }

        assert!(app.screen_sizes.is_empty());
    }
}
