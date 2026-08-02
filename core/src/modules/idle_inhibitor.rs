//! Bar toggle keeping the session awake through the Wayland idle inhibitor.
//!
//! The entry renders the state of the inhibitor owned by the settings module,
//! so the bar entry and the settings quick toggle always agree on a single
//! Wayland inhibitor instead of fighting over two of them. The entry holds no
//! state and no background work of its own — the data lives in the control
//! centre and this module is a render function over it, the shape described
//! in `ARCHITECTURE.md` as the target for every module.

use iced::Element;

use super::{OnModulePress, control_center::Message as ControlCenterMessage};
use crate::components::icons::{IconTheme, Icons, icon};

/// Glyph representing `inhibited`.
#[must_use]
pub const fn state_icon(inhibited: bool) -> Icons {
    if inhibited {
        Icons::IdleInhibitorActive
    } else {
        Icons::IdleInhibitorInactive
    }
}

/// The bar entry: the inhibitor glyph, pressed to toggle it.
#[must_use]
pub fn bar_view<M>(
    inhibited: bool,
    icons: &IconTheme
) -> (Element<'static, M>, Option<OnModulePress<M>>)
where
    M: 'static + Clone + From<ControlCenterMessage>
{
    (
        icon(icons, state_icon(inhibited)).into(),
        Some(OnModulePress::Action(Box::new(M::from(
            ControlCenterMessage::ToggleInhibitIdle
        ))))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(
        inhibited: bool,
        icons: &IconTheme
    ) -> (
        Element<'static, ControlCenterMessage>,
        Option<OnModulePress<ControlCenterMessage>>
    ) {
        bar_view(inhibited, icons)
    }

    #[test]
    fn renders_the_activated_glyph_while_inhibited() {
        assert_eq!(state_icon(true).default_glyph(), "\u{f0176}");
        assert_eq!(state_icon(true), Icons::IdleInhibitorActive);

        let (_, action) = view(true, &IconTheme::default());
        assert!(action.is_some());
    }

    #[test]
    fn renders_the_deactivated_glyph_while_idle_is_allowed() {
        assert_eq!(state_icon(false).default_glyph(), "\u{f06ca}");
        assert_eq!(state_icon(false), Icons::IdleInhibitorInactive);

        let (_, action) = view(false, &IconTheme::default());
        assert!(action.is_some());
    }

    #[test]
    fn honours_configured_icon_overrides() {
        let mut icons = IconTheme::default();
        icons.set(Icons::IdleInhibitorActive, "A");
        icons.set(Icons::IdleInhibitorInactive, "D");

        assert_eq!(icons.glyph(state_icon(true)), "A");
        assert_eq!(icons.glyph(state_icon(false)), "D");
    }

    #[test]
    fn a_press_toggles_the_shared_inhibitor() {
        let (_, action) = view(false, &IconTheme::default());

        match action {
            Some(OnModulePress::Action(message)) => {
                assert!(matches!(*message, ControlCenterMessage::ToggleInhibitIdle));
            }
            _ => panic!("expected the idle inhibitor toggle action")
        }
    }
}
