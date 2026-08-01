//! Bar module driving the desktop wallpaper.
//!
//! The wallpaper is its own thing on the desktop, changed on its own and for
//! its own reasons, so it gets its own entry rather than riding along inside
//! the theme. A theme is a whole look; a wallpaper is one picture inside it,
//! and the two are asked for at different moments.
//!
//! Pressing the entry moves forward, the right button moves back, and the
//! middle button opens the picker: a grid of the theme's wallpapers drawn
//! from the square thumbnails HyDE already keeps in its cache, one press on
//! a tile and the desktop wears it.

use hydebar_proto::config::Config;
use iced::{Element, Length, Task, widget::{Column, Row, container}};
use log::error;
use serde::Deserialize;

use super::{Module, OnModulePress};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale
    },
    services::hyprland_notify::report,
    utils::hyde_shell
};

/// Tiles per row of the picker grid.
const PICKER_COLUMNS: usize = 3;

/// One wallpaper as the desktop lists it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WallpaperEntry {
    /// Full path of the picture, what a set command takes.
    pub path:     String,
    /// File name, the tile's caption.
    pub basename: String,
    /// Square thumbnail HyDE keeps in its cache.
    pub sqre:     String
}

/// Reads the wallpapers of the theme in force from the desktop.
///
/// A failure answers with an empty list and the picker says so; the desktop
/// not being HyDE is not an error the bar can fix.
fn list_wallpapers() -> Vec<WallpaperEntry> {
    let Ok(output) = std::process::Command::new("hydectl")
        .args(["wallpaper", "list"])
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

/// Choice made in the wallpaper module.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Ask HyDE for the next wallpaper of the theme in force.
    Next,
    /// Ask HyDE for the previous wallpaper of the theme in force.
    Previous,
    /// Report that the wallpaper change has ended.
    Changed {
        /// Why the desktop refused, if it did.
        failure: Option<String>
    },
    /// Deliver the wallpapers of the theme in force to the picker.
    Listed(Vec<WallpaperEntry>),
    /// Ask the desktop to wear the picture at the given path.
    Pick(String)
}

/// State of the wallpaper module.
///
/// The desktop owns the wallpaper and its order, so the module keeps nothing of
/// its own: it asks, and the desktop does the rest.
#[derive(Debug, Clone, Default)]
pub struct Wallpaper {
    /// Wallpapers of the theme in force, while the picker shows them.
    entries: Vec<WallpaperEntry>
}

impl Wallpaper {
    /// Builds the module.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts reading the theme's wallpapers, off this thread.
    #[must_use]
    pub fn load_entries(&self) -> Task<Message> {
        Task::perform(
            async {
                tokio::task::spawn_blocking(list_wallpapers)
                    .await
                    .unwrap_or_default()
            },
            Message::Listed
        )
    }

    /// Renders the picker: the theme's wallpapers as pressable tiles.
    pub fn menu_view<'a>(&self, font_size: f32) -> Element<'a, Message> {
        if self.entries.is_empty() {
            return container(
                crate::components::text::text("no wallpapers to offer")
                    .size(scale::scaled(font_size))
            )
            .padding(scale::scaled(8.0))
            .into();
        }

        let tile = scale::scaled(font_size * 7.0);
        let gap = scale::scaled(6.0);
        let mut grid = Column::new().spacing(gap);

        for band in self.entries.chunks(PICKER_COLUMNS) {
            let mut row = Row::new().spacing(gap);

            for entry in band {
                let thumb = iced::widget::image(iced::widget::image::Handle::from_path(
                    &entry.sqre
                ))
                .width(Length::Fixed(tile))
                .height(Length::Fixed(tile))
                .content_fit(iced::ContentFit::Cover);

                row = row.push(
                    iced::widget::mouse_area(thumb).on_press(Message::Pick(entry.path.clone()))
                );
            }

            grid = grid.push(row);
        }

        grid.into()
    }

    /// Applies a press made on the module.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::Next => {
                return Task::perform(hyde_shell::run(hyde_shell::next_wallpaper()), |failure| {
                    Message::Changed {
                        failure
                    }
                });
            }
            Message::Previous => {
                return Task::perform(
                    hyde_shell::run(hyde_shell::previous_wallpaper()),
                    |failure| Message::Changed {
                        failure
                    }
                );
            }
            Message::Changed {
                failure
            } => {
                if let Some(reason) = failure {
                    error!("the wallpaper could not be changed: {reason}");
                    report(config, "the desktop refused to change the wallpaper");
                }
            }
            Message::Listed(entries) => {
                self.entries = entries;
            }
            Message::Pick(path) => {
                let command = format!("hydectl wallpaper set '{}'", path.replace('\'', "'\\''"));

                return Task::perform(hyde_shell::run(command), |failure| Message::Changed {
                    failure
                });
            }
        }

        Task::none()
    }
}

impl<M> Module<M> for Wallpaper
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        Some((icon(icons, Icons::Wallpaper).into(), None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_directions_ask_the_desktop_for_different_things() {
        assert_ne!(
            hyde_shell::next_wallpaper(),
            hyde_shell::previous_wallpaper()
        );
    }

    #[test]
    fn a_refused_change_is_reported_rather_than_swallowed() {
        let mut wallpaper = Wallpaper::default();

        let task = wallpaper.update(
            Message::Changed {
                failure: Some("no".to_owned())
            },
            &Config::default()
        );

        let _task: Task<Message> = task;
    }
}
