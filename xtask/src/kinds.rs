//! The shapes a scaffolded module can take, and the template values each
//! shape fills in.
//!
//! Placeholders stay dumb text: every kind decision lives here, in tested
//! Rust, so a template never branches.

use std::fmt;

/// The background work the new module owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
    /// A static entry: state plus drawing, no background work.
    Simple,
    /// A readout sampled on the bar's two clocks.
    Polling,
    /// An entry fed by an event stream it owns.
    Listener,
    /// A listener entry opening a menu on press.
    Menu
}

/// Why a kind flag cannot become a module shape.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidKind {
    /// What was typed.
    pub input: String
}

impl fmt::Display for InvalidKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid kind '{}': simple, polling, listener or menu",
            self.input
        )
    }
}

impl ModuleKind {
    /// Reads the `--kind` flag, defaulting to a static entry.
    pub fn from_flag(flag: Option<&str>) -> Result<Self, InvalidKind> {
        match flag {
            None | Some("simple") => Ok(Self::Simple),
            Some("polling") => Ok(Self::Polling),
            Some("listener") => Ok(Self::Listener),
            Some("menu") => Ok(Self::Menu),
            Some(input) => Err(InvalidKind {
                input: input.to_owned()
            })
        }
    }

    /// Whether the shape needs a `module.rs` with a trait implementation.
    ///
    /// A static entry rides the trait defaults entirely.
    #[must_use]
    pub const fn needs_module_file(self) -> bool {
        !matches!(self, Self::Simple)
    }

    /// Imports above the `mod` declarations of the module root.
    pub const fn root_imports(self) -> &'static str {
        match self {
            Self::Simple | Self::Polling => "",
            Self::Listener | Self::Menu => {
                "use tokio::task::JoinHandle;\n\nuse crate::ModuleEventSender;\n\n"
            }
        }
    }

    /// Submodule declarations of the module root.
    #[must_use]
    pub const fn root_modules(self) -> &'static str {
        match self {
            Self::Simple => "mod state;\nmod view;",
            Self::Polling | Self::Listener | Self::Menu => "mod module;\nmod state;\nmod view;"
        }
    }

    /// Rooms the module root documentation tours.
    pub const fn module_line(self) -> &'static str {
        match self {
            Self::Simple => "",
            Self::Polling | Self::Listener | Self::Menu => {
                "\n//! [`module`] starts and stops the background work."
            }
        }
    }

    /// State fields the module root carries.
    pub const fn root_fields(self) -> &'static str {
        match self {
            Self::Simple | Self::Polling => "    value: String,",
            Self::Listener | Self::Menu => {
                "    value:  String,\n    sender: Option<ModuleEventSender<Message>>,\n    task:   Option<JoinHandle<()>>,"
            }
        }
    }

    /// Constructor values matching the fields above.
    pub const fn root_init(self) -> &'static str {
        match self {
            Self::Simple | Self::Polling => "            value: String::from(\"{{Pascal}}\"),",
            Self::Listener | Self::Menu => {
                "            value:  String::from(\"{{Pascal}}\"),\n            sender: None,\n            task:   None,"
            }
        }
    }

    /// Debug support: derived while every field prints, manual once the
    /// root holds handles and senders.
    pub const fn debug_pieces(self) -> (&'static str, &'static str) {
        match self {
            Self::Simple | Self::Polling => ("#[derive(Debug)]\n", ""),
            Self::Listener | Self::Menu => (
                "",
                "impl std::fmt::Debug for {{Pascal}} {\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        f.debug_struct(\"{{Pascal}}\")\n            .field(\"value\", &self.value)\n            .field(\"sender\", &self.sender)\n            .field(\"task\", &self.task)\n            .finish()\n    }\n}\n"
            )
        }
    }

    /// What a press on the entry answers with.
    pub const fn press_action(self) -> &'static str {
        match self {
            Self::Simple | Self::Polling | Self::Listener => "None",
            Self::Menu => "Some(OnModulePress::ToggleMenu(MenuType::{{Pascal}}))"
        }
    }

    /// Extra view import a menu entry needs for its press action.
    pub const fn view_imports(self) -> &'static str {
        match self {
            Self::Simple | Self::Polling | Self::Listener => "",
            Self::Menu => "use crate::menu::MenuType;\n\n"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_flag_scaffolds_a_static_entry() {
        assert_eq!(
            ModuleKind::from_flag(None).expect("default"),
            ModuleKind::Simple
        );
    }

    #[test]
    fn every_flag_spells_its_shape() {
        assert_eq!(
            ModuleKind::from_flag(Some("polling")).expect("polling"),
            ModuleKind::Polling
        );
        assert_eq!(
            ModuleKind::from_flag(Some("listener")).expect("listener"),
            ModuleKind::Listener
        );
        assert_eq!(
            ModuleKind::from_flag(Some("menu")).expect("menu"),
            ModuleKind::Menu
        );
        assert!(ModuleKind::from_flag(Some("widget")).is_err());
    }

    #[test]
    fn only_a_static_entry_skips_the_trait_file() {
        assert!(!ModuleKind::Simple.needs_module_file());
        assert!(
            ModuleKind::from_flag(Some("polling"))
                .expect("polling")
                .needs_module_file()
        );
        assert!(
            ModuleKind::from_flag(Some("menu"))
                .expect("menu")
                .needs_module_file()
        );
    }

    #[test]
    fn listener_and_menu_share_the_registration_shape() {
        assert_eq!(
            ModuleKind::Listener.root_fields(),
            ModuleKind::Menu.root_fields()
        );
        assert_eq!(
            ModuleKind::Listener.root_init(),
            ModuleKind::Menu.root_init()
        );
    }
}
