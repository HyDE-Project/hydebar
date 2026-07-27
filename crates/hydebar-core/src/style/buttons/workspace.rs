//! Style of the workspace indicator buttons.

use iced::{
    Background, Border, Theme,
    theme::palette,
    widget::button::{self, Status}
};

use crate::config::AppearanceColor;

/// Builds the workspace button style closure, handling optional colours.
pub fn workspace_button_style(
    is_empty: bool,
    colors: Option<Option<AppearanceColor>>
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let (bg_color, fg_color) = colors
            .map(|c| {
                c.map_or(
                    (
                        theme.extended_palette().primary.base.color,
                        theme.extended_palette().primary.base.text
                    ),
                    |c| {
                        let color = palette::Primary::generate(
                            c.get_base(),
                            theme.palette().background,
                            c.get_text().unwrap_or(theme.palette().text)
                        );
                        (color.base.color, color.base.text)
                    }
                )
            })
            .unwrap_or((
                theme.extended_palette().background.weak.color,
                theme.palette().text
            ));
        let mut base = button::Style {
            background: Some(Background::Color(if is_empty {
                theme.extended_palette().background.weak.color
            } else {
                bg_color
            })),
            border: Border {
                width:  if is_empty { 1.0 } else { 0.0 },
                color:  bg_color,
                radius: 16.0.into()
            },
            text_color: if is_empty {
                theme.extended_palette().background.weak.text
            } else {
                fg_color
            },
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                let (bg_color, fg_color) = colors
                    .map(|c| {
                        c.map_or(
                            (
                                theme.extended_palette().primary.strong.color,
                                theme.extended_palette().primary.strong.text
                            ),
                            |c| {
                                let color = palette::Primary::generate(
                                    c.get_base(),
                                    theme.palette().background,
                                    c.get_text().unwrap_or(theme.palette().text)
                                );
                                (color.strong.color, color.strong.text)
                            }
                        )
                    })
                    .unwrap_or((
                        theme.extended_palette().background.strong.color,
                        theme.palette().text
                    ));

                base.background = Some(Background::Color(if is_empty {
                    theme.extended_palette().background.strong.color
                } else {
                    bg_color
                }));
                base.text_color = if is_empty {
                    theme.extended_palette().background.weak.text
                } else {
                    fg_color
                };
                base
            }
            _ => base
        }
    }
}
