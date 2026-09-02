//! Registration of a freshly appeared monitor.
//!
//! A monitor the configuration asked for gets a full surface group, replacing
//! whatever the same name owned before and retiring the fallback surface a
//! bar without outputs runs on; one the configuration did not ask for is
//! recorded without surfaces so a later reload can still find it.

use iced::{OutputId, Task};
use log::debug;

use super::super::{Outputs, ShellInfo};
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
    /// let task =
    ///     outputs.add(style, &config.outputs, position, name, wl_output, &config, 1.0, None);
    /// spawn(task);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn add<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        request_outputs: &config::Outputs,
        position: Position,
        name: &str,
        wl_output: OutputId,
        config: &crate::config::Config,
        scale_factor: f64,
        height: Option<f32>
    ) -> Task<Message> {
        let target = is_output_requested(Some(name), request_outputs);

        if target {
            debug!("Found target output, creating a new layer surface");

            let LayerSurfaceCreation {
                main_id,
                menu_id,
                tooltip_id,
                desk_id,
                notifications_id,
                task
            } = create_layer_surfaces(
                style,
                Some(wl_output),
                position,
                config.menu_keyboard_focus,
                scale_factor,
                height,
                config.layer
            );

            let destroy_task = match self
                .0
                .iter()
                .position(|(key, _, _)| key.as_deref() == Some(name))
            {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => destroy_layer_surfaces(
                            shell_info.id,
                            shell_info.menu.id,
                            shell_info.tooltip_id,
                            shell_info.desk_id,
                            shell_info.notifications_id
                        ),
                        _ => Task::none()
                    }
                }
                _ => Task::none()
            };

            self.0.push((
                Some(name.to_owned()),
                Some(ShellInfo {
                    id: main_id,
                    menu: Menu::new(menu_id, Some(wl_output)),
                    position,
                    style,
                    scale_factor: config.appearance.scale_factor,
                    height: config.appearance.height_px(),
                    tooltip_id,
                    tooltip: None,
                    desk_id,
                    notifications_id
                }),
                Some(wl_output)
            ));

            let destroy_fallback_task = match self.0.iter().position(|(key, _, _)| key.is_none()) {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => destroy_layer_surfaces(
                            shell_info.id,
                            shell_info.menu.id,
                            shell_info.tooltip_id,
                            shell_info.desk_id,
                            shell_info.notifications_id
                        ),
                        _ => Task::none()
                    }
                }
                _ => Task::none()
            };

            Task::batch(vec![destroy_task, destroy_fallback_task, task])
        } else {
            let destroy_task = match self
                .0
                .iter()
                .position(|(key, _, _)| key.as_deref() == Some(name))
            {
                Some(index) => {
                    let old_output = self.0.swap_remove(index);

                    match old_output.1 {
                        Some(shell_info) => destroy_layer_surfaces(
                            shell_info.id,
                            shell_info.menu.id,
                            shell_info.tooltip_id,
                            shell_info.desk_id,
                            shell_info.notifications_id
                        ),
                        _ => Task::none()
                    }
                }
                _ => Task::none()
            };

            self.0.push((Some(name.to_owned()), None, Some(wl_output)));

            destroy_task
        }
    }
}
