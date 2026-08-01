//! Bar entry stepping and picking the `HyDE` bar layout in force.
//!
//! The desktop owns the layout roster and the record of the one in force;
//! the module asks, and the desktop does the rest — the bar itself follows
//! through the state watch it already keeps. The mouse speaks the same
//! dialect as the wallpaper entry: the side buttons step, the middle
//! button opens the picker.

use iced::{
    Element, Task,
    widget::{Column, container}
};
use log::error;

use crate::{
    ModuleContext,
    components::{
        icons::{IconTheme, Icons, icon, icon_raw},
        scale
    },
    modules::{Module, ModuleError, OnModulePress},
    utils::hyde_shell
};

/// One layout the desktop offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutEntry {
    /// Name the desktop lists the layout under.
    pub name:   String,
    /// Whether this is the layout in force.
    pub active: bool
}

/// Messages the module answers.
#[derive(Debug, Clone)]
pub enum Message {
    /// Ask the desktop for the next layout.
    Next,
    /// Ask the desktop for the previous layout.
    Previous,
    /// Report that the layout change has ended.
    Changed {
        /// Why the desktop refused, if it did.
        failure: Option<String>
    },
    /// Deliver the desktop's layout roster to the picker.
    Listed(Vec<LayoutEntry>),
    /// Ask the desktop to arrange the bar by the named layout.
    Pick(String),
    /// Advance the loading indicator by one frame.
    Tick
}

/// State of the bar layout module.
#[derive(Debug, Clone, Default)]
pub struct BarLayout {
    /// Layouts the desktop offers, while the picker shows them.
    entries: Vec<LayoutEntry>,
    /// Whether the roster is being read right now.
    loading: bool,
    /// Frame the loading indicator is on.
    spinner: crate::modules::themes::Spinner
}

/// Reads the roster and the record of the layout in force.
fn list_layouts(previous: &[LayoutEntry]) -> Vec<LayoutEntry> {
    let _ = previous;

    let Ok(output) = std::process::Command::new("hyde-shell")
        .args(["waybar", "--json"])
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let listed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();

    let active = active_layout_name();

    listed["layouts"]
        .as_array()
        .map(|layouts| {
            layouts
                .iter()
                .filter_map(|entry| entry["name"].as_str())
                .map(|name| LayoutEntry {
                    name:   name.to_owned(),
                    active: active.as_deref() == Some(name)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The layout name the desktop's state records as in force.
fn active_layout_name() -> Option<String> {
    let staterc = hydebar_proto::hyde_dirs::HydeDirs::from_env()?.staterc();
    let source = std::fs::read_to_string(staterc).ok()?;

    source.lines().find_map(|line| {
        let value = line.strip_prefix("WAYBAR_LAYOUT_NAME=")?;

        Some(value.trim().trim_matches('"').to_owned())
    })
}

impl BarLayout {
    /// Builds the module.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the picker has nothing to show yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether the roster is being read right now.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        self.loading
    }

    /// Starts reading the desktop's layouts, off this thread.
    #[must_use]
    pub fn load_entries(&mut self) -> Task<Message> {
        self.loading = true;

        let previous = self.entries.clone();

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || list_layouts(&previous))
                    .await
                    .unwrap_or_default()
            },
            Message::Listed
        )
    }

    /// Renders the picker: the desktop's layouts as pressable rows.
    #[must_use]
    pub fn menu_view<'a>(&self, font_size: f32) -> Element<'a, Message> {
        let gap = scale::scaled(4.0);
        let mut column = Column::new().spacing(gap).padding(scale::scaled(8.0));

        for entry in &self.entries {
            let label =
                crate::components::text::text(entry.name.clone()).size(scale::scaled(font_size));

            let row = container(label)
                .padding([scale::scaled(4.0), scale::scaled(10.0)])
                .width(iced::Length::Fill)
                .style({
                    let active = entry.active;
                    move |theme: &iced::Theme| {
                        if active {
                            container::Style {
                                background: Some(
                                    theme.extended_palette().primary.weak.color.into()
                                ),
                                text_color: Some(theme.extended_palette().primary.weak.text),
                                border: iced::border::rounded(6),
                                ..container::Style::default()
                            }
                        } else {
                            container::Style::default()
                        }
                    }
                });

            column = column
                .push(iced::widget::mouse_area(row).on_press(Message::Pick(entry.name.clone())));
        }

        column.into()
    }

    /// Applies a press made on the module.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Next => {
                return Task::perform(hyde_shell::run(hyde_shell::next_bar_layout()), |failure| {
                    Message::Changed {
                        failure
                    }
                });
            }
            Message::Previous => {
                return Task::perform(
                    hyde_shell::run(hyde_shell::previous_bar_layout()),
                    |failure| Message::Changed {
                        failure
                    }
                );
            }
            Message::Changed {
                failure
            } => {
                if let Some(reason) = failure {
                    error!("the bar layout could not be changed: {reason}");
                }
            }
            Message::Listed(entries) => {
                self.entries = entries;
                self.loading = false;
            }
            Message::Pick(name) => {
                return Task::perform(
                    hyde_shell::run(hyde_shell::set_bar_layout(&name)),
                    |failure| Message::Changed {
                        failure
                    }
                );
            }
            Message::Tick => {
                self.spinner.advance();
            }
        }

        Task::none()
    }
}

impl<M> Module<M> for BarLayout
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        _ctx: &ModuleContext,
        (): Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if self.loading {
            return Some((icon_raw(self.spinner.glyph().to_owned()).into(), None));
        }

        Some((icon(icons, Icons::BarLayout).into(), None))
    }

    fn subscription(&self) -> Option<iced::Subscription<M>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_directions_ask_the_desktop_for_different_things() {
        assert_ne!(
            hyde_shell::next_bar_layout(),
            hyde_shell::previous_bar_layout()
        );
    }

    #[test]
    fn a_layout_name_is_passed_as_one_quoted_argument() {
        assert_eq!(
            hyde_shell::set_bar_layout("hyprdots/01"),
            "hyde-shell waybar --set 'hyprdots/01' && hyde-shell waybar --kill"
        );
    }
}
