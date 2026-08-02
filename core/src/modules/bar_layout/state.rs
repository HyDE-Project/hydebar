//! Message folding and command dispatch for the bar layout module.

use iced::Task;
use log::error;

use super::{BarLayout, Message, roster};
use crate::utils::hyde_shell;

impl BarLayout {
    /// Starts reading the desktop's layouts, off this thread.
    #[must_use]
    pub fn load_entries(&mut self) -> Task<Message> {
        self.loading = true;

        Task::perform(
            async {
                tokio::task::spawn_blocking(roster::list_layouts)
                    .await
                    .unwrap_or_default()
            },
            Message::Listed
        )
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
