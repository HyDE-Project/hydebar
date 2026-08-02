//! The screenshot module: captures, recordings and the menu that starts them.
//!
//! One folder, four rooms: [`capture`] takes the screenshots, [`recording`]
//! starts and stops the recorder, [`menu`] draws the actions menu and
//! [`module`] wires the module to the bar. The root holds the state the
//! rooms share.

mod capture;
mod menu;
mod module;
mod recording;

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
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn default_creates_not_recording() {
        let screenshot = Screenshot::default();
        assert!(!screenshot.is_recording);
    }
}
