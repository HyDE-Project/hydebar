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

#[derive(Debug, Clone)]
pub enum BrightnessMessage {
    Event(Box<ServiceEvent<BrightnessService>>),
    Change(u32)
}

impl BrightnessData {
    pub fn brightness_slider(&self, icons: &IconTheme) -> Element<'_, Message> {
        row!(
            container(icon(icons, Icons::Brightness))
                .padding([scale::scaled(8.0), scale::scaled(11.0)]),
            slider(0..=100, self.current * 100 / self.max, |v| {
                Message::Brightness(BrightnessMessage::Change(v * self.max / 100))
            })
            .step(1_u32)
            .width(Length::Fill),
        )
        .align_y(Alignment::Center)
        .spacing(scale::scaled(8.0))
        .into()
    }
}
