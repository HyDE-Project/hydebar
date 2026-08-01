use std::process::Command;

use iced::{
    Alignment, Element,
    widget::{Column, Row, button, container}
};
use log::{debug, error};

use super::{Module, OnModulePress};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    menu::MenuType
};

/// Screenshot action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotAction {
    Area,
    Window,
    Fullscreen
}

/// Message emitted by the screenshot module.
#[derive(Debug, Clone, Copy)]
pub enum ScreenshotMessage {
    TakeScreenshot(ScreenshotAction),
    StartRecording,
    StopRecording
}

/// Screenshot and recording module.
#[derive(Debug, Default)]
pub struct Screenshot {
    pub is_recording: bool,
    /// Identifier of the recorder this bar started, while one runs.
    recorder_pid:     Option<u32>
}

impl Screenshot {
    /// Take a screenshot with the specified action.
    ///
    /// The whole capture runs on its own thread: the area capture waits for
    /// the user to finish dragging the selection, and a bar that waited with
    /// it stopped repainting and answering for that whole time. Waiting for
    /// the capture tools on that thread also reaps them, so no capture leaves
    /// a zombie process behind.
    pub fn take_screenshot(&self, action: ScreenshotAction) {
        let screenshot_dir = dirs::picture_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("Screenshots");

        if let Err(err) = std::fs::create_dir_all(&screenshot_dir) {
            error!("Failed to create screenshots directory: {err}");
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = screenshot_dir.join(format!("screenshot_{timestamp}.png"));

        std::thread::spawn(move || {
            let result = match action {
                ScreenshotAction::Area => {
                    debug!("Taking area screenshot");

                    match Command::new("slurp").output() {
                        Ok(output) if output.status.success() => {
                            let geometry =
                                String::from_utf8_lossy(&output.stdout).trim().to_string();

                            Command::new("grim")
                                .arg("-g")
                                .arg(geometry)
                                .arg(&filename)
                                .status()
                        }
                        Ok(_) => {
                            debug!("Slurp cancelled by user");
                            return;
                        }
                        Err(err) => {
                            error!("Failed to run slurp: {err}");
                            return;
                        }
                    }
                }
                ScreenshotAction::Window => {
                    debug!("Taking window screenshot (fullscreen for now)");
                    Command::new("grim").arg(&filename).status()
                }
                ScreenshotAction::Fullscreen => {
                    debug!("Taking fullscreen screenshot");
                    Command::new("grim").arg(&filename).status()
                }
            };

            match result {
                Ok(status) if status.success() => {
                    debug!("Screenshot saved to: {}", filename.display());
                }
                Ok(status) => error!("screenshot tool reported {status}"),
                Err(err) => error!("Failed to take screenshot: {err}")
            }
        });
    }

    /// Start screen recording.
    pub fn start_recording(&mut self) {
        if self.is_recording {
            error!("Recording already in progress");
            return;
        }

        let video_dir = dirs::video_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("Recordings");

        // Create directory if it doesn't exist
        if let Err(err) = std::fs::create_dir_all(&video_dir) {
            error!("Failed to create recordings directory: {err}");
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = video_dir.join(format!("recording_{timestamp}.mp4"));

        debug!("Starting recording to: {}", filename.display());

        match Command::new("wf-recorder").arg("-f").arg(&filename).spawn() {
            Ok(mut child) => {
                self.recorder_pid = Some(child.id());
                self.is_recording = true;
                debug!("Recording started");

                std::thread::spawn(move || match child.wait() {
                    Ok(status) if status.success() => {}
                    Ok(status) => error!("the recorder ended with {status}"),
                    Err(err) => error!("waiting on the recorder failed: {err}")
                });
            }
            Err(err) => error!("Failed to start recording: {err}")
        }
    }

    /// Stop screen recording.
    pub fn stop_recording(&mut self) {
        if !self.is_recording {
            error!("No recording in progress");
            return;
        }

        debug!("Stopping recording");

        let Some(pid) = self.recorder_pid.take() else {
            self.is_recording = false;
            return;
        };

        match Command::new("kill")
            .arg("-INT")
            .arg(pid.to_string())
            .spawn()
        {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                self.is_recording = false;
                debug!("Recording stopped");
            }
            Err(err) => error!("Failed to stop recording: {err}")
        }
    }

    /// Update the module state based on messages.
    pub fn update(&mut self, message: ScreenshotMessage) {
        match message {
            ScreenshotMessage::TakeScreenshot(action) => {
                self.take_screenshot(action);
            }
            ScreenshotMessage::StartRecording => {
                self.start_recording();
            }
            ScreenshotMessage::StopRecording => {
                self.stop_recording();
            }
        }
    }

    /// Render screenshot actions menu.
    #[must_use]
    pub fn menu_view(&self, _opacity: f32, icons: &IconTheme) -> Element<'_, ScreenshotMessage> {
        let mut content = Column::new()
            .spacing(scale::scaled(8.0))
            .padding(scale::scaled(12.0));

        // Screenshot section
        content = content.push(text("Screenshot").size(scale::scaled(16.0)));

        let screenshot_buttons = Column::new()
            .push(
                button(
                    Row::new()
                        .push(icon(icons, Icons::AreaSelect))
                        .push(text("Select Area"))
                        .spacing(scale::icon_gap())
                        .align_y(Alignment::Center)
                )
                .on_press(ScreenshotMessage::TakeScreenshot(ScreenshotAction::Area))
                .width(iced::Length::Fill)
            )
            .push(
                button(
                    Row::new()
                        .push(icon(icons, Icons::WindowCapture))
                        .push(text("Current Window"))
                        .spacing(scale::icon_gap())
                        .align_y(Alignment::Center)
                )
                .on_press(ScreenshotMessage::TakeScreenshot(ScreenshotAction::Window))
                .width(iced::Length::Fill)
            )
            .push(
                button(
                    Row::new()
                        .push(icon(icons, Icons::Fullscreen))
                        .push(text("Fullscreen"))
                        .spacing(scale::icon_gap())
                        .align_y(Alignment::Center)
                )
                .on_press(ScreenshotMessage::TakeScreenshot(
                    ScreenshotAction::Fullscreen
                ))
                .width(iced::Length::Fill)
            )
            .spacing(scale::scaled(4.0));

        content = content.push(screenshot_buttons);

        // Recording section
        content = content.push(text("Recording").size(scale::scaled(16.0)));

        let recording_button = if self.is_recording {
            button(
                Row::new()
                    .push(icon(icons, Icons::Stop))
                    .push(text("Stop Recording"))
                    .spacing(scale::icon_gap())
                    .align_y(Alignment::Center)
            )
            .on_press(ScreenshotMessage::StopRecording)
            .width(iced::Length::Fill)
        } else {
            button(
                Row::new()
                    .push(icon(icons, Icons::Record))
                    .push(text("Start Recording"))
                    .spacing(scale::icon_gap())
                    .align_y(Alignment::Center)
            )
            .on_press(ScreenshotMessage::StartRecording)
            .width(iced::Length::Fill)
        };

        content = content.push(recording_button);

        container(content).into()
    }
}

impl<M> Module<M> for Screenshot
where
    M: 'static + Clone + From<ScreenshotMessage>
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    /// Render camera icon with recording indicator.
    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let content = if self.is_recording {
            Row::new()
                .push(icon(icons, Icons::Point))
                .push(icon(icons, Icons::Camera))
                .spacing(scale::icon_gap())
                .align_y(Alignment::Center)
        } else {
            Row::new().push(icon(icons, Icons::Camera))
        };

        Some((
            container(content).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Screenshot))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_creates_not_recording() {
        let screenshot = Screenshot::default();
        assert!(!screenshot.is_recording);
    }
}
