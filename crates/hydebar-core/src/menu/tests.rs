//! Regression tests of the menu fade and its geometry.

use std::time::Duration;

use iced::{Point, Task, window::Id};

use super::{kind::MenuType, size::MenuSize, state::Menu};
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
