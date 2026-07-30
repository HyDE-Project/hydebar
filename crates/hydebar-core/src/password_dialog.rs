use iced::{
    Alignment, Element, Length, SurfaceId as Id,
    alignment::Vertical,
    widget::{Space, button, column, row, text, text_input}
};

use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale
    },
    style::{confirm_button_style, outline_button_style, text_input_style}
};

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    DialogConfirmed(Id),
    DialogCancelled(Id)
}

pub fn view<'a>(
    id: Id,
    wifi_ssid: &str,
    current_password: &str,
    opacity: f32,
    icons: &IconTheme
) -> Element<'a, Message> {
    column!(
        row!(
            icon(icons, Icons::WifiLock4).size(scale::scaled(32.0)),
            text("Authentication required").size(scale::scaled(22.0)),
        )
        .spacing(16)
        .align_y(Alignment::Center),
        text(format!("Insert password to connect to: {wifi_ssid}")),
        text_input("", current_password)
            .secure(true)
            .size(scale::scaled(16.0))
            .padding([8, 16])
            .style(text_input_style)
            .on_input(Message::PasswordChanged)
            .on_submit(Message::DialogConfirmed(id)),
        row!(
            Space::new().width(Length::Fill),
            button(text("Cancel").align_y(Vertical::Center))
                .padding([4, 32])
                .style(outline_button_style(opacity))
                .height(Length::Fixed(50.))
                .on_press(Message::DialogCancelled(id)),
            button(text("Confirm").align_y(Vertical::Center))
                .padding([4, 32])
                .height(Length::Fixed(50.))
                .style(confirm_button_style(opacity))
                .on_press(Message::DialogConfirmed(id))
        )
        .spacing(8)
        .width(Length::Fill)
    )
    .spacing(16)
    .padding(16)
    .max_width(350.)
    .into()
}
