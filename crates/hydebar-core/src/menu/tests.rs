//! Regression tests of the menu fade and its geometry.

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
    while menu.tick_animation(&config.appearance.animations, Duration::from_millis(8)) {
        frames += 1;
        assert!(frames < 1000, "menu fade failed to settle");
    }
}

#[test]
fn opening_a_menu_fades_in_over_frames() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique());

    let _task: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    assert!(menu.is_animating());
    assert_eq!(menu.get_opacity(), 0.0);

    drain_animation(&mut menu, &config);

    assert!(!menu.is_animating());
    assert_eq!(menu.get_opacity(), config.appearance.menu.opacity);
}

#[test]
fn closing_a_menu_fades_back_to_transparent() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique());
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    drain_animation(&mut menu, &config);

    let _close: Task<()> = menu.close(&config);
    assert!(menu.is_animating());

    drain_animation(&mut menu, &config);

    assert_eq!(menu.get_opacity(), 0.0);
}

#[test]
fn disabled_animations_snap_to_the_target() {
    let mut config = Config::default();
    config.appearance.animations.enabled = false;
    let mut menu = Menu::new(Id::unique());

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
    let mut menu = Menu::new(Id::unique());
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert!(open_type(&menu).is_none());
}

#[test]
fn a_menu_dismisses_only_once_the_press_that_armed_it_completed() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique());
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();

    // the module the press landed on still owns the rest of the click
    assert_eq!(open_type(&menu), Some(&MenuType::Calendar));
}

#[test]
fn a_press_the_menu_of_another_module_claimed_switches_instead_of_dismissing() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique());
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();
    let _toggle: Task<()> = menu.toggle(MenuType::Settings, button_ref(), &config);
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert_eq!(open_type(&menu), Some(&MenuType::Settings));
}

#[test]
fn a_press_opening_a_menu_does_not_take_it_back_down() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique());

    menu.arm_dismissal();
    let _toggle: Task<()> = menu.toggle(MenuType::Calendar, button_ref(), &config);
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert_eq!(open_type(&menu), Some(&MenuType::Calendar));
}

#[test]
fn a_press_on_the_module_of_the_open_menu_leaves_it_closed() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique());
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);

    menu.arm_dismissal();
    let _toggle: Task<()> = menu.toggle(MenuType::Calendar, button_ref(), &config);
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert!(open_type(&menu).is_none());
}

#[test]
fn a_completed_press_no_longer_arms_the_next_one() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique());
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
    let mut menu = Menu::new(Id::unique());

    menu.arm_dismissal();
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    let _dismiss: Task<()> = menu.dismiss_if_armed(&config);

    assert_eq!(open_type(&menu), Some(&MenuType::Calendar));
}

#[test]
fn reopening_mid_fade_keeps_the_current_opacity() {
    let config = Config::default();
    let mut menu = Menu::new(Id::unique());
    let _open: Task<()> = menu.open(MenuType::Calendar, button_ref(), &config);
    let _ = menu.tick_animation(&config.appearance.animations, Duration::from_millis(40));

    let mid_fade = menu.get_opacity();
    let _close: Task<()> = menu.close(&config);

    assert_eq!(menu.get_opacity(), mid_fade);
    assert!(menu.is_animating());
}
