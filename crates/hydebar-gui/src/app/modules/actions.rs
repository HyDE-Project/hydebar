//! Press actions a bar module binds to its mouse buttons.

use hydebar_core::{
    config::{CustomModuleDef, ModuleName},
    modules::OnModulePress
};
use iced::window::Id;

use super::ModuleActions;
use crate::app::state::{App, Message};

impl App {
    /// Collects the per mouse button actions of a module.
    ///
    /// Only custom modules declare right and middle press commands today, every
    /// other module keeps reacting to the left press alone.
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
            _ => ModuleActions {
                left,
                ..ModuleActions::default()
            }
        }
    }
}

/// Binds the actions a module declared to its bar button.
pub(super) fn attach_module_actions<'a>(
    button: hydebar_core::position_button::PositionButton<'a, Message>,
    actions: ModuleActions,
    id: Id
) -> hydebar_core::position_button::PositionButton<'a, Message> {
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
        Some(message) => button.on_right_press(message),
        None => button
    };

    match actions.middle {
        Some(message) => button.on_middle_press(message),
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
        right:  launch_command(definition.command_right.as_deref()),
        middle: launch_command(definition.command_middle.as_deref())
    }
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
            launched(actions.right).as_deref(),
            Some("hyde-shell wallpaper -p")
        );
        assert_eq!(
            launched(actions.middle).as_deref(),
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
}
