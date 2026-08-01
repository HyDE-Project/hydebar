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
fn list_layouts() -> Vec<LayoutEntry> {
    let listing = std::process::Command::new("timeout")
        .args(["10", "hyde-shell", "waybar", "--json"])
        .output();

    let Ok(output) = listing else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let listed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();

    let active = active_layout_name();

    let names: Vec<String> = listed["layouts"]
        .as_array()
        .map(|layouts| {
            layouts
                .iter()
                .filter_map(|entry| entry["name"].as_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let chosen = active
        .as_deref()
        .and_then(|active| active_index(&names, active));

    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| LayoutEntry {
            active: Some(index) == chosen,
            name
        })
        .collect()
}

/// The one roster position the recorded name speaks of, if it names one.
///
/// The roster lists `hyprdots/02` while the state may record the bare
/// `02`: an exact match wins outright, and the tail spelling is honoured
/// only while it is unambiguous — `02` beside two differently housed
/// `02`s marks nobody rather than everybody.
fn active_index(names: &[String], recorded: &str) -> Option<usize> {
    if let Some(index) = names.iter().position(|name| name == recorded) {
        return Some(index);
    }

    let mut tails = names
        .iter()
        .enumerate()
        .filter(|(_, name)| name.rsplit('/').next().is_some_and(|tail| tail == recorded));

    let first = tails.next().map(|(index, _)| index);

    match tails.next() {
        Some(_) => None,
        None => first
    }
}

/// The layout name the desktop's state records as in force.
///
/// Read through the same shell-variable reader the layout loader uses,
/// so the picker's active mark and the arrangement on the bar can never
/// disagree about what the record says.
fn active_layout_name() -> Option<String> {
    let staterc = hydebar_proto::hyde_dirs::HydeDirs::from_env()?.staterc();
    let source = std::fs::read_to_string(staterc).ok()?;

    hydebar_proto::shell_vars::value_of(&source, "WAYBAR_LAYOUT_NAME")
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

        Task::perform(
            async {
                tokio::task::spawn_blocking(list_layouts)
                    .await
                    .unwrap_or_default()
            },
            Message::Listed
        )
    }

    /// Renders the picker: the desktop's layouts as pressable cards.
    #[must_use]
    pub fn menu_view<'a>(&self, font_size: f32) -> Element<'a, Message> {
        let gap = scale::scaled(6.0);
        let mut column = Column::new().spacing(gap).padding(scale::scaled(10.0));

        for entry in &self.entries {
            let marker = if entry.active {
                Icons::Point.default_glyph()
            } else {
                Icons::None.default_glyph()
            };

            let face = iced::widget::Row::new()
                .push(icon_raw(marker.to_owned()))
                .push(
                    crate::components::text::text(entry.name.clone())
                        .size(scale::scaled(font_size))
                )
                .spacing(scale::scaled(8.0))
                .align_y(iced::Alignment::Center);

            let card = container(face)
                .padding([scale::scaled(6.0), scale::scaled(12.0)])
                .width(iced::Length::Fill)
                .style({
                    let active = entry.active;
                    move |theme: &iced::Theme| {
                        let palette = theme.extended_palette();

                        if active {
                            container::Style {
                                background: Some(palette.primary.weak.color.into()),
                                text_color: Some(palette.primary.weak.text),
                                border: iced::border::rounded(8),
                                ..container::Style::default()
                            }
                        } else {
                            container::Style {
                                background: Some(palette.background.weak.color.into()),
                                border: iced::border::rounded(8),
                                ..container::Style::default()
                            }
                        }
                    }
                });

            column = column
                .push(iced::widget::mouse_area(card).on_press(Message::Pick(entry.name.clone())));
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

                return self.load_entries();
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
            "hyde-shell waybar --set 'hyprdots/01'; hyde-shell waybar --kill"
        );
    }
}
