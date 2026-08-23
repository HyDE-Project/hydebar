use iced::{
    Alignment, Element, Length,
    widget::{container, row, slider}
};

use super::Message;
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale
    },
    services::{
        ServiceEvent,
        brightness::{BrightnessData, BrightnessService}
    }
};

/// What the backlight section of the quick settings answers to.
#[derive(Debug, Clone)]
pub enum BrightnessMessage {
    /// The backlight said something.
    Event(Box<ServiceEvent<BrightnessService>>),
    /// Set the backlight to this level.
    Change(u32)
}

impl BrightnessData {
    /// The slider that sets the backlight.
    #[must_use]
    pub fn brightness_slider(&self, icons: &IconTheme) -> Element<'_, Message> {
        row!(
            container(icon(icons, Icons::Brightness))
                .padding([scale::scaled(8.0), scale::scaled(11.0)]),
            slider(
                0..=100,
                self.current.saturating_mul(100) / self.max.max(1),
                |v| Message::Brightness(BrightnessMessage::Change(v * self.max.max(1) / 100))
            )
            .step(1_u32)
            .width(Length::Fill),
        )
        .align_y(Alignment::Center)
        .spacing(scale::scaled(8.0))
        .into()
    }
}
