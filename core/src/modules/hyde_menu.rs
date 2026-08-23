//! The `HyDE` menu: the desktop's own menu tree, drawn by the bar.
//!
//! The reference waybar opens a GTK menu built from an XML file the `HyDE`
//! Project ships, and every item runs a command from the module's action
//! table. This module reads the very same two files — nothing about the menu
//! is stated here — and renders the tree in the bar's own window: submenus
//! unfold in place, a leaf runs its command and the window closes.
//!
//! One folder, four rooms: [`definition`] reads the desktop's module table,
//! [`tree`] parses its menu file, [`view`] draws the bar entry and the tree
//! it opens, and [`module`] reads the two files when the bar wires the menu
//! up. The root holds the state and the messages.

use iced::Element;

use crate::ModuleEventSender;

mod definition;
mod module;
mod tree;
mod view;

/// What the user asks of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Unfold or fold the submenu with this identifier.
    Toggle(String),
    /// Run the action with this identifier and close the menu.
    Run(iced::SurfaceId, String),
    /// The desktop's menu files, re-read off the drawing thread.
    Loaded {
        /// Glyph the bar entry shows, as the desktop's module states it.
        glyph:   Option<String>,
        /// Command per leaf identifier.
        actions: std::collections::HashMap<String, String>,
        /// The tree the desktop's menu file describes.
        tree:    Vec<Entry>
    }
}

/// One entry of the menu tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A leaf: label shown, action run on press.
    Item {
        /// Key the entry is addressed by.
        id:    String,
        /// What the entry reads as.
        label: String
    },
    /// A dividing line between groups.
    Separator,
    /// A named branch that unfolds in place.
    Submenu {
        /// Key the entry is addressed by.
        id:       String,
        /// What the entry reads as.
        label:    String,
        /// What opens under it.
        children: Vec<Self>
    }
}

/// The `HyDE` menu module.
#[derive(Debug, Default)]
pub struct HydeMenu {
    /// The tree the desktop's menu file describes.
    tree:     Vec<Entry>,
    /// Command per leaf identifier.
    actions:  std::collections::HashMap<String, String>,
    /// Glyph the bar entry shows, as the desktop's module states it.
    glyph:    Option<String>,
    /// Identifiers of the branches currently unfolded.
    expanded: std::collections::HashSet<String>,
    /// Where a read done off the drawing thread reports back.
    sender:   Option<ModuleEventSender<Message>>
}

impl HydeMenu {
    /// Starts re-reading the desktop's files, folding every branch.
    ///
    /// Called when the menu opens, so an edited menu file is picked up
    /// without restarting the bar. The reads and the parse run on the
    /// blocking pool: the files are small, but the opening animation must
    /// not wait on the filesystem.
    pub fn reload(&mut self) -> iced::Task<Message> {
        self.expanded.clear();

        iced::Task::perform(
            async { tokio::task::spawn_blocking(read).await.ok() },
            |loaded| {
                loaded.unwrap_or_else(|| Message::Loaded {
                    glyph:   None,
                    actions: std::collections::HashMap::new(),
                    tree:    Vec::new()
                })
            }
        )
    }

    /// Applies what the user asked, reporting the command to run, if any.
    pub fn update(&mut self, message: Message) -> Option<(iced::SurfaceId, String)> {
        match message {
            Message::Toggle(id) => {
                if !self.expanded.remove(&id) {
                    self.expanded.insert(id);
                }

                None
            }
            Message::Loaded {
                glyph,
                actions,
                tree
            } => {
                self.glyph = glyph;
                self.actions = actions;
                self.tree = tree;

                log::info!(
                    "desktop menu: {} entries, {} actions",
                    self.tree.len(),
                    self.actions.len()
                );

                None
            }
            Message::Run(surface, id) => {
                let command = self.actions.get(&id)?.clone();

                Some((surface, command))
            }
        }
    }

    /// The menu tree as the window shows it.
    #[must_use]
    pub fn menu_view(&self, id: iced::SurfaceId, opacity: f32) -> Element<'_, Message> {
        view::tree_view(&self.tree, &self.expanded, id, opacity)
    }

    /// What the menu offers at its top level, each as its glyph and name.
    ///
    /// Read off the tree already in hand rather than the file it came from:
    /// the canvas asks this on every frame of an unfolding.
    #[must_use]
    pub fn choices(&self) -> Vec<(String, String)> {
        choices_of(&self.tree)
    }
}

