//! Regression tests of the menu fade and its geometry.

#![allow(clippy::float_cmp)]

use std::time::Duration;

use iced::{Point, SurfaceId as Id, Task};

use super::{kind::MenuType, state::Menu};
use crate::{config::Config, position_button::ButtonUIRef};

fn button_ref() -> ButtonUIRef {
    ButtonUIRef {
        position: Point::new(10.0, 10.0),
        viewport: (1920.0, 1080.0)
    }
}

fn drain_animation(menu: &mut Menu, config: &Config) {
    let mut frames = 0;
    while menu
        .tick_animation::<()>(&config.appearance.animations, Duration::from_millis(8))
        .0
    {
        frames += 1;
        assert!(frames < 1000, "menu fade failed to settle");
    }
}

#[test]
fn opening_a_menu_fades_in_over_frames() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);

    let _task: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    assert!(menu.is_animating());
    assert_eq!(menu.get_opacity(), 0.0);

    drain_animation(&mut menu, &config);

    assert!(!menu.is_animating());
    assert_eq!(menu.get_opacity(), config.appearance.menu.opacity);
}

#[test]
fn closing_a_menu_snaps_dark_and_leaves_the_fade_to_the_compositor() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    drain_animation(&mut menu, &config);

    let _close: Task<()> = menu.close(&config);

    assert_eq!(
        menu.get_opacity(),
        0.0,
        "the app-side fade ends at once; the destroyed surface fades as one layer"
    );
    assert!(!menu.is_animating());
}

#[test]
fn disabled_animations_snap_to_the_target() {
    let mut config = Config::default();
    config.appearance.animations.enabled = false;
    let mut menu = Menu::new(Id::unique(), None);

    let _task: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    assert!(!menu.is_animating());
    assert_eq!(menu.get_opacity(), config.appearance.menu.opacity);
}

fn open_type(menu: &Menu) -> Option<&MenuType> {
    menu.menu_info.as_ref().map(|(menu_type, _)| menu_type)
}

#[test]
fn a_press_on_the_bar_takes_the_open_menu_down() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert!(!menu.is_open());
}

#[test]
fn a_menu_dismisses_only_once_the_press_that_armed_it_completed() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();

    // the module the press landed on still owns the rest of the click
    assert_eq!(open_type(&menu), Some(&MenuType::Calendar));
}

#[test]
fn a_press_the_menu_of_another_module_claimed_switches_instead_of_dismissing() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();
    let _toggle: Task<()> = menu.toggle(MenuType::Settings, button_ref(), &config);
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert_eq!(open_type(&menu), Some(&MenuType::Settings));
}

#[test]
fn a_press_opening_a_menu_does_not_take_it_back_down() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);

    menu.arm_dismissal();
    let _toggle: Task<()> = menu.toggle(MenuType::Calendar, button_ref(), &config);
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert_eq!(open_type(&menu), Some(&MenuType::Calendar));
}

#[test]
fn a_press_on_the_module_of_the_open_menu_leaves_it_closed() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();
    let _toggle: Task<()> = menu.toggle(MenuType::Calendar, button_ref(), &config);
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert!(!menu.is_open());
}

#[test]
fn a_completed_press_no_longer_arms_the_next_one() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);
    let _reopen: Task<()> = menu.open(MenuType::Settings, button_ref(), &config);
    let _again: Task<()> = menu.dismiss_if_armed(&config);

    assert_eq!(open_type(&menu), Some(&MenuType::Settings));
}

#[test]
fn arming_a_closed_menu_arms_nothing() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);

    menu.arm_dismissal();
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert_eq!(open_type(&menu), Some(&MenuType::Calendar));
}

#[test]
fn a_closed_menu_is_emptied_at_once_and_gets_a_successor_surface() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let departed = menu.id;
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    drain_animation(&mut menu, &config);

    let _close: Task<()> = menu.close(&config);

    assert!(!menu.is_open());
    assert!(
        open_type(&menu).is_none(),
        "the content leaves with the destroyed surface, not after it"
    );
    assert_ne!(
        menu.id, departed,
        "a fresh surface stands ready for the next open"
    );
}

#[test]
fn a_second_close_changes_nothing_and_spends_no_surface() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    drain_animation(&mut menu, &config);

    let _close: Task<()> = menu.close(&config);
    let successor = menu.id;
    let _again: Task<()> = menu.close(&config);

    assert_eq!(
        menu.id, successor,
        "closing a closed menu must not churn surfaces"
    );
}

#[test]
fn toggling_a_closing_menu_reopens_it() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    drain_animation(&mut menu, &config);

    let _close: Task<()> = menu.close(&config);
    let _toggle: Task<()> = menu.toggle(MenuType::Calendar, button_ref(), &config);

    assert!(
        menu.is_open(),
        "a click during the fade out brings the menu back"
    );
}

#[test]
fn disabled_animations_close_at_once() {
    let mut config = Config::default();
    config.appearance.animations.enabled = false;
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    let _close: Task<()> = menu.close(&config);

    assert!(open_type(&menu).is_none());
    assert_eq!(menu.get_opacity(), 0.0);
}

#[test]
fn closing_mid_open_starts_the_successor_from_dark() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique(), None);
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    let _: (bool, Task<()>) =
        menu.tick_animation(&config.appearance.animations, Duration::from_millis(40));

    let _close: Task<()> = menu.close(&config);

    assert_eq!(
        menu.get_opacity(),
        0.0,
        "the successor surface must not inherit a half-travelled fade"
    );
    assert!(!menu.is_animating());
}
