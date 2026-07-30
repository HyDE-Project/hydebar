//! Reconciliation of surfaces with the configuration.

use iced::{Anchor, Task, set_anchor, set_exclusive_zone, set_size};
use log::debug;

use super::Outputs;
use crate::{
    config::{self, AppearanceStyle, Position},
    outputs::{config::is_output_requested, wayland::layer_height}
};

impl Outputs {
    /// Synchronise the tracked outputs with the desired configuration.
    ///
    /// The method returns a [`Task`] aggregating all compositor operations
    /// required to add or remove surfaces as well as to update style or
    /// position changes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (mut outputs, _task) =
    ///     Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// let task = outputs.sync::<()>(
    ///     config.appearance.style,
    ///     &config.outputs,
    ///     config.position,
    ///     &config,
    ///     config.appearance.scale_factor,
    ///     config.appearance.height
    /// );
    /// # let _ = task;
    /// ```
    pub fn sync<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        request_outputs: &config::Outputs,
        position: Position,
        config: &crate::config::Config,
        scale_factor: f64,
        height: Option<f32>
    ) -> Task<Message> {
        debug!("Syncing outputs: {self:?}, request_outputs: {request_outputs:?}");

        let to_remove = self
            .0
            .iter()
            .filter_map(|(name, shell_info, wl_output)| {
                if !is_output_requested(name.as_deref(), request_outputs) && shell_info.is_some() {
                    Some(wl_output.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();
        debug!("Removing outputs: {to_remove:?}");

        let to_add = self
            .0
            .iter()
            .filter_map(|(name, shell_info, wl_output)| {
                if is_output_requested(name.as_deref(), request_outputs) && shell_info.is_none() {
                    Some((name.clone(), wl_output.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        debug!("Adding outputs: {to_add:?}");

        let mut tasks = Vec::new();

        for (name, wl_output) in to_add {
            if let Some(wl_output) = wl_output
                && let Some(name) = name
            {
                tasks.push(self.add(
                    style,
                    request_outputs,
                    position,
                    name.as_str(),
                    wl_output,
                    config,
                    scale_factor,
                    height
                ));
            }
        }

        for wl_output in to_remove {
            tasks.push(self.remove(style, position, wl_output, config));
        }

        for shell_info in self.0.iter_mut().filter_map(|(_, shell_info, _)| {
            if let Some(shell_info) = shell_info
                && shell_info.position != position
            {
                Some(shell_info)
            } else {
                None
            }
        }) {
            debug!(
                "Repositioning output: {:?}, new position {:?}",
                shell_info.id, position
            );
            shell_info.position = position;
            tasks.push(set_anchor(
                shell_info.id,
                match position {
                    Position::Top => Anchor::TOP,
                    Position::Bottom => Anchor::BOTTOM
                } | Anchor::LEFT
                    | Anchor::RIGHT
            ));
        }

        for shell_info in self.0.iter_mut().filter_map(|(_, shell_info, _)| {
            if let Some(shell_info) = shell_info
                && (shell_info.style != style
                    || shell_info.scale_factor != config.appearance.scale_factor
                    || shell_info.height != config.appearance.height)
            {
                Some(shell_info)
            } else {
                None
            }
        }) {
            debug!(
                "Change style, scale_factor or height for output: {:?}, new style {:?}, new scale_factor {:?}, new height {:?}",
                shell_info.id, style, config.appearance.scale_factor, config.appearance.height
            );
            shell_info.style = style;
            shell_info.scale_factor = config.appearance.scale_factor;
            shell_info.height = config.appearance.height;
            let height = layer_height(
                style,
                config.appearance.scale_factor,
                config.appearance.height
            );
            tasks.push(Task::batch(vec![
                set_size(shell_info.id, (0, height as u32)),
                set_exclusive_zone(shell_info.id, height as i32),
            ]));
        }

        Task::batch(tasks)
    }
}
