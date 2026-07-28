//! Tooltip visibility on the tooltip surface of an output.
//!
//! The surface stays out of the way in the background layer and only rises to
//! the overlay while a tooltip is shown, so it never covers the desktop for
//! longer than the hover lasts.

use iced::{
    Task,
    platform_specific::shell::wayland::commands::layer_surface::{Layer, set_layer},
    runtime::{Action, task, window::Action as WindowAction},
    window::Id
};

use super::{Outputs, ShellInfo};
use crate::tooltip::TooltipInfo;

/// Asks the runtime to paint every surface again.
///
/// Nothing else drives the tooltip surface: surfaces are only redrawn while an
/// animation runs or the compositor reconfigures them, so a tooltip appearing
/// or vanishing has to ask for the frame that shows it.
fn redraw_surfaces<Message: 'static>() -> Task<Message> {
    task::effect(Action::Window(WindowAction::RedrawAll))
}

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

    /// Shows `info` on the tooltip surface of the output owning `id`.
    ///
    /// The surface only leaves the background layer once, so a tooltip
    /// following the pointer from module to module costs a redraw and nothing
    /// more.
    pub fn show_tooltip<Message: 'static>(&mut self, id: Id, info: TooltipInfo) -> Task<Message> {
        match self.shell_info_mut(id) {
            Some(shell_info) => {
                if shell_info.tooltip.as_ref() == Some(&info) {
                    return Task::none();
                }

                let was_hidden = shell_info.tooltip.is_none();
                let tooltip_id = shell_info.tooltip_id;
                shell_info.tooltip = Some(info);

                if was_hidden {
                    Task::batch(vec![
                        set_layer(tooltip_id, Layer::Overlay),
                        redraw_surfaces(),
                    ])
                } else {
                    redraw_surfaces()
                }
            }
            _ => Task::none()
        }
    }

    /// Hides the tooltip of the output owning `id`.
    ///
    /// The surface sinks back into the background so it stops covering the
    /// windows below it.
    pub fn hide_tooltip<Message: 'static>(&mut self, id: Id) -> Task<Message> {
        match self.shell_info_mut(id) {
            Some(shell_info) => {
                if shell_info.tooltip.take().is_none() {
                    return Task::none();
                }

                Task::batch(vec![
                    set_layer(shell_info.tooltip_id, Layer::Background),
                    redraw_surfaces(),
                ])
            }
            _ => Task::none()
        }
    }

    /// Returns the tooltip the output owning `id` is showing.
    pub fn tooltip(&self, id: Id) -> Option<&TooltipInfo> {
        self.0
            .iter()
            .find_map(|(_, shell_info, _)| match shell_info {
                Some(shell_info) if shell_info.owns(id) => shell_info.tooltip.as_ref(),
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

        let _task: Task<()> = outputs.show_tooltip(id, info("Memory"));

        assert_eq!(outputs.tooltip(id), Some(&info("Memory")));
        assert_eq!(outputs.tooltip(menu_id), Some(&info("Memory")));
    }

    #[test]
    fn hiding_clears_the_tooltip() {
        let (mut outputs, id) = outputs();
        let _show: Task<()> = outputs.show_tooltip(id, info("Memory"));

        let _hide: Task<()> = outputs.hide_tooltip(id);

        assert!(outputs.tooltip(id).is_none());
    }

    #[test]
    fn a_tooltip_is_shown_next_to_an_open_menu() {
        let (mut outputs, id) = outputs();
        let config = Config::default();
        let _menu: Task<()> = outputs.toggle_menu(
            id,
            crate::menu::MenuType::Settings,
            info("Memory").anchor,
            &config
        );

        let _task: Task<()> = outputs.show_tooltip(id, info("Memory"));

        assert_eq!(outputs.tooltip(id), Some(&info("Memory")));
    }

    #[test]
    fn opening_a_menu_drops_the_tooltip_it_would_cover() {
        let (mut outputs, id) = outputs();
        let config = Config::default();
        let _task: Task<()> = outputs.show_tooltip(id, info("Memory"));

        let _menu: Task<()> = outputs.toggle_menu(
            id,
            crate::menu::MenuType::Settings,
            info("Memory").anchor,
            &config
        );

        assert!(outputs.tooltip(id).is_none());
    }
}
