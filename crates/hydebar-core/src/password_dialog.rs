use iced::{
    Alignment, Element, Length, SurfaceId as Id,
    alignment::Vertical,
    widget::{Space, button, column, row, text_input}
};

use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
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
        .spacing(scale::scaled(16.0))
        .align_y(Alignment::Center),
        text(format!("Insert password to connect to: {wifi_ssid}")),
        text_input("", current_password)
            .secure(true)
            .size(scale::scaled(16.0))
            .padding([scale::scaled(8.0), scale::scaled(16.0)])
            .style(text_input_style)
            .on_input(Message::PasswordChanged)
            .on_submit(Message::DialogConfirmed(id)),
        row!(
            Space::new().width(Length::Fill),
            button(text("Cancel").align_y(Vertical::Center))
                .padding([scale::scaled(4.0), scale::scaled(32.0)])
                .style(outline_button_style(opacity))
                .height(Length::Fixed(scale::scaled(50.0)))
                .on_press(Message::DialogCancelled(id)),
            button(text("Confirm").align_y(Vertical::Center))
                .padding([scale::scaled(4.0), scale::scaled(32.0)])
                .height(Length::Fixed(scale::scaled(50.0)))
                .style(confirm_button_style(opacity))
                .on_press(Message::DialogConfirmed(id))
        )
        .spacing(scale::scaled(8.0))
        .width(Length::Fill)
    )
    .spacing(scale::scaled(16.0))
    .padding(scale::scaled(16.0))
    .max_width(350.)
    .into()
}
