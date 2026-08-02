//! Drawing of the screenshot actions menu.

use iced::{
    Alignment, Element,
    widget::{Column, Row, button, container}
};

use super::{Screenshot, ScreenshotAction, ScreenshotMessage};
use crate::components::{
    icons::{IconTheme, Icons, icon},
    scale,
    text::text
};

impl Screenshot {
    /// Render screenshot actions menu.
    ///
    /// The screenshot section comes first, the recording section under it.
    #[must_use]
    pub fn menu_view(&self, _opacity: f32, icons: &IconTheme) -> Element<'_, ScreenshotMessage> {
        let mut content = Column::new()
            .spacing(scale::scaled(8.0))
            .padding(scale::scaled(12.0));

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
