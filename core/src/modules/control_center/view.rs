//! Drawing of the quick settings: the bar entry and the panel it opens.

use iced::{Element, SurfaceId as Id};

use super::state::{ControlCenter, Message};
use crate::{
    components::icons::IconTheme,
    config::{ControlCenterModuleConfig, Position},
    modules::OnModulePress
};

mod bar;
mod helpers;
mod menu;
mod quick_button;
mod standalone;

pub use quick_button::quick_setting_button;

impl ControlCenter {
    /// The bar entry: the quick settings gathered behind one glyph.
    ///
    /// Rendered by the module itself, so the bar layer holds no quick
    /// settings drawing of its own; the standalone audio, network, bluetooth
    /// and power entries render from the same state through their own
    /// methods.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>
    {
        Some(self.render_bar(icons))
    }

    /// The panel the entry opens.
    #[must_use]
    pub fn menu_view(
        &self,
        id: Id,
        config: &ControlCenterModuleConfig,
        opacity: f32,
        position: Position,
        icons: &IconTheme
    ) -> Element<'_, Message> {
        self.render_menu(id, config, opacity, position, icons)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    //! Unit tests for the settings menu layout helpers.

    use iced::{
        Element,
        widget::{button, text}
    };

    use super::{helpers::quick_settings_section, quick_setting_button};
    use crate::{
        components::icons::{IconTheme, Icons},
        modules::control_center::state::{Message, SubMenu}
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
}
