//! Bar module configuring the bar itself.
//!
//! Every choice made here is written straight back into the configuration file
//! the bar was started from. The file watcher picks the change up and reloads,
//! so the menu never holds state of its own: what it draws is always what the
//! running configuration says.
//!
//! One folder, seven rooms: [`message`] names the choices and the keys
//! they write, [`steps`] walks the stepped size ranges, [`persist`]
//! stores an edited layout, [`layout`] holds the pure layout edits,
//! [`tab`] the pages, [`view`] the drawing and [`writer`] the file
//! access. The root holds the state the rooms share.

mod layout;
mod message;
mod persist;
mod steps;
mod tab;
mod view;
mod writer;

use std::path::{Path, PathBuf};

use hydebar_proto::config::Config;
use iced::{Element, Task};
pub use layout::{LayoutEdit, Section, Slot};
pub use message::{Message, announce_source};
pub use tab::Tab;
pub use writer::{SettingValue, SettingsWriteError, write_setting};

use super::{Module, OnModulePress};
use crate::{
    components::icons::{IconTheme, Icons, icon},
    menu::MenuType
};

/// Bar entry opening the settings of the bar.
#[derive(Default, Debug, Clone)]
pub struct Settings {
    /// File the choices are written to.
    config_path: PathBuf,
    /// Page the window currently shows.
    tab:         Tab,
    /// Module the editor acts on, once one is picked.
    selected:    Option<Slot>,
    /// Section the editor is showing.
    section:     Section
}

impl Settings {
    /// Creates the module writing to `config_path`.
    #[must_use]
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            tab: Tab::default(),
            selected: None,
            section: Section::Left
        }
    }

    /// Section the editor is showing.
    #[must_use]
    pub const fn section(&self) -> Section {
        self.section
    }

    /// Module the editor acts on, once one is picked.
    #[must_use]
    pub const fn selected(&self) -> Option<Slot> {
        self.selected
    }

    /// Page the window currently shows.
    #[must_use]
    pub const fn tab(&self) -> Tab {
        self.tab
    }

    /// Applies a choice made in the window.
    ///
    /// Picking a tab is the only choice kept in memory; everything else lands
    /// in the configuration file and comes back through the reload.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::SelectTab(tab) => self.tab = tab,
            Message::SelectSlot(slot) => self.selected = slot,
            Message::SelectSection(section) => {
                self.section = section;
                self.selected = None;
            }
            Message::EditLayout(edit) => {
                let modules = layout::apply(&config.modules, &edit);
                persist::store_layout(&self.config_path, &modules);
                self.selected = persist::follow(&edit, &modules);

                if let Some(slot) = self.selected {
                    self.section = slot.section;
                }
            }
            other => other.apply(&self.config_path)
        }

        Task::none()
    }

    /// File the choices are written to.
    #[must_use]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

impl<M> Module<M> for Settings
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        Some((
            icon(icons, Icons::Settings).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Settings))
        ))
    }
}
