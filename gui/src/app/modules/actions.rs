//! Press actions a bar module binds to its mouse buttons.

use hydebar_core::{
    config::{CustomMenuEntry, CustomModuleDef, ModuleName},
    menu::MenuType,
    modules::OnModulePress
};
use iced::SurfaceId as Id;

use super::ModuleActions;
use crate::app::state::{App, Message};

impl App {
    /// Collects the per mouse button actions of a module.
    ///
    /// Custom modules declare right and middle press commands, and a module
    /// cycling display formats hands its left button over to the cycling and
    /// keeps its menu on the right button.
    pub(super) fn module_actions(
        &self,
        module_name: &ModuleName,
        left: Option<OnModulePress<Message>>
    ) -> ModuleActions {
        match module_name {
            ModuleName::Custom(name) => {
                match self.config.custom_modules.iter().find(|m| &m.name == name) {
                    Some(definition) => ModuleActions {
                        left,
                        ..custom_module_actions(definition)
                    },
                    None => ModuleActions {
                        left,
                        ..ModuleActions::default()
                    }
                }
            }
            ModuleName::Themes => ModuleActions {
                left:   Some(OnModulePress::Action(Box::new(Message::Themes(
                    hydebar_core::modules::themes::Message::NextTheme
                )))),
                right:  Some(OnModulePress::Action(Box::new(Message::Themes(
                    hydebar_core::modules::themes::Message::PreviousTheme
                )))),
                middle: Some(OnModulePress::ToggleMenu(MenuType::Themes))
            },
            ModuleName::Wallpaper => ModuleActions {
                left:   Some(OnModulePress::Action(Box::new(Message::Wallpaper(
                    hydebar_core::modules::wallpaper::Message::Next
                )))),
                right:  Some(OnModulePress::Action(Box::new(Message::Wallpaper(
                    hydebar_core::modules::wallpaper::Message::Previous
                )))),
                middle: Some(OnModulePress::ToggleMenu(MenuType::Wallpaper))
            },
            ModuleName::BarLayout => ModuleActions {
                left:   Some(OnModulePress::Action(Box::new(Message::BarLayout(
                    hydebar_core::modules::bar_layout::Message::Next
                )))),
                right:  Some(OnModulePress::Action(Box::new(Message::BarLayout(
                    hydebar_core::modules::bar_layout::Message::Previous
                )))),
                middle: Some(OnModulePress::ToggleMenu(MenuType::BarLayout))
            },
            _ => ModuleActions {
                left,
                right: self.displaced_menu(module_name),
                ..ModuleActions::default()
            }
        }
    }

    /// Menu of a module whose left button cycles through display formats.
    ///
    /// A module without alternative formats keeps its menu on the left button,
    /// so nothing moves for a configuration that declares none.
    fn displaced_menu(&self, module_name: &ModuleName) -> Option<OnModulePress<Message>> {
        let menu = match module_name {
            ModuleName::Clock if self.config.clock.has_alternatives() => MenuType::Calendar,
            ModuleName::SystemInfo if self.config.system.has_alternatives() => {
                MenuType::SystemInfo
            }
            _ => return None
        };

        Some(OnModulePress::ToggleMenu(menu))
    }
}

/// Binds the actions a module declared to its bar button.
pub(super) fn attach_module_actions(
    button: hydebar_core::position_button::PositionButton<'_, Message>,
    actions: ModuleActions,
    id: Id
) -> hydebar_core::position_button::PositionButton<'_, Message> {
    let button = match actions.left {
        Some(OnModulePress::Action(action)) => button.on_press(*action),
        Some(OnModulePress::ToggleMenu(menu_type)) => {
            button.on_press_with_position(move |button_ui_ref| {
                Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
            })
        }
        None => button
    };

    let button = match actions.right {
        Some(OnModulePress::Action(action)) => button.on_right_press(*action),
        Some(OnModulePress::ToggleMenu(menu_type)) => {
            button.on_right_press_with_position(move |button_ui_ref| {
                Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
            })
        }
        None => button
    };

    match actions.middle {
        Some(OnModulePress::Action(action)) => button.on_middle_press(*action),
        Some(OnModulePress::ToggleMenu(menu_type)) => {
            button.on_middle_press_with_position(move |button_ui_ref| {
                Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
            })
        }
        None => button
    }
}

