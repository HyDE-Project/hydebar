//! Tooltip visibility on the tooltip surface of an output.
//!
//! The surface stays out of the way in the background layer and only rises to
//! the overlay while a tooltip is shown, so it never covers the desktop for
//! longer than the hover lasts.

use iced::{Layer, SurfaceId as Id, Task, set_layer};

use super::{Outputs, ShellInfo};
use crate::{config::ModuleName, tooltip::TooltipInfo};

impl Outputs {
    /// Resolves the output owning `id`, whichever of its surfaces it names.
    fn shell_info_mut(&mut self, id: Id) -> Option<&mut ShellInfo> {
        self.0
            .iter_mut()
            .find_map(|(_, shell_info, _)| match shell_info {
                Some(shell_info) if shell_info.owns(id) => Some(shell_info),
                _ => None
            })
    }

    /// Shows `info` for `owner` on the tooltip surface of the output owning
    /// `id`.
    ///
    /// The surface only leaves the background layer once, so a tooltip
    /// following the pointer from module to module costs a redraw and nothing
    /// more.
    pub fn show_tooltip<Message: 'static>(
        &mut self,
        id: Id,
        owner: ModuleName,
        info: TooltipInfo
    ) -> Task<Message> {
        match self.shell_info_mut(id) {
            Some(shell_info) => {
                if shell_info.tooltip.as_ref() == Some(&(owner.clone(), info.clone())) {
                    return Task::none();
                }

                let was_hidden = shell_info.tooltip.is_none();
                let tooltip_id = shell_info.tooltip_id;
                shell_info.tooltip = Some((owner, info));

                if was_hidden {
                    set_layer(tooltip_id, Layer::Overlay)
                } else {
                    Task::none()
                }
            }
            _ => Task::none()
        }
    }

    /// Hides the tooltip `owner` published on the output owning `id`.
    ///
    /// A tooltip belonging to another module stays: the pointer moving towards
    /// the start of the bar delivers the next module's enter before the
    /// previous module's leave, and that stale leave must not take the fresh
    /// tooltip down. Passing [`None`] hides whatever is showing, for a pointer
    /// that settled on a module with nothing to say.
    pub fn hide_tooltip<Message: 'static>(
        &mut self,
        id: Id,
        owner: Option<&ModuleName>
    ) -> Task<Message> {
        match self.shell_info_mut(id) {
            Some(shell_info) => {
                let shown_by_owner = match (&shell_info.tooltip, owner) {
                    (Some((shown, _)), Some(leaving)) => shown == leaving,
                    (Some(_), None) => true,
                    (None, _) => false
                };

                if !shown_by_owner {
                    return Task::none();
                }

                shell_info.tooltip = None;

                set_layer(shell_info.tooltip_id, Layer::Background)
            }
            _ => Task::none()
        }
    }

    /// Returns the tooltip the output owning `id` is showing.
    pub fn tooltip(&self, id: Id) -> Option<&TooltipInfo> {
        self.0
            .iter()
            .find_map(|(_, shell_info, _)| match shell_info {
                Some(shell_info) if shell_info.owns(id) => {
                    shell_info.tooltip.as_ref().map(|(_, info)| info)
                }
                _ => None
            })
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;

    use super::*;
    use crate::{config::Config, outputs::Outputs, position_button::ButtonUIRef};

    fn info(text: &str) -> TooltipInfo {
        TooltipInfo {
            text:   text.to_owned(),
            anchor: ButtonUIRef {
                position: Point::new(100.0, 19.0),
                viewport: (1360.0, 38.0)
            }
        }
    }

    fn outputs() -> (Outputs, Id) {
        let config = Config::default();
        let (outputs, _task) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);
        let id = outputs
            .0
            .first()
            .and_then(|(_, shell_info, _)| shell_info.as_ref())
            .expect("the fallback surface exists")
            .id;

        (outputs, id)
    }

    #[test]
    fn a_shown_tooltip_is_readable_from_both_surfaces() {
        let (mut outputs, id) = outputs();
        let menu_id = outputs.shell_info_mut(id).expect("output").menu.id;

        let _task: Task<()> = outputs.show_tooltip(id, ModuleName::Clock, info("Memory"));

        assert_eq!(outputs.tooltip(id), Some(&info("Memory")));
        assert_eq!(outputs.tooltip(menu_id), Some(&info("Memory")));
    }

    #[test]
    fn hiding_clears_the_tooltip() {
        let (mut outputs, id) = outputs();
        let _show: Task<()> = outputs.show_tooltip(id, ModuleName::Clock, info("Memory"));

        let _hide: Task<()> = outputs.hide_tooltip(id, Some(&ModuleName::Clock));

        assert!(outputs.tooltip(id).is_none());
    }

    /// Moving towards the start of the bar delivers the next module's enter
    /// before the previous module's leave; the stale leave must not take the
    /// fresh tooltip down with it, or hints never show in that direction.
    #[test]
    fn a_stale_leave_from_another_module_keeps_the_fresh_tooltip() {
        let (mut outputs, id) = outputs();
        let _show: Task<()> = outputs.show_tooltip(id, ModuleName::Clock, info("Clock"));

        let _stale: Task<()> = outputs.hide_tooltip(id, Some(&ModuleName::Battery));

        assert_eq!(outputs.tooltip(id), Some(&info("Clock")));
    }

    /// A pointer settling on a module with nothing to say hides whatever is
    /// showing, whoever it belonged to.
    #[test]
    fn settling_on_a_silent_module_hides_any_tooltip() {
        let (mut outputs, id) = outputs();
        let _show: Task<()> = outputs.show_tooltip(id, ModuleName::Clock, info("Clock"));

        let _hide: Task<()> = outputs.hide_tooltip(id, None);

        assert!(outputs.tooltip(id).is_none());
    }

    #[test]
    fn a_tooltip_is_shown_next_to_an_open_menu() {
        let (mut outputs, id) = outputs();
        let config = Config::default();
        let _menu: Task<()> = outputs.toggle_menu(
            id,
            crate::menu::MenuType::ControlCenter,
            info("Memory").anchor,
            &config
        );

        let _task: Task<()> = outputs.show_tooltip(id, ModuleName::Clock, info("Memory"));

        assert_eq!(outputs.tooltip(id), Some(&info("Memory")));
    }

    #[test]
    fn opening_a_menu_drops_the_tooltip_it_would_cover() {
        let (mut outputs, id) = outputs();
        let config = Config::default();
        let _task: Task<()> = outputs.show_tooltip(id, ModuleName::Clock, info("Memory"));

        let _menu: Task<()> = outputs.toggle_menu(
            id,
            crate::menu::MenuType::ControlCenter,
            info("Memory").anchor,
            &config
        );

        assert!(outputs.tooltip(id).is_none());
    }
}
