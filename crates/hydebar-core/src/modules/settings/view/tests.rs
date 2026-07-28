//! Unit tests for the settings menu layout helpers.

use iced::{
    Element,
    widget::{button, text}
};

use super::{
    helpers::{quick_settings_section, sub_menu_wrapper},
    quick_setting_button
};
use crate::{
    components::icons::{IconTheme, Icons},
    modules::settings::state::{Message, SubMenu}
};

#[test]
fn quick_settings_section_pairs_buttons() {
    let button_a: Element<'_, Message> = button(text("a"))
        .on_press(Message::ToggleInhibitIdle)
        .into();
    let button_b: Element<'_, Message> = button(text("b"))
        .on_press(Message::ToggleInhibitIdle)
        .into();

    let section = quick_settings_section(vec![(button_a, None), (button_b, None)], 1.0);
    let children = section.as_widget().children();
    assert_eq!(children.len(), 1);
}

#[test]
fn quick_settings_section_renders_menu_when_present() {
    let button_a: Element<'_, Message> = button(text("a"))
        .on_press(Message::ToggleInhibitIdle)
        .into();
    let menu: Element<'_, Message> = text("menu").into();

    let section = quick_settings_section(vec![(button_a, Some(menu))], 1.0);
    let children = section.as_widget().children();
    assert_eq!(children.len(), 2);
}

#[test]
fn quick_setting_button_can_render_submenu_toggle() {
    let icons = IconTheme::default();
    let element: Element<'_, Message> = quick_setting_button(
        &icons,
        Icons::Power,
        "Test".into(),
        None,
        true,
        Message::ToggleInhibitIdle,
        Some((
            SubMenu::Wifi,
            Some(SubMenu::Wifi),
            Message::ToggleInhibitIdle
        )),
        1.0
    );

    // A button renders a single row child that contains the submenu toggle.
    let children = element.as_widget().children();
    assert_eq!(children.len(), 1);
}