/// Builds the launch message of a command declared by a custom module.
///
/// Commands that are absent or blank leave their mouse button unbound.
fn launch_command(command: Option<&str>) -> Option<Message> {
    let command = command?.trim();

    if command.is_empty() {
        return None;
    }

    Some(Message::LaunchCommand(command.to_owned()))
}

/// Builds the per mouse button actions declared by a custom module.
fn custom_module_actions(definition: &CustomModuleDef) -> ModuleActions {
    ModuleActions {
        left:   custom_module_action(definition),
        right:  custom_module_right_action(definition),
        middle: launch_command(definition.command_middle.as_deref())
            .map(|message| OnModulePress::Action(Box::new(message)))
    }
}

/// Builds the right press action of a custom module.
///
/// A definition declaring an actionable context menu entry opens the menu and
/// ignores `command_right`: one press cannot both open a menu and run a
/// command, and the menu wins.
fn custom_module_right_action(definition: &CustomModuleDef) -> Option<OnModulePress<Message>> {
    if definition.has_menu() {
        return Some(OnModulePress::ToggleMenu(MenuType::Custom(
            definition.name.clone()
        )));
    }

    launch_command(definition.command_right.as_deref())
        .map(|message| OnModulePress::Action(Box::new(message)))
}

/// Builds the message selecting an entry of a custom module context menu
/// publishes.
///
/// It carries the surface the menu belongs to so the entry both runs its
/// command and dismisses the menu.
pub fn custom_menu_message(id: Id, entry: &CustomMenuEntry) -> Message {
    Message::CustomMenuAction(id, entry.command.trim().to_owned())
}

