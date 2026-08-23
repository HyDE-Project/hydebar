//! The `HyDE` menu: the desktop's own menu tree, drawn by the bar.
//!
//! The reference waybar opens a GTK menu built from an XML file the `HyDE`
//! Project ships, and every item runs a command from the module's action
//! table. This module reads the very same two files — nothing about the menu
//! is stated here — and renders the tree in the bar's own window: submenus
//! unfold in place, a leaf runs its command and the window closes.
//!
//! One folder, three rooms: [`definition`] reads the desktop's module table,
//! [`tree`] parses its menu file, [`view`] draws the bar entry and the tree
//! it opens. The root holds the state and the messages.

use iced::Element;

use crate::modules::Module;

mod definition;
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
    expanded: std::collections::HashSet<String>
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
            async {
                tokio::task::spawn_blocking(|| {
                    let Some(definition) = definition::read_definition() else {
                        log::warn!("the desktop ships no menu definition, the menu stays empty");
                        return None;
                    };

                    let tree = definition
                        .menu_file
                        .as_deref()
                        .and_then(tree::read_tree)
                        .unwrap_or_default();

                    Some((definition.glyph, definition.actions, tree))
                })
                .await
                .ok()
                .flatten()
            },
            |loaded| {
                let (glyph, actions, tree) = loaded.unwrap_or_default();

                Message::Loaded {
                    glyph,
                    actions,
                    tree
                }
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
}

impl<M> Module<M> for HydeMenu
where
    M: 'static
{
    type ViewData<'a> = ();
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        _ctx: &crate::ModuleContext,
        _data: Self::RegistrationData<'_>
    ) -> Result<(), super::ModuleError> {
        self.reload();

        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

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
