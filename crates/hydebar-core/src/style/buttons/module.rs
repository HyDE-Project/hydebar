//! Style of the buttons hosting bar modules.

use iced::{
    Border, Color, Theme,
    widget::button::{self, Status}
};

use crate::config::AppearanceStyle;

/// Builds the module button style closure based on the appearance
/// configuration.
pub fn module_button_style(
    style: AppearanceStyle,
    opacity: f32,
    transparent: bool,
    focused: bool
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: match style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => None,
                AppearanceStyle::Islands => {
                    if transparent {
                        None
                    } else {
                        Some(theme.palette().background.scale_alpha(opacity).into())
                    }
                }
            },
            border: if focused {
                Border {
                    width:  2.0,
                    radius: 12.0.into(),
                    color:  theme.palette().primary
                }
            } else {
                Border {
                    width:  0.0,
                    radius: 12.0.into(),
                    color:  Color::TRANSPARENT
                }
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
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
