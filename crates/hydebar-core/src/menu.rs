use std::time::Duration;

use iced::{
    self, Element, Length, Padding, Task,
    alignment::{Horizontal, Vertical},
    platform_specific::shell::wayland::commands::layer_surface::{
        KeyboardInteractivity, Layer, set_keyboard_interactivity, set_layer
    },
    widget::{container, mouse_area},
    window::Id
};

use crate::{
    animation::Spring,
    config::{AnimationConfig, AppearanceStyle, Position},
    position_button::ButtonUIRef,
    style::{menu_backdrop_style, menu_container_style}
};

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum MenuType {
    Updates,
    ControlCenter,
    Tray(String),
    MediaPlayer,
    SystemInfo,
    Notifications,
    Screenshot,
    Calendar,
    /// Context menu of the custom module carrying the given name.
    Custom(String)
}

#[derive(Clone, Debug)]
pub struct Menu {
    pub id:        Id,
    pub menu_info: Option<(MenuType, ButtonUIRef)>,
    opacity:       Spring
}

impl Menu {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            menu_info: None,
            opacity: Spring::new(0.0)
        }
    }

    pub fn open<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config
    ) -> Task<Message> {
        self.menu_info.replace((menu_type, button_ui_ref));

        self.aim_opacity(
            config.appearance.menu.opacity,
            &config.appearance.animations
        );

        let mut tasks = vec![set_layer(self.id, Layer::Overlay)];

        if config.menu_keyboard_focus {
            tasks.push(set_keyboard_interactivity(
                self.id,
                KeyboardInteractivity::OnDemand
            ));
        }

        Task::batch(tasks)
    }

    pub fn close<Message: 'static>(&mut self, config: &crate::config::Config) -> Task<Message> {
        if self.menu_info.is_some() {
            self.menu_info.take();

            self.aim_opacity(0.0, &config.appearance.animations);

            let mut tasks = vec![set_layer(self.id, Layer::Background)];

            if config.menu_keyboard_focus {
                tasks.push(set_keyboard_interactivity(
                    self.id,
                    KeyboardInteractivity::None
                ));
            }

            Task::batch(tasks)
        } else {
            Task::none()
        }
    }

    pub fn toggle<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.menu_info.as_mut() {
            None => self.open(menu_type, button_ui_ref, config),
            Some((current_type, _)) if *current_type == menu_type => self.close(config),
            Some((current_type, current_button_ui_ref)) => {
                *current_type = menu_type;
                *current_button_ui_ref = button_ui_ref;
                Task::none()
            }
        }
    }

    pub fn close_if<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        config: &crate::config::Config
    ) -> Task<Message> {
        if let Some((current_type, _)) = self.menu_info.as_ref() {
            if *current_type == menu_type {
                self.close(config)
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    pub fn request_keyboard<Message: 'static>(&self, menu_keyboard_focus: bool) -> Task<Message> {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::OnDemand)
        } else {
            Task::none()
        }
    }

    pub fn release_keyboard<Message: 'static>(&self, menu_keyboard_focus: bool) -> Task<Message> {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::None)
        } else {
            Task::none()
        }
    }

    /// Points the opacity spring at `target`, or jumps to it when animations
    /// are disabled.
    fn aim_opacity(&mut self, target: f32, animation_config: &AnimationConfig) {
        if animation_config.enabled {
            self.opacity.set_response(Duration::from_millis(
                animation_config.menu_fade_duration_ms
            ));
            self.opacity.set_target(target);
        } else {
            self.opacity.snap_to(target);
        }
    }

    /// Advances the opacity spring by `elapsed` and reports whether the menu
    /// still needs frames.
    pub fn tick_animation(
        &mut self,
        animation_config: &AnimationConfig,
        elapsed: Duration
    ) -> bool {
        if !animation_config.enabled {
            return false;
        }

        self.opacity.advance(elapsed)
    }

    /// Returns whether the menu has an unfinished opacity animation.
    pub fn is_animating(&self) -> bool {
        self.opacity.is_animating()
    }

    /// Get the current animated opacity for rendering
    pub fn get_opacity(&self) -> f32 {
        self.opacity.value()
    }
}

pub enum MenuSize {
    Small,
    Medium,
    Large
}

impl MenuSize {
    fn size(&self) -> f32 {
        match self {
            MenuSize::Small => 250.,
            MenuSize::Medium => 350.,
            MenuSize::Large => 450.
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn menu_wrapper<Message: Clone + 'static>(
    _id: Id,
    content: Element<'_, Message>,
    menu_size: MenuSize,
    button_ui_ref: ButtonUIRef,
    bar_position: Position,
    style: AppearanceStyle,
    opacity: f32,
    menu_backdrop: f32,
    none_message: Message,
    close_menu_message: Message
) -> Element<'_, Message> {
    mouse_area(
        container(
            mouse_area(
                container(content)
                    .height(Length::Shrink)
                    .width(Length::Fill)
                    .max_width(menu_size.size())
                    .padding(16)
                    .style(menu_container_style(opacity))
            )
            .on_release(none_message)
        )
        .align_y(match bar_position {
            Position::Top => Vertical::Top,
            Position::Bottom => Vertical::Bottom
        })
        .align_x(Horizontal::Left)
        .padding({
            let size = menu_size.size();

            let v_padding = match style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => 2,
                AppearanceStyle::Islands => 0
            };

            Padding::new(0.)
                .top(if bar_position == Position::Top {
                    v_padding
                } else {
                    0
                })
                .bottom(if bar_position == Position::Bottom {
                    v_padding
                } else {
                    0
                })
                .left(f32::min(
                    f32::max(button_ui_ref.position.x - size / 2., 8.),
                    button_ui_ref.viewport.0 - size - 8.
                ))
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(menu_backdrop_style(menu_backdrop))
    )
    .on_release(close_menu_message)
    .into()
}

#[cfg(test)]
mod tests {
    use iced::{Point, window::Id};

    use super::*;
    use crate::config::Config;

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
}
