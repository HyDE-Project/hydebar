//! Registration of a freshly appeared monitor.
//!
//! A monitor the configuration asked for gets a full surface group, replacing
//! whatever the same name owned before and retiring the fallback surface a
//! bar without outputs runs on; one the configuration did not ask for is
//! recorded without surfaces so a later reload can still find it.

use iced::{OutputId, Task};
use log::debug;
use num_traits::ToPrimitive;

use super::super::{Outputs, ShellInfo};
use crate::{
    HEIGHT,
    config::{self, AppearanceStyle, Position},
    frame::panel_widths,
    menu::Menu,
    outputs::{
        config::is_output_requested,
        wayland::{
            LayerSurfaceCreation, PanelSurfaces, create_layer_surfaces, create_panel_surfaces,
            frame_settings, layer_height, new_layer_surface
        }
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

            #[expect(
                clippy::cast_possible_truncation,
                reason = "the bar height constant is small enough for pixel counts"
            )]
            let fallback_bar_height = HEIGHT as f32;
            let widths = panel_widths(
                &config.appearance.frame,
                position,
                config.appearance.scale_factor
            );

            let PanelSurfaces {
                top_id,
                left_id,
                right_id,
                bottom_id,
                task: panel_task
            } = create_panel_surfaces(
                Some(wl_output),
                config.layer,
                position,
                layer_height(
                    config.appearance.style,
                    config.appearance.scale_factor,
                    config.appearance.height_px()
                )
                .to_f32()
                .unwrap_or(fallback_bar_height),
                widths
            );

            let (frame_id, frame_task) = if config.appearance.frame.enabled {
                let (id, task) = new_layer_surface(frame_settings(Some(wl_output), config.layer));

                (Some(id), Some(task))
            } else {
                (None, None)
            };

            let destroy_task = self.destroy_matching(|entry| entry.0.as_deref() == Some(name));

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
                    notifications_id,
                    frame_id,
                    frame_enabled: config.appearance.frame.enabled,
                    reservations: widths,
                    left_panel_id: left_id,
                    right_panel_id: right_id,
                    bottom_panel_id: bottom_id,
                    top_panel_id: top_id
                }),
                Some(wl_output)
            ));

            let destroy_fallback_task = self.destroy_matching(|entry| entry.0.is_none());

            let mut tasks = vec![destroy_task, destroy_fallback_task, task, panel_task];

            if let Some(frame_task) = frame_task {
                tasks.push(frame_task);
            }

            Task::batch(tasks)
        } else {
            let destroy_task = self.destroy_matching(|entry| entry.0.as_deref() == Some(name));

            self.0.push((Some(name.to_owned()), None, Some(wl_output)));

            destroy_task
        }
    }
}
