//! Context menu a custom module opens on a right press.
//!
//! It is the native counterpart of the Waybar `menu-actions` map: every
//! entry a definition declares becomes a row running its command,
//! without the separate GTK menu file Waybar needs to name the rows.

use iced::{
    Element, Length,
    widget::{Column, button, row}
};

use crate::{
    components::{icons::icon_raw, scale, text::text},
    config::{Appearance, CustomMenuEntry, CustomModuleDef},
    style::menu_entry_button_style
};

/// Builds the rows of the context menu declared by a custom module.
///
/// `on_select` turns the pressed entry into the message the caller reacts
/// to, so the module stays unaware of how the command is run and of how
/// the menu surface is dismissed.
///
/// Entries carry their glyph verbatim rather than through the icon theme:
/// an entry names the glyph the way the module itself does, so there is
/// no named icon to resolve.
///
/// Entries missing a label or a command are dropped by
/// [`CustomModuleDef::menu_entries`], leaving an empty column for a
/// definition without a usable entry.
pub fn menu_view<'a, M>(
    definition: &CustomModuleDef,
    appearance: &Appearance,
    opacity: f32,
    on_select: impl Fn(&CustomMenuEntry) -> M + 'a
) -> Element<'a, M>
where
    M: Clone + 'a
{
    let radius = appearance.pill_radius();
    let gap = appearance.icon_label_gap();

    Column::with_children(
        definition
            .menu_entries()
            .map(|entry| {
                let label: Element<'a, M> = match entry.icon.as_deref() {
                    Some(glyph) if !glyph.is_empty() => {
                        row![icon_raw(glyph.to_owned()), text(entry.label.clone())]
                            .spacing(gap)
                            .align_y(iced::Alignment::Center)
                            .into()
                    }
                    _ => text(entry.label.clone()).into()
                };

                button(label)
                    .padding([scale::scaled(4.0), scale::scaled(12.0)])
                    .width(Length::Fill)
                    .style(menu_entry_button_style(opacity, radius))
                    .on_press(on_select(entry))
                    .into()
            })
            .collect::<Vec<Element<'a, M>>>()
    )
    .width(Length::Fill)
    .spacing(scale::scaled(4.0))
    .into()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn definition() -> CustomModuleDef {
        toml::from_str(
            r#"
        name = "power"
        command = ""

        [[menu]]
        label = "Lock"
        icon = "L"
        command = "hyde-shell lockscreen.sh"

        [[menu]]
        label = "Logout"
        command = "hyprctl dispatch exit 0"
        "#
        )
        .expect("definition")
    }

    #[test]
    fn selecting_an_entry_builds_the_message_of_its_command() {
        let definition = definition();
        let commands = definition
            .menu_entries()
            .map(|entry| entry.command.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            commands,
            vec![
                String::from("hyde-shell lockscreen.sh"),
                String::from("hyprctl dispatch exit 0")
            ]
        );
    }

    #[test]
    fn renders_a_row_per_declared_entry() {
        let definition = definition();
        let appearance = Appearance::default();

        let _element: Element<'_, String> =
            menu_view(&definition, &appearance, appearance.menu.opacity, |entry| {
                entry.command.clone()
            });
    }
}
