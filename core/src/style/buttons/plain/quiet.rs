//! Buttons that are only there once the pointer is on them.

use iced::{
    Border, Color, Theme,
    widget::button::{self, Status}
};

/// Builds a ghost button style closure that fades in on hover.
pub fn ghost_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width:  0.0,
                radius: 4.0.into(),
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

/// Builds the style of a context menu entry.
///
/// It is the ghost style of the other menu rows, rounded with the pill radius
/// the bar is themed with so an entry echoes the module it was opened from.
pub fn menu_entry_button_style(
    opacity: f32,
    radius: f32
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let base = ghost_button_style(opacity)(theme, status);

        button::Style {
            border: Border {
                radius: radius.into(),
                ..base.border
            },
            ..base
        }
    }
}