/// Builds the left press action running the command declared by a custom
/// module.
///
/// Modules that leave the command empty stay inert so they render as plain
/// indicators instead of unresponsive buttons.
pub(super) fn custom_module_action(
    definition: &CustomModuleDef
) -> Option<OnModulePress<Message>> {
    launch_command(Some(definition.command.as_str()))
        .map(|message| OnModulePress::Action(Box::new(message)))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn definition(command: &str) -> CustomModuleDef {
        CustomModuleDef {
            name: String::from("example"),
            command: String::from(command),
            ..CustomModuleDef::default()
        }
    }

    fn launched(message: Option<Message>) -> Option<String> {
        match message {
            Some(Message::LaunchCommand(command)) => Some(command),
            Some(other) => panic!("unexpected message: {other:?}"),
            None => None
        }
    }

    fn launched_action(action: Option<OnModulePress<Message>>) -> Option<String> {
        match action {
            Some(OnModulePress::Action(message)) => launched(Some(*message)),
            Some(other) => panic!("unexpected action: {other:?}"),
            None => None
        }
    }

    fn menu_with(entries: &[(&str, &str)]) -> Vec<CustomMenuEntry> {
        entries
            .iter()
            .map(|(label, command)| CustomMenuEntry {
                label:   String::from(*label),
                command: String::from(*command),
                icon:    None
            })
            .collect()
    }

    #[test]
    fn builds_a_launch_action_for_a_configured_command() {
        let action = custom_module_action(&definition("  notify-send hi  "));

        // the command is trimmed so shell invocations stay predictable
        match action {
            Some(OnModulePress::Action(message)) => match *message {
                Message::LaunchCommand(command) => assert_eq!(command, "notify-send hi"),
                other => panic!("unexpected message: {other:?}")
            },
            _ => panic!("expected a launch action")
        }
    }

    #[test]
    fn leaves_a_module_without_a_command_inert() {
        assert!(custom_module_action(&definition("   ")).is_none());
    }

    #[test]
    fn binds_one_action_per_declared_mouse_button() {
        let actions = custom_module_actions(&CustomModuleDef {
            command_right: Some(String::from("hyde-shell wallpaper -p")),
            command_middle: Some(String::from("hyde-shell wallpaper --select")),
            ..definition("hyde-shell wallpaper -n")
        });

        match actions.left {
            Some(OnModulePress::Action(message)) => {
                assert_eq!(
                    launched(Some(*message)).as_deref(),
                    Some("hyde-shell wallpaper -n")
                );
            }
            _ => panic!("expected a launch action")
        }
        assert_eq!(
            launched_action(actions.right).as_deref(),
            Some("hyde-shell wallpaper -p")
        );
        assert_eq!(
            launched_action(actions.middle).as_deref(),
            Some("hyde-shell wallpaper --select")
        );
    }

    #[test]
    fn leaves_the_side_buttons_unbound_without_their_commands() {
        let actions = custom_module_actions(&definition("hyde-shell wallpaper -n"));

        assert!(actions.left.is_some());
        assert!(actions.right.is_none());
        assert!(actions.middle.is_none());
        assert!(!actions.is_inert());
    }

    #[test]
    fn ignores_blank_side_commands() {
        let actions = custom_module_actions(&CustomModuleDef {
            command_right: Some(String::from("   ")),
            command_middle: Some(String::new()),
            ..definition("   ")
        });

        assert!(actions.is_inert());
    }

    #[test]
    fn a_right_press_opens_the_context_menu_of_the_module() {
        let actions = custom_module_actions(&CustomModuleDef {
            menu: menu_with(&[("Lock", "hyde-shell lockscreen.sh")]),
            ..definition("hyde-shell logoutlaunch 1")
        });

        match actions.right {
            Some(OnModulePress::ToggleMenu(MenuType::Custom(name))) => {
                assert_eq!(name, "example");
            }
            other => panic!("expected a context menu toggle, got {other:?}")
        }
    }

    #[test]
    fn the_context_menu_wins_over_the_right_press_command() {
        let actions = custom_module_actions(&CustomModuleDef {
            command_right: Some(String::from("systemctl poweroff")),
            menu: menu_with(&[("Lock", "hyde-shell lockscreen.sh")]),
            ..definition("hyde-shell logoutlaunch 1")
        });

        assert!(matches!(
            actions.right,
            Some(OnModulePress::ToggleMenu(MenuType::Custom(_)))
        ));
    }

    #[test]
    fn a_menu_without_an_actionable_entry_keeps_the_right_press_command() {
        let actions = custom_module_actions(&CustomModuleDef {
            command_right: Some(String::from("systemctl poweroff")),
            menu: menu_with(&[("  ", "systemctl reboot"), ("Reboot", "   ")]),
            ..definition("hyde-shell logoutlaunch 1")
        });

        assert_eq!(
            launched_action(actions.right).as_deref(),
            Some("systemctl poweroff")
        );
    }

    #[test]
    fn a_module_is_pressable_through_its_menu_alone() {
        let actions = custom_module_actions(&CustomModuleDef {
            menu: menu_with(&[("Lock", "hyde-shell lockscreen.sh")]),
            ..definition("   ")
        });

        assert!(actions.left.is_none());
        assert!(!actions.is_inert());
    }

    #[test]
    fn selecting_an_entry_launches_its_command_on_the_menu_surface() {
        let id = Id::unique();
        let entry = CustomMenuEntry {
            label:   String::from("Lock"),
            command: String::from("  hyde-shell lockscreen.sh  "),
            icon:    None
        };

        match custom_menu_message(id, &entry) {
            Message::CustomMenuAction(surface, command) => {
                assert_eq!(surface, id);
                assert_eq!(command, "hyde-shell lockscreen.sh");
            }
            other => panic!("unexpected message: {other:?}")
        }
    }
}
