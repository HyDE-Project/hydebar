//! Creation and removal of per output surfaces.
//!
//! A monitor arriving is handled in [`add`], a monitor departing — and the
//! full teardown on the way out — in [`remove`].

use iced::{OutputId, Task};

use super::{Outputs, ShellInfo};
use crate::outputs::wayland::destroy_layer_surfaces;

mod add;
mod remove;

impl Outputs {
    /// Destroys the surface group answering `matches`, if any.
    ///
    /// Adding a monitor retires whatever the same name owned before, and the
    /// fallback goes the same way when a real monitor arrives: both are the
    /// same "find, take out, destroy" step, stated once so they cannot drift
    /// apart on the next surface the group gains. Removal matches by output
    /// rather than by name and keeps its own step.
    fn destroy_matching<Message: 'static>(
        &mut self,
        matches: impl Fn(&(Option<String>, Option<ShellInfo>, Option<OutputId>)) -> bool
    ) -> Task<Message> {
        match self.0.iter().position(matches) {
            Some(index) => self
                .0
                .swap_remove(index)
                .1
                .map_or_else(Task::none, |shell_info| {
                    let ShellInfo {
                        id,
                        menu,
                        tooltip_id,
                        desk_id,
                        frame_id,
                        notifications_id,
                        ..
                    } = shell_info;

                    destroy_layer_surfaces(
                        id,
                        menu.id,
                        tooltip_id,
                        desk_id,
                        frame_id,
                        notifications_id
                    )
                }),
            _ => Task::none()
        }
    }
}