/// The choices the desktop's menu for `module` offers, each as its glyph and
/// the name it is called by.
///
/// The desktop ships a menu beside several of its waybar modules — the power
/// module above all — and a bar module the user wired to the same script has
/// nothing of its own to say: it is a button, and the canvas would open it
/// into an empty block. What it can show instead is what pressing it leads
/// to, read from the very files the reference bar reads.
///
/// The top level of the menu and no deeper. A branch stands for what it
/// unfolds into — one glyph for shutting down rather than two for shutting
/// down now and shutting down presently — and a row that spelled out every
/// leaf would say less by saying more.
///
/// A label here reads `󰍁  Lock`: the glyph
/// leads and the name follows, and both are wanted — the canvas has the room
/// to say what a choice is instead of leaving the glyph to carry it alone. A
/// label written in letters alone carries no glyph and is passed over.
///
/// Blocking on purpose, like everything else that reads the desktop's files:
/// the caller decides which pool it runs on.
#[must_use]
pub fn desktop_choices(module: &str) -> Vec<(String, String)> {
    let Some(definition) = definition::read_definition(module) else {
        return Vec::new();
    };

    choices_of(
        &definition
            .menu_file
            .as_deref()
            .and_then(tree::read_tree)
            .unwrap_or_default()
    )
}

/// The choices the top level of `tree` offers, in the order it lists them.
fn choices_of(tree: &[Entry]) -> Vec<(String, String)> {
    tree.iter()
        .filter_map(|entry| match entry {
            Entry::Item {
                label, ..
            }
            | Entry::Submenu {
                label, ..
            } => named(label),
            Entry::Separator => None
        })
        .collect()
}

/// A menu label split into the glyph it leads with and the name it carries.
///
/// The desktop writes its labels glyph first and name after, and the two are
/// told apart by what they are made of: a name is letters and digits, a glyph
/// is neither. A label that leads with letters carries no glyph, and nothing
/// is made of it.
fn named(label: &str) -> Option<(String, String)> {
    let mut words = label.split_whitespace();
    let leading = words.next()?;

    if leading.chars().any(char::is_alphanumeric) {
        return None;
    }

    Some((leading.to_owned(), words.collect::<Vec<&str>>().join(" ")))
}

/// Reads the desktop's two menu files, as the message that folds them in.
///
/// Blocking on purpose — the caller decides which pool it runs on — and one
/// function rather than two, so the read the bar does on its own and the read
/// an opening menu asks for can never disagree about where the menu lives.
fn read() -> Message {
    let Some(definition) = definition::read_definition("hyde-menu") else {
        log::warn!("the desktop ships no menu definition, the menu stays empty");

        return Message::Loaded {
            glyph:   None,
            actions: std::collections::HashMap::new(),
            tree:    Vec::new()
        };
    };

    Message::Loaded {
        glyph:   definition.glyph,
        actions: definition.actions,
        tree:    definition
            .menu_file
            .as_deref()
            .and_then(tree::read_tree)
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_label_gives_up_the_glyph_it_leads_with_and_the_name_it_carries() {
        assert_eq!(
            named("\u{f0341}  Lock"),
            Some(("\u{f0341}".to_owned(), "Lock".to_owned()))
        );
        assert_eq!(
            named("\u{f0bab}  Reboot to UEFI").map(|(_, name)| name),
            Some("Reboot to UEFI".to_owned())
        );
    }

    #[test]
    fn a_label_of_letters_alone_offers_no_choice() {
        assert!(named("Lock").is_none());
        assert!(named("").is_none());
    }

    #[test]
    fn the_choices_are_the_top_level_of_the_menu_and_nothing_deeper() {
        let tree = vec![
            Entry::Item {
                id:    "lock".to_owned(),
                label: "\u{f0341}  Lock".to_owned()
            },
            Entry::Separator,
            Entry::Submenu {
                id:       "shutdown".to_owned(),
                label:    "\u{f06a6}  Shutdown".to_owned(),
                children: vec![Entry::Item {
                    id:    "now".to_owned(),
                    label: "\u{f06a6}  Shutdown Now".to_owned()
                }]
            },
        ];

        assert_eq!(
            choices_of(&tree),
            vec![
                ("\u{f0341}".to_owned(), "Lock".to_owned()),
                ("\u{f06a6}".to_owned(), "Shutdown".to_owned())
            ],
            "a branch stands for what it unfolds into, and a rule for nothing"
        );
    }

    #[test]
    fn a_toggle_unfolds_and_folds_the_same_branch() {
        let mut menu = HydeMenu::default();

        assert!(menu.update(Message::Toggle("a".into())).is_none());
        assert!(menu.expanded.contains("a"));

        assert!(menu.update(Message::Toggle("a".into())).is_none());
        assert!(!menu.expanded.contains("a"));
    }

    #[test]
    fn a_run_reports_the_command_of_the_action() {
        let mut menu = HydeMenu::default();
        menu.actions
            .insert("go".into(), "hyde-shell wallpaper --next".into());

        let run = menu.update(Message::Run(iced::SurfaceId::MAIN, "go".into()));

        assert_eq!(
            run.map(|(_, command)| command),
            Some(String::from("hyde-shell wallpaper --next"))
        );
    }

    #[test]
    fn an_unknown_action_runs_nothing() {
        let mut menu = HydeMenu::default();

        assert!(
            menu.update(Message::Run(iced::SurfaceId::MAIN, "gone".into()))
                .is_none()
        );
    }
}
