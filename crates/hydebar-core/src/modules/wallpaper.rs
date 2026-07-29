//! Bar module driving the desktop wallpaper.
//!
//! The wallpaper is its own thing on the desktop, changed on its own and for
//! its own reasons, so it gets its own entry rather than riding along inside
//! the theme. A theme is a whole look; a wallpaper is one picture inside it,
//! and the two are asked for at different moments.
//!
//! The entry carries no menu, because a wallpaper has nothing to choose from:
//! the desktop keeps an order and the only two things worth asking for are the
//! next picture in it and the previous one. Those are the two buttons a mouse
//! already has, so pressing the entry moves forward and pressing it with the
//! right button moves back — no surface has to open for either.

use hydebar_proto::config::Config;
use iced::{Element, Task};
use log::error;

use super::{Module, OnModulePress};
use crate::{
    components::icons::{IconTheme, Icons, icon},
    services::hyprland_notify::report,
    utils::hyde_shell
};

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
    }
}

/// State of the wallpaper module.
///
/// The desktop owns the wallpaper and its order, so the module keeps nothing of
/// its own: it asks, and the desktop does the rest.
#[derive(Debug, Clone, Default)]
pub struct Wallpaper;

impl Wallpaper {
    /// Builds the module.
    #[must_use]
    pub fn new() -> Self {
        Self
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

        assert!(matches!(task, Task { .. }));
    }
}
