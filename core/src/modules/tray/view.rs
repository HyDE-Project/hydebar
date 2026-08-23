//! Rendering of tray icons and their menus.

use iced::{
    Element, Length,
    widget::{Column, Row, button, row, rule, toggler}
};

use super::super::tray::{TrayMessage, TrayModule};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale,
        text::text
    },
    services::tray::dbus::{Layout, LayoutProps},
    style::ghost_button_style
};

impl TrayModule {
    /// The menu one tray icon opens.
    #[must_use]
    pub fn menu_view(
        &self,
        name: &'_ str,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, TrayMessage> {
        self.service
            .as_ref()
            .and_then(|service| service.data.iter().find(|item| item.name == name))
            .map_or_else(
                || Row::new().into(),
                |item| {
                    Column::with_children(
                        item.menu
                            .2
                            .iter()
                            .map(|menu| self.menu_voice(name, menu, opacity, icons))
                    )
                    .spacing(scale::scaled(8.0))
                    .into()
                }
            )
    }

    pub(super) fn menu_voice(
        &self,
        name: &str,
        layout: &Layout,
        opacity: f32,
        icons: &IconTheme
    ) -> Element<'_, TrayMessage> {
        match &layout.1 {
            LayoutProps {
                label: Some(label),
                toggle_type: Some(toggle_type),
                toggle_state: Some(state),
                ..
            } if toggle_type == "checkmark" => toggler(*state > 0)
                .label(label.replace('_', ""))
                .on_toggle({
                    let name = name.to_owned();
                    let id = layout.0;

                    move |_| TrayMessage::MenuSelected(name.clone(), id)
                })
                .width(Length::Fill)
                .into(),
            LayoutProps {
                children_display: Some(display),
                label: Some(label),
                ..
            } if display == "submenu" => {
                let is_open = self.submenus.contains(&layout.0);
                Column::new()
                    .push(
                        button(row!(
                            text(label.replace('_', "")).width(Length::Fill),
                            icon(
                                icons,
                                if is_open {
                                    Icons::MenuOpen
                                } else {
                                    Icons::MenuClosed
                                }
                            )
                        ))
                        .style(ghost_button_style(opacity))
                        .padding([scale::scaled(8.0), scale::scaled(8.0)])
                        .on_press(TrayMessage::ToggleSubmenu(layout.0))
                        .width(Length::Fill)
                    )
                    .push_maybe(if is_open {
                        Some(
                            Column::with_children(
                                layout
                                    .2
                                    .iter()
                                    .map(|menu| self.menu_voice(name, menu, opacity, icons))
                            )
                            .padding(iced::Padding {
                                top:    0.0,
                                right:  0.0,
                                bottom: 0.0,
                                left:   16.0
                            })
                            .spacing(scale::scaled(4.0))
                        )
                    } else {
                        None
                    })
                    .into()
            }
            LayoutProps {
                label: Some(label), ..
            } => button(text(label.replace('_', "")))
                .style(ghost_button_style(opacity))
                .on_press(TrayMessage::MenuSelected(name.to_owned(), layout.0))
                .width(Length::Fill)
                .padding([scale::scaled(8.0), scale::scaled(8.0)])
                .into(),
            LayoutProps {
                type_: Some(t), ..
            } if t == "separator" => rule::horizontal(1).into(),
            _ => Row::new().into()
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced_test::simulator;

    use super::{
        super::module::{default_command_factory, default_listener_spawner},
        *
    };

    fn module(submenus: Vec<i32>) -> TrayModule {
        TrayModule {
            service: None,
            submenus,
            sender: None,
            runtime: None,
            listener_handles: Vec::new(),
            listener_spawner: default_listener_spawner(),
            command_factory: default_command_factory()
        }
    }

    fn icons() -> IconTheme {
        IconTheme::default()
    }

    const fn props() -> LayoutProps {
        LayoutProps {
            children_display: None,
            label:            None,
            type_:            None,
            toggle_type:      None,
            toggle_state:     None
        }
    }

    fn labelled(id: i32, label: &str) -> Layout {
        Layout(
            id,
            LayoutProps {
                label: Some(label.to_owned()),
                ..props()
            },
            Vec::new()
        )
    }

    fn checkmark(id: i32, label: &str, state: i32) -> Layout {
        Layout(
            id,
            LayoutProps {
                label: Some(label.to_owned()),
                toggle_type: Some("checkmark".to_owned()),
                toggle_state: Some(state),
                ..props()
            },
            Vec::new()
        )
    }

    fn submenu(id: i32, label: &str, children: Vec<Layout>) -> Layout {
        Layout(
            id,
            LayoutProps {
                children_display: Some("submenu".to_owned()),
                label: Some(label.to_owned()),
                ..props()
            },
            children
        )
    }

    fn separator(id: i32) -> Layout {
        Layout(
            id,
            LayoutProps {
                type_: Some("separator".to_owned()),
                ..props()
            },
            Vec::new()
        )
    }

    #[test]
    fn a_module_without_a_service_draws_an_empty_menu() {
        let module = module(Vec::new());

        let mut ui = simulator(module.menu_view("anything", 1.0, &icons()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn a_plain_entry_is_named_without_its_mnemonic_marks() {
        let module = module(Vec::new());

        let mut ui = simulator(module.menu_voice("app", &labelled(1, "_Quit"), 1.0, &icons()));

        assert!(ui.find("Quit").is_ok());
    }

    #[test]
    fn pressing_a_plain_entry_names_the_item_and_the_entry() {
        let module = module(Vec::new());

        let mut ui = simulator(module.menu_voice("app", &labelled(7, "Quit"), 1.0, &icons()));
        let _ = ui.click("Quit").expect("the entry is pressable");

        assert!(ui.into_messages().any(|message| matches!(
            message,
            TrayMessage::MenuSelected(item, id) if item == "app" && id == 7
        )));
    }

    #[test]
    fn a_checkmark_entry_is_drawn_as_a_toggle() {
        let module = module(Vec::new());

        let mut ui =
            simulator(module.menu_voice("app", &checkmark(2, "_Muted", 1), 1.0, &icons()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn a_checkmark_entry_is_drawn_in_both_positions() {
        let module = module(Vec::new());

        for state in [0, 1] {
            let mut ui =
                simulator(module.menu_voice("app", &checkmark(2, "Muted", state), 1.0, &icons()));

            assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
        }
    }

    #[test]
    fn switching_a_checkmark_entry_names_the_item_and_the_entry() {
        let module = module(Vec::new());

        let mut ui = simulator(module.menu_voice("app", &checkmark(3, "Muted", 0), 1.0, &icons()));
        ui.point_at(iced::Point::new(1000.0, 10.0));
        let _ = ui.simulate(iced_test::simulator::click());

        assert!(ui.into_messages().any(|message| matches!(
            message,
            TrayMessage::MenuSelected(item, id) if item == "app" && id == 3
        )));
    }

    #[test]
    fn a_closed_submenu_hides_what_it_holds() {
        let module = module(Vec::new());
        let layout = submenu(4, "More", vec![labelled(5, "Inside")]);

        let mut ui = simulator(module.menu_voice("app", &layout, 1.0, &icons()));

        assert!(ui.find("More").is_ok());
        assert!(ui.find("Inside").is_err());
    }

    #[test]
    fn an_open_submenu_shows_what_it_holds() {
        let module = module(vec![4]);
        let layout = submenu(4, "More", vec![labelled(5, "Inside")]);

        let mut ui = simulator(module.menu_voice("app", &layout, 1.0, &icons()));

        assert!(ui.find("More").is_ok());
        assert!(ui.find("Inside").is_ok());
    }

    #[test]
    fn pressing_a_submenu_asks_to_fold_it_the_other_way() {
        let module = module(Vec::new());
        let layout = submenu(4, "More", Vec::new());

        let mut ui = simulator(module.menu_voice("app", &layout, 1.0, &icons()));
        let _ = ui.click("More").expect("the submenu is pressable");

        assert!(
            ui.into_messages()
                .any(|message| matches!(message, TrayMessage::ToggleSubmenu(4)))
        );
    }

    #[test]
    fn an_open_submenu_draws_the_entries_nested_under_it() {
        let module = module(vec![4]);
        let layout = submenu(4, "More", vec![labelled(5, "Inside")]);

        let mut ui = simulator(module.menu_voice("app", &layout, 1.0, &icons()));

        let head = ui
            .find("More")
            .expect("the submenu is drawn")
            .visible_bounds()
            .expect("the submenu is visible");
        let nested = ui
            .find("Inside")
            .expect("the entry is drawn")
            .visible_bounds()
            .expect("the entry is visible");

        assert!(nested.y > head.y);
        assert!(nested.x > head.x);
    }

    #[test]
    fn a_separator_is_drawn_as_a_rule() {
        let module = module(Vec::new());

        let mut ui = simulator(module.menu_voice("app", &separator(6), 1.0, &icons()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn an_entry_that_says_nothing_about_itself_is_drawn_as_nothing() {
        let module = module(Vec::new());
        let bare = Layout(8, props(), Vec::new());

        let mut ui = simulator(module.menu_voice("app", &bare, 1.0, &icons()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn a_toggle_of_an_unknown_kind_falls_back_to_a_plain_entry() {
        let module = module(Vec::new());
        let layout = Layout(
            9,
            LayoutProps {
                label: Some("Radio".to_owned()),
                toggle_type: Some("radio".to_owned()),
                toggle_state: Some(1),
                ..props()
            },
            Vec::new()
        );

        let mut ui = simulator(module.menu_voice("app", &layout, 1.0, &icons()));
        let _ = ui.click("Radio").expect("the entry is pressable");

        assert!(ui.into_messages().any(|message| matches!(
            message,
            TrayMessage::MenuSelected(item, id) if item == "app" && id == 9
        )));
    }
}
