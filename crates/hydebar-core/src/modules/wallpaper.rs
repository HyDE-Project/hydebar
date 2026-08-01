//! Bar module driving the desktop wallpaper.
//!
//! The wallpaper is its own thing on the desktop, changed on its own and for
//! its own reasons, so it gets its own entry rather than riding along inside
//! the theme. A theme is a whole look; a wallpaper is one picture inside it,
//! and the two are asked for at different moments.
//!
//! Pressing the entry moves forward, the right button moves back, and the
//! middle button opens the picker: a grid of the theme's wallpapers drawn
//! from the square thumbnails `HyDE` already keeps in its cache, one press on
//! a tile and the desktop wears it.

use hydebar_proto::config::Config;
use iced::{
    Element, Length, Task,
    widget::{Column, Row}
};
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
struct ListedWallpaper {
    /// Full path of the picture, what a set command takes.
    path:     String,
    /// File name, the tile's caption.
    basename: String,
    /// Square thumbnail `HyDE` keeps in its cache.
    sqre:     String
}

/// Side the decoded thumbnails are scaled to, in pixels.
const THUMB_SIDE: u32 = 256;

/// One wallpaper ready to draw: path to set, pixels to show.
///
/// The thumbnail is decoded here rather than handed to the renderer as a
/// path: the renderer stays silent about a file it cannot read, and a grid
/// of invisible tiles is exactly the bug this replaced. Decoded pixels
/// either exist or the tile is not offered.
#[derive(Debug, Clone)]
pub struct WallpaperEntry {
    /// Full path of the picture, what a set command takes.
    pub path:  String,
    /// Decoded square thumbnail.
    thumbnail: iced::widget::image::Handle
}

/// Reads the wallpapers of the theme in force from the desktop.
///
/// A failure answers with an empty list and the picker says so; the desktop
/// not being `HyDE` is not an error the bar can fix.
fn list_wallpapers(
    known: &std::collections::HashMap<String, iced::widget::image::Handle>
) -> Vec<WallpaperEntry> {
    let Ok(output) = std::process::Command::new("hydectl")
        .args(["wallpaper", "list"])
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let listed: Vec<ListedWallpaper> = serde_json::from_slice(&output.stdout).unwrap_or_default();

    listed
        .into_iter()
        .filter_map(|entry| {
            if let Some(thumbnail) = known.get(&entry.path) {
                return Some(WallpaperEntry {
                    path:      entry.path,
                    thumbnail: thumbnail.clone()
                });
            }

            let decoded = std::fs::read(&entry.sqre)
                .ok()
                .and_then(|bytes| ::image::load_from_memory(&bytes).ok())
                .or_else(|| {
                    std::fs::read(&entry.path)
                        .ok()
                        .and_then(|bytes| ::image::load_from_memory(&bytes).ok())
                })?
                .thumbnail(THUMB_SIDE, THUMB_SIDE)
                .into_rgba8();
            let (width, height) = decoded.dimensions();

            Some(WallpaperEntry {
                path:      entry.path,
                thumbnail: iced::widget::image::Handle::from_rgba(
                    width,
                    height,
                    decoded.into_raw()
                )
            })
        })
        .collect()
}

/// Choice made in the wallpaper module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Ask `HyDE` for the next wallpaper of the theme in force.
    Next,
    /// Ask `HyDE` for the previous wallpaper of the theme in force.
    Previous,
    /// Report that the wallpaper change has ended.
    Changed {
        /// Why the desktop refused, if it did.
        failure: Option<String>
    },
    /// Deliver the wallpapers of the theme in force to the picker.
    Listed(Vec<WallpaperEntry>),
    /// Advance the loading indicator by one frame.
    Tick,
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
    entries: Vec<WallpaperEntry>,
    /// Whether a listing is being read right now.
    ///
    /// While it is, the bar entry itself spins: the press was taken and
    /// the pictures are on their way, said in place without a window of
    /// placeholder text jumping in first.
    loading: bool,
    /// Frame the loading indicator is on.
    spinner: crate::modules::themes::Spinner
}

impl Wallpaper {
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

    /// Whether a listing is being read right now.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        self.loading
    }

    /// Starts reading the theme's wallpapers, off this thread.
    ///
    /// Thumbnails already decoded ride along and are reused by path, so a
    /// reopened picker decodes nothing and a theme switch decodes only the
    /// pictures it brought.
    #[must_use]
    pub fn load_entries(&mut self) -> Task<Message> {
        self.loading = true;

        let known: std::collections::HashMap<String, iced::widget::image::Handle> = self
            .entries
            .iter()
            .map(|entry| (entry.path.clone(), entry.thumbnail.clone()))
            .collect();

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || list_wallpapers(&known))
                    .await
                    .unwrap_or_default()
            },
            Message::Listed
        )
    }

    /// Renders the picker: the theme's wallpapers as pressable tiles.
    #[must_use]
    pub fn menu_view<'a>(&self, font_size: f32) -> Element<'a, Message> {
        let tile = scale::scaled(font_size * 7.0);
        let gap = scale::scaled(6.0);
        let mut grid = Column::new().spacing(gap);

        for band in self.entries.chunks(PICKER_COLUMNS) {
            let mut row = Row::new().spacing(gap);

            for entry in band {
                let thumb = iced::widget::image(entry.thumbnail.clone())
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
                self.loading = false;
            }
            Message::Tick => {
                self.spinner.advance();
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
        if self.loading {
            return Some((
                crate::components::icons::icon_raw(self.spinner.glyph().to_owned()).into(),
                None
            ));
        }

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
