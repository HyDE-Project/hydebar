//! Style of the quick settings buttons.

use iced::{
    Border, Color, Theme,
    widget::button::{self, Status}
};

/// Builds the quick settings button style closure with active feedback.
pub fn quick_settings_button_style(
    is_active: bool,
    opacity: f32
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let mut base = button::Style {
            background: Some(
                if is_active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.color
                }
                .scale_alpha(opacity)
                .into()
            ),
            border: Border {
                width:  0.0,
                radius: 32.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: if is_active {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            },
            ..button::Style::default()
        };
        match status {
            Status::Hovered => {
                let peach = theme.extended_palette().primary.weak.color;
                base.background = Some(
                    if is_active {
                        peach
                    } else {
                        theme.extended_palette().background.strong.color
                    }
                    .scale_alpha(opacity)
                    .into()
                );
                base
            }
            _ => base
        }
    }
}

/// Builds the submenu button style closure used inside quick settings menus.
pub fn quick_settings_submenu_button_style(
    is_active: bool,
    opacity: f32
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width:  0.0,
                radius: 16.0.into(),
                color:  Color::TRANSPARENT
            },
            text_color: if is_active {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            },
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
                base.text_color = theme.palette().text;
                base
            }
            _ => base
        }
    }
}
