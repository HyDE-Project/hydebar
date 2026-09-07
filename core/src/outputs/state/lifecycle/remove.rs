//! Teardown of surfaces when a monitor departs or the bar leaves.
//!
//! A departed monitor keeps its record so a return finds it again, and the
//! last surface group to go is replaced with a fallback so the bar never runs
//! without a surface. On the way out everything is destroyed at once, because
//! a bar being replaced has to leave the screen before the process does.

use iced::{OutputId, Task};
use log::debug;

use super::super::{Outputs, ShellInfo};
use crate::{
    config::{AppearanceStyle, Position},
    menu::Menu,
    outputs::wayland::{LayerSurfaceCreation, create_layer_surfaces, destroy_layer_surfaces}
};

impl Outputs {
    /// Remove the layer-surfaces associated with a departed monitor.
    ///
    /// The returned [`Task`] destroys the compositor resources and potentially
    /// spawns a fallback surface when no monitors remain.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let task = outputs.remove(style, position, wl_output, &config);
    /// spawn(task);
    /// ```
    pub fn remove<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        position: Position,
        wl_output: OutputId,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.0.iter().position(|(_, _, assigned_wl_output)| {
            assigned_wl_output
                .as_ref()
                .is_some_and(|assigned_wl_output| *assigned_wl_output == wl_output)
        }) {
            Some(index_to_remove) => {
                debug!("Removing layer surface for output");

                let (name, shell_info, wl_output) = self.0.swap_remove(index_to_remove);

                let destroy_task = if let Some(shell_info) = shell_info {
                    destroy_layer_surfaces(
                        shell_info.id,
                        shell_info.menu.id,
                        shell_info.tooltip_id,
                        shell_info.desk_id,
                        shell_info.notifications_id
                    )
                } else {
                    Task::none()
                };

                self.0.push((name, None, wl_output));

                if self.0.iter().any(|(_, shell_info, _)| shell_info.is_some()) {
                    Task::batch(vec![destroy_task])
                } else {
                    debug!("No outputs left, creating a fallback layer surface");

                    let LayerSurfaceCreation {
                        main_id,
                        menu_id,
                        tooltip_id,
                        desk_id,
                        notifications_id,
                        task
                    } = create_layer_surfaces(
                        style,
                        None,
                        position,
                        config.menu_keyboard_focus,
                        config.appearance.scale_factor,
                        config.appearance.height_px(),
                        config.layer
                    );

                    self.0.push((
                        None,
                        Some(ShellInfo {
                            id: main_id,
                            menu: Menu::new(menu_id, None),
                            position,
                            style,
                            scale_factor: config.appearance.scale_factor,
                            height: config.appearance.height_px(),
                            tooltip_id,
                            tooltip: None,
                            desk_id,
                            notifications_id
                        }),
                        None
                    ));

                    Task::batch(vec![destroy_task, task])
                }
            }
            _ => Task::none()
        }
    }

    /// Remove every surface the bar currently owns.
    ///
    /// Used on the way out: the compositor drops the surfaces when the client
    /// disconnects anyway, but a bar that is being replaced has to leave the
    /// screen before the process goes away, otherwise the successor draws over
    /// a bar that is still visible.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let task = outputs.destroy_all();
    /// spawn(task);
    /// ```
    pub fn destroy_all<Message: 'static>(&mut self) -> Task<Message> {
        let tasks = self
            .0
            .iter_mut()
            .filter_map(|(_, shell_info, _)| shell_info.take())
            .map(|shell_info| {
                destroy_layer_surfaces(
                    shell_info.id,
                    shell_info.menu.id,
                    shell_info.tooltip_id,
                    shell_info.desk_id,
                    shell_info.notifications_id
                )
            })
            .collect::<Vec<_>>();

        debug!("Destroying {} output surface groups", tasks.len());

        Task::batch(tasks)
    }
}
