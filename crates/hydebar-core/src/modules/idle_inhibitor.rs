//! Bar toggle keeping the session awake through the Wayland idle inhibitor.
//!
//! The module renders the state of the inhibitor owned by the settings module,
//! so the bar entry and the settings quick toggle always agree on a single
//! Wayland inhibitor instead of fighting over two of them.

use iced::Element;

use super::{Module, OnModulePress, settings::Message as SettingsMessage};
use crate::components::icons::{IconTheme, Icons, icon};

/// Bar entry rendering the idle inhibitor state as a toggle.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleInhibitor;

impl IdleInhibitor {
    /// Glyph representing `inhibited`.
    #[must_use]
    pub const fn state_icon(inhibited: bool) -> Icons {
        if inhibited {
            Icons::IdleInhibitorActive
        } else {
            Icons::IdleInhibitorInactive
        }
    }
}

impl<M> Module<M> for IdleInhibitor
where
    M: 'static + Clone + From<SettingsMessage>
{
    /// Whether the session is currently kept awake, plus the glyph table.
    type ViewData<'a> = (bool, &'a IconTheme);
    type RegistrationData<'a> = ();

    fn view(
        &self,
        (inhibited, icons): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        Some((
            icon(icons, Self::state_icon(inhibited)).into(),
            Some(OnModulePress::Action(Box::new(M::from(
                SettingsMessage::ToggleInhibitIdle
            ))))
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::{ModuleContext, event_bus::EventBus, modules::ModuleError};

    fn view(
        inhibited: bool,
        icons: &IconTheme
    ) -> (
        Element<'static, SettingsMessage>,
        Option<OnModulePress<SettingsMessage>>
    ) {
        <IdleInhibitor as Module<SettingsMessage>>::view(&IdleInhibitor, (inhibited, icons))
            .expect("the idle inhibitor module always renders")
    }

    #[test]
    fn renders_the_activated_glyph_while_inhibited() {
        assert_eq!(IdleInhibitor::state_icon(true).default_glyph(), "\u{f0576}");
        assert_eq!(IdleInhibitor::state_icon(true), Icons::IdleInhibitorActive);

        let (_, action) = view(true, &IconTheme::default());
        assert!(action.is_some());
    }

    #[test]
    fn renders_the_deactivated_glyph_while_idle_is_allowed() {
        assert_eq!(
            IdleInhibitor::state_icon(false).default_glyph(),
            "\u{f06ca}"
        );
        assert_eq!(
            IdleInhibitor::state_icon(false),
            Icons::IdleInhibitorInactive
        );

        let (_, action) = view(false, &IconTheme::default());
        assert!(action.is_some());
    }

    #[test]
    fn honours_configured_icon_overrides() {
        let mut icons = IconTheme::default();
        icons.set(Icons::IdleInhibitorActive, "A");
        icons.set(Icons::IdleInhibitorInactive, "D");

        assert_eq!(icons.glyph(IdleInhibitor::state_icon(true)), "A");
        assert_eq!(icons.glyph(IdleInhibitor::state_icon(false)), "D");
    }

    #[test]
    fn a_press_toggles_the_shared_inhibitor() {
        let (_, action) = view(false, &IconTheme::default());

        match action {
            Some(OnModulePress::Action(message)) => {
                assert!(matches!(*message, SettingsMessage::ToggleInhibitIdle));
            }
            _ => panic!("expected the idle inhibitor toggle action")
        }
    }

    #[test]
    fn register_succeeds() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut module = IdleInhibitor;

        let result: Result<(), ModuleError> =
            <IdleInhibitor as Module<SettingsMessage>>::register(&mut module, &ctx, ());
        assert!(result.is_ok());
    }
}
