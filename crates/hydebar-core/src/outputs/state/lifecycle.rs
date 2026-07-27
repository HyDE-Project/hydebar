//! Creation and removal of per output surfaces.

use iced::Task;
use log::debug;
use wayland_client::protocol::wl_output::WlOutput;

use super::{Outputs, ShellInfo};
use crate::{
    config::{self, AppearanceStyle, Position},
    menu::Menu,
    outputs::{
        config::is_output_requested,
        wayland::{LayerSurfaceCreation, create_layer_surfaces, destroy_layer_surfaces}
    }
};

impl Outputs {
    /// Register a new monitor if it matches the configuration filters.
    ///
    /// Callers must execute the returned [`Task`] to materialise the
    /// compositor-side layer-surfaces. When the monitor name is not requested
    /// by configuration the [`Task`] is empty and the state records the
    /// Wayland output for future synchronisation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let (mut outputs, _) = Outputs::new(style, position, &config);
    /// let wl_output = obtain_wl_output();
    /// let task = outputs.add(style, &config.outputs, position, name, wl_output, &config);
    /// spawn(task);
    /// ```
    pub fn add<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        request_outputs: &config::Outputs,
        position: Position,
        name: &str,
        wl_output: WlOutput,
        config: &crate::config::Config
    ) -> Task<Message> {
        let target = is_output_requested(Some(name), request_outputs);

        if target {
            debug!("Found target output, creating a new layer surface");

            let LayerSurfaceCreation {
                main_id,
                menu_id,
                task
            } = create_layer_surfaces(
                style,
                Some(wl_output.clone()),
                position,
                config.menu_keyboard_focus,
                config.appearance.scale_factor
            );

            let destroy_task = match self
                .0
                .iter()
                .position(|(key, _, _)| key.as_deref() == Some(name))
            {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => {
                            destroy_layer_surfaces(shell_info.id, shell_info.menu.id)
                        }
                        _ => Task::none()
                    }
                }
                _ => Task::none()
            };

            self.0.push((
                Some(name.to_owned()),
                Some(ShellInfo {
                    id: main_id,
                    menu: Menu::new(menu_id),
                    position,
                    style,
                    scale_factor: config.appearance.scale_factor
                }),
                Some(wl_output)
            ));

            let destroy_fallback_task = match self.0.iter().position(|(key, _, _)| key.is_none()) {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => {
                            destroy_layer_surfaces(shell_info.id, shell_info.menu.id)
                        }
                        _ => Task::none()
                    }
                }
                _ => Task::none()
            };

            Task::batch(vec![destroy_task, destroy_fallback_task, task])
        } else {
            self.0.push((Some(name.to_owned()), None, Some(wl_output)));

            Task::none()
        }
    }

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
        wl_output: WlOutput,
        config: &crate::config::Config
    ) -> Task<Message> {
        match self.0.iter().position(|(_, _, assigned_wl_output)| {
            assigned_wl_output
                .as_ref()
                .map(|assigned_wl_output| *assigned_wl_output == wl_output)
                .unwrap_or_default()
        }) {
            Some(index_to_remove) => {
                debug!("Removing layer surface for output");

                let (name, shell_info, wl_output) = self.0.swap_remove(index_to_remove);

                let destroy_task = if let Some(shell_info) = shell_info {
                    destroy_layer_surfaces(shell_info.id, shell_info.menu.id)
                } else {
                    Task::none()
                };

                self.0.push((name.to_owned(), None, wl_output));

                if !self.0.iter().any(|(_, shell_info, _)| shell_info.is_some()) {
                    debug!("No outputs left, creating a fallback layer surface");

                    let LayerSurfaceCreation {
                        main_id,
                        menu_id,
                        task
                    } = create_layer_surfaces(
                        style,
                        None,
                        position,
                        config.menu_keyboard_focus,
                        config.appearance.scale_factor
                    );

                    self.0.push((
                        None,
                        Some(ShellInfo {
                            id: main_id,
                            menu: Menu::new(menu_id),
                            position,
                            style,
                            scale_factor: config.appearance.scale_factor
                        }),
                        None
                    ));

                    Task::batch(vec![destroy_task, task])
                } else {
                    Task::batch(vec![destroy_task])
                }
            }
            _ => Task::none()
        }
    }
}
