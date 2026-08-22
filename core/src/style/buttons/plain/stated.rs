//! Buttons carrying a border or a fill of their own.

use iced::{
    Border, Color, Theme,
    widget::button::{self, Status}
};

/// Builds an outline button style closure that highlights borders on hover.
pub fn outline_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width:  2.0,
                radius: 32.0.into(),
                color:  theme.extended_palette().background.weak.color
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into()
                );
                base
            }
            _ => base
        }
    }
}

/// Builds the confirm button style closure with filled background.
pub fn confirm_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity)
                    .into()
            ),
            border: Border {
                width:  2.0,
                radius: 32.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .strong
                        .color
                        .scale_alpha(opacity)
                        .into()
                );
                base
            }
            _ => base
        }
    }
}

/// Builds the rounded settings button style closure.
pub fn settings_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity)
                    .into()
            ),
            border: Border {
                width:  0.0,
                radius: 32.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .strong
                        .color
                        .scale_alpha(opacity)
                        .into()
                );
                base
            }
            _ => base
        }
    }
}
