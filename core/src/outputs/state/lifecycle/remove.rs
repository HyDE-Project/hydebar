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
    frame::PanelWidths,
    menu::Menu,
    outputs::wayland::{
        LayerSurfaceCreation, create_layer_surfaces, destroy_layer_surfaces,
        destroy_optional_surfaces
    }
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
                    let frame_and_panels = destroy_optional_surfaces(&[
                        shell_info.frame_id,
                        shell_info.left_panel_id,
                        shell_info.right_panel_id,
                        shell_info.bottom_panel_id,
                        shell_info.top_panel_id
                    ]);

                    Task::batch(vec![
                        destroy_layer_surfaces(
                            shell_info.id,
                            shell_info.menu.id,
                            shell_info.tooltip_id,
                            shell_info.desk_id,
                            shell_info.notifications_id
                        ),
                        frame_and_panels,
                    ])
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
                            notifications_id,
                            frame_id: None,
                            frame_enabled: false,
                            reservations: PanelWidths::default(),
                            left_panel_id: None,
                            right_panel_id: None,
                            bottom_panel_id: None,
                            top_panel_id: None
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
                let frame_and_panels = destroy_optional_surfaces(&[
                    shell_info.frame_id,
                    shell_info.left_panel_id,
                    shell_info.right_panel_id,
                    shell_info.bottom_panel_id,
                    shell_info.top_panel_id
                ]);

                Task::batch(vec![
                    destroy_layer_surfaces(
                        shell_info.id,
                        shell_info.menu.id,
                        shell_info.tooltip_id,
                        shell_info.desk_id,
                        shell_info.notifications_id
                    ),
                    frame_and_panels,
                ])
            })
            .collect::<Vec<_>>();

        debug!("Destroying {} output surface groups", tasks.len());

        Task::batch(tasks)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::config::Config;

    fn config_with(frame_enabled: bool) -> Config {
        let mut config = Config::default();
        config.appearance.frame.enabled = frame_enabled;
        config.outputs = crate::config::Outputs::Targets(vec!["DP-1".to_owned()]);

        config
    }

    fn tracked(config: &Config) -> (Outputs, iced::OutputId) {
        let (mut outputs, _) =
            Outputs::new::<()>(config.appearance.style, config.position, config);
        let id = iced::OutputId::for_tests();
        let task = outputs.add::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            "DP-1",
            id,
            config,
            1.0,
            None
        );
        let _ = task;

        (outputs, id)
    }

    #[test]
    fn a_departed_output_keeps_its_name_but_holds_no_surfaces() {
        let config = config_with(true);
        let (mut outputs, id) = tracked(&config);

        let task = outputs.remove::<()>(config.appearance.style, config.position, id, &config);
        let _ = task;

        // the record stays so a return finds it again; the surfaces are
        // stated destroyed, ring and panels included
        let (_, info, _) = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("the record stays");

        assert!(info.is_none());
    }

    #[test]
    fn the_last_output_out_leaves_a_fallback_behind() {
        let config = config_with(true);
        let (mut outputs, id) = tracked(&config);

        let task = outputs.remove::<()>(config.appearance.style, config.position, id, &config);
        let _ = task;

        assert!(
            outputs
                .0
                .iter()
                .any(|(name, info, _)| name.is_none() && info.is_some())
        );
    }

    #[test]
    fn destroying_everything_holds_no_group() {
        let config = config_with(true);
        let (mut outputs, _) = tracked(&config);

        let task = outputs.destroy_all::<()>();
        let _ = task;

        assert!(outputs.0.iter().all(|(_, info, _)| info.is_none()));
    }

    #[test]
    fn removing_a_stranger_changes_nothing() {
        let config = config_with(true);
        let (mut outputs, _) = tracked(&config);
        let before = outputs.0.len();

        let task = outputs.remove::<()>(
            config.appearance.style,
            config.position,
            iced::OutputId::for_tests(),
            &config
        );
        let _ = task;

        assert_eq!(outputs.0.len(), before);
    }

    #[test]
    fn removing_a_surface_free_record_restates_nothing() {
        let config = config_with(true);
        let (mut outputs, _) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);
        let id = iced::OutputId::for_tests();

        // the configurator never asked for this screen: the record stands
        // surface-free, and taking it down states no destruction at all
        let task = outputs.add::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            "HDMI-1",
            id,
            &config,
            1.0,
            None
        );
        let _ = task;

        let task = outputs.remove::<()>(config.appearance.style, config.position, id, &config);
        let _ = task;

        let (_, info, _) = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("HDMI-1"))
            .expect("the record stays");

        assert!(info.is_none());
    }

    #[test]
    fn removing_one_of_two_leaves_the_other_standing() {
        let mut config = config_with(true);
        config.outputs =
            crate::config::Outputs::Targets(vec!["DP-1".to_owned(), "HDMI-1".to_owned()]);

        let (mut outputs, _) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);
        let dp = iced::OutputId::for_tests();
        let hdmi = iced::OutputId::for_tests();

        for (name, id) in [("DP-1", dp), ("HDMI-1", hdmi)] {
            let task = outputs.add::<()>(
                config.appearance.style,
                &config.outputs,
                config.position,
                name,
                id,
                &config,
                1.0,
                None
            );
            let _ = task;
        }

        let task = outputs.remove::<()>(config.appearance.style, config.position, dp, &config);
        let _ = task;

        // the departed record holds no surfaces, the other group is
        // untouched, and no fallback appears while a group still stands
        let departed = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("the record stays");
        assert!(departed.1.is_none());

        let staying = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("HDMI-1"))
            .expect("the other group stands");
        assert!(
            staying
                .1
                .as_ref()
                .expect("the group stands")
                .frame_id
                .is_some()
        );

        assert!(outputs.0.iter().all(|(name, _, _)| name.is_some()));
    }
}
