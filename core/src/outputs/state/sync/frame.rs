//! Reconciliation of the frame with the configuration.
//!
//! The ring and its reservation panels outlive a config flip. The sync
//! reads what the configuration calls for now, compares it with what each
//! output's surface group carries, and states the difference to the
//! compositor: a switched-on frame gets its fullscreen surface, a
//! switched-off one loses it, and a resized edge is a `set_size` plus a
//! fresh exclusive zone away from real.

use iced::{OutputId, Task, set_anchor, set_exclusive_zone, set_margin, set_size};
use log::debug;

use super::Outputs;
use crate::{
    HEIGHT,
    config::{self, Position},
    frame::{PanelWidths, panel_widths},
    outputs::{
        state::ShellInfo,
        wayland::{
            PanelEdge, destroy_layer_surface, draw_only, frame_settings, new_layer_surface,
            panel_anchor, panel_clearance, panel_settings
        }
    }
};

impl Outputs {
    /// Lines the frame of every real output up with the configuration.
    ///
    /// The fallback group — a bar's surfaces before the compositor reports
    /// a monitor — owns no frame and no panels, and rings itself in no
    /// numbers, so the pass reads real outputs and their assigned
    /// monitors only.
    pub(crate) fn reconcile_frame<Message: 'static>(
        &mut self,
        position: Position,
        config: &config::Config
    ) -> Task<Message> {
        let frame = &config.appearance.frame;
        let desired = panel_widths(frame, position, config.appearance.scale_factor);

        let mut tasks = Vec::new();

        for (_name, shell_info, output) in &mut self.0 {
            let (Some(shell_info), Some(output)) = (shell_info, output) else {
                continue;
            };

            tasks.push(Self::reconcile_ring(
                *output,
                shell_info,
                frame.enabled,
                config.layer
            ));
            tasks.push(Self::reconcile_reservations(
                shell_info,
                *output,
                desired,
                position,
                config,
                bar_height(config)
            ));
        }

        debug!("Frame reconciliation: {:?}", tasks.len());

        Task::batch(tasks)
    }

    /// Brings a ring surface into being or takes it away, per switch.
    fn reconcile_ring<Message: 'static>(
        output: OutputId,
        shell_info: &mut ShellInfo,
        desired: bool,
        layer: config::BarLayer
    ) -> Task<Message> {
        if shell_info.frame_enabled == desired {
            return Task::none();
        }

        shell_info.frame_enabled = desired;

        if desired {
            let (id, task) = new_layer_surface(frame_settings(Some(output), layer));
            shell_info.frame_id = Some(id);

            Task::batch(vec![task, draw_only(id)])
        } else if let Some(id) = shell_info.frame_id.take() {
            destroy_layer_surface(id)
        } else {
            Task::none()
        }
    }

    /// Resizes, creates and destroys the reservation panels until their
    /// widths state the configuration again.
    fn reconcile_reservations<Message: 'static>(
        shell_info: &mut ShellInfo,
        output: OutputId,
        desired: PanelWidths,
        position: Position,
        config: &config::Config,
        bar_height: f32
    ) -> Task<Message> {
        let fallback_bar_height = bar_height;

        let edges = [
            (PanelEdge::Top, desired.top, &mut shell_info.top_panel_id),
            (
                PanelEdge::Bottom,
                desired.bottom,
                &mut shell_info.bottom_panel_id
            ),
            (PanelEdge::Left, desired.left, &mut shell_info.left_panel_id),
            (
                PanelEdge::Right,
                desired.right,
                &mut shell_info.right_panel_id
            )
        ];

        debug!("Reservations change: {desired:?}");

        let mut tasks = Vec::new();

        for (edge, width, id) in edges {
            match (&mut *id, width) {
                (Some(surface), 0) => {
                    tasks.push(destroy_layer_surface(*surface));
                    *id = None;
                }
                (None, width) if width > 0 => {
                    let (surface, task) = new_layer_surface(panel_settings(
                        edge,
                        Some(output),
                        config.layer,
                        position,
                        fallback_bar_height,
                        width
                    ));
                    tasks.push(task);
                    *id = Some(surface);
                }
                (Some(surface), width) => {
                    let (anchor, size) = panel_anchor(edge, width);
                    let clearance = panel_clearance(position, edge, fallback_bar_height);

                    tasks.push(Task::batch(vec![
                        set_anchor(*surface, anchor),
                        set_size(*surface, size),
                        set_exclusive_zone(*surface, i32::try_from(width).unwrap_or(i32::MAX)),
                        set_margin(*surface, clearance),
                    ]));
                }
                (None, _) => {}
            }
        }

        shell_info.reservations = desired;

        Task::batch(tasks)
    }
}

/// The bar's height as the configuration names it, in pixels.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the bar height constant is exactly representable in f32"
)]
fn bar_height(config: &config::Config) -> f32 {
    config.appearance.height_px().unwrap_or(HEIGHT as f32)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced::SurfaceId as Id;

    use super::*;
    use crate::config::Config;

    fn config_with(frame_enabled: bool) -> Config {
        let mut config = Config::default();
        config.appearance.frame.enabled = frame_enabled;

        config
    }

    fn outputs_with<Message: 'static>(frame_enabled: bool) -> (Outputs, Option<Id>, Config) {
        let mut config = config_with(frame_enabled);
        config.outputs = crate::config::Outputs::Targets(vec!["DP-1".to_owned()]);

        let (outputs, _) =
            Outputs::new::<Message>(config.appearance.style, config.position, &config);

        let id = iced::OutputId::for_tests();

        let mut outputs = outputs;
        let task = outputs.add::<Message>(
            config.appearance.style,
            &config.outputs,
            config.position,
            "DP-1",
            id,
            &config,
            1.0,
            None
        );
        let _ = task;

        let specs = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("a requested output leaves a record")
            .1
            .as_ref()
            .expect("a requested output gains a group");
        let ring = specs.frame_id;

        (outputs, ring, config)
    }

    #[test]
    fn a_switched_on_frame_arrives_with_a_ring_and_reservations() {
        let (outputs, ring, _) = outputs_with::<()>(true);

        assert!(ring.is_some());

        let specs = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("the output keeps its record")
            .1
            .as_ref()
            .expect("the group stands");

        assert!(specs.frame_enabled);
        assert_eq!(specs.reservations.top, 0);
        assert_eq!(specs.reservations.left, 10);
        assert_eq!(specs.reservations.right, 10);
        assert_eq!(specs.reservations.bottom, 10);
        assert!(specs.left_panel_id.is_some());
        assert!(specs.right_panel_id.is_some());
        assert!(specs.bottom_panel_id.is_some());
        assert!(specs.top_panel_id.is_none());
    }

    #[test]
    fn a_switch_off_takes_the_ring_down_with_every_reservation() {
        let (mut outputs, ring, _config) = outputs_with::<()>(true);
        assert!(ring.is_some());

        let config = config_with(false);
        let task = outputs.sync::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            &config,
            1.0,
            None
        );
        let _ = task;

        let specs = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("the output keeps its record")
            .1
            .as_ref()
            .expect("the group stands");

        assert!(!specs.frame_enabled);
        assert!(specs.frame_id.is_none());
        assert_eq!(specs.reservations, PanelWidths::default());
        assert!(specs.left_panel_id.is_none());
        assert!(specs.right_panel_id.is_none());
        assert!(specs.bottom_panel_id.is_none());
        assert!(specs.top_panel_id.is_none());
    }

    #[test]
    fn a_switch_back_keeps_the_bar_edge_free_of_reservations() {
        let (mut outputs, _ring, _config) = outputs_with::<()>(true);

        let config = config_with(false);
        let task = outputs.sync::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            &config,
            1.0,
            None
        );
        let _ = task;

        let config = config_with(true);
        let task = outputs.sync::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            &config,
            1.0,
            None
        );
        let _ = task;

        let specs = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("the output keeps its record")
            .1
            .as_ref()
            .expect("the group stands");

        assert!(specs.frame_id.is_some());
        assert!(specs.left_panel_id.is_some());
        assert!(specs.right_panel_id.is_some());
        assert!(specs.bottom_panel_id.is_some());
        assert!(specs.top_panel_id.is_none());
    }

    #[test]
    fn an_unchanged_sync_turns_nothing_down() {
        let (mut outputs, ring, config) = outputs_with::<()>(true);

        let task = outputs.sync::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            &config,
            1.0,
            None
        );
        let _ = task;

        let specs = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("the output keeps its record")
            .1
            .as_ref()
            .expect("the group stands");

        assert_eq!(specs.frame_id, ring);
        assert!(specs.left_panel_id.is_some());
    }

    #[test]
    fn a_disabled_frame_from_the_start_nets_no_surfaces() {
        let (outputs, ring, _) = outputs_with::<()>(false);

        assert!(ring.is_none());

        let specs = outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("the output keeps its record")
            .1
            .as_ref()
            .expect("the group stands");

        assert!(!specs.frame_enabled);
        assert!(specs.left_panel_id.is_none());
        assert_eq!(specs.reservations, PanelWidths::default());
    }

    fn tracked_group(outputs: &mut Outputs, name: &str, config: &Config) {
        let task = outputs.add::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            name,
            iced::OutputId::for_tests(),
            config,
            1.0,
            None
        );
        let _ = task;
    }

    fn group_of<'a>(outputs: &'a Outputs, name: &str) -> &'a ShellInfo {
        outputs
            .0
            .iter()
            .find(|(entry, _, _)| entry.as_deref() == Some(name))
            .expect("the output keeps its record")
            .1
            .as_ref()
            .expect("the group stands")
    }

    #[test]
    fn two_outputs_own_two_rings_and_lose_both_together() {
        let mut config = config_with(true);
        config.outputs =
            crate::config::Outputs::Targets(vec!["DP-1".to_owned(), "HDMI-1".to_owned()]);

        let (mut outputs, _) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);

        tracked_group(&mut outputs, "DP-1", &config);
        tracked_group(&mut outputs, "HDMI-1", &config);

        // every output owns its surfaces: two rings, two sets of panels, no
        // shared identifiers
        let dp_ring = group_of(&outputs, "DP-1").frame_id;
        let hdmi_ring = group_of(&outputs, "HDMI-1").frame_id;

        assert!(dp_ring.is_some());
        assert!(hdmi_ring.is_some());
        assert_ne!(dp_ring, hdmi_ring);
        assert_ne!(
            group_of(&outputs, "DP-1").left_panel_id,
            group_of(&outputs, "HDMI-1").left_panel_id
        );

        let off = config_with(false);
        let task = outputs.sync::<()>(
            off.appearance.style,
            &config.outputs,
            off.position,
            &off,
            1.0,
            None
        );
        let _ = task;

        // one flip takes both rings down, group by group
        assert!(group_of(&outputs, "DP-1").frame_id.is_none());
        assert!(group_of(&outputs, "HDMI-1").frame_id.is_none());
        assert!(group_of(&outputs, "DP-1").left_panel_id.is_none());
        assert!(group_of(&outputs, "HDMI-1").left_panel_id.is_none());
    }

    #[test]
    fn a_sync_with_no_real_output_touches_nothing() {
        let config = config_with(true);
        let (mut outputs, _) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);

        // the fallback group owns no frame and no panels: the pass reads
        // real outputs only and leaves the fallback exactly as it found it
        let task = outputs.sync::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            &config,
            1.0,
            None
        );
        let _ = task;

        assert_eq!(outputs.0.len(), 1);

        let (_, info, _) = &outputs.0[0];
        let specs = info.as_ref().expect("the fallback stands");

        assert!(!specs.frame_enabled);
        assert!(specs.frame_id.is_none());
    }

    #[test]
    fn a_ring_lost_outside_the_sync_is_not_destroyed_twice() {
        let (mut outputs, _ring, _) = outputs_with::<()>(true);

        // the switch still reads on while the surface is already gone: the
        // sync takes the record down without stating a second destruction
        for (_, info, _) in &mut outputs.0 {
            if let Some(specs) = info {
                specs.frame_id = None;
            }
        }

        let mut off = config_with(false);
        off.outputs = crate::config::Outputs::Targets(vec!["DP-1".to_owned()]);
        let task = outputs.sync::<()>(
            off.appearance.style,
            &off.outputs,
            off.position,
            &off,
            1.0,
            None
        );
        let _ = task;

        let specs = group_of(&outputs, "DP-1");

        assert!(!specs.frame_enabled);
        assert!(specs.frame_id.is_none());
    }

    #[test]
    fn every_surface_answers_to_its_own_output() {
        let (outputs, _ring, _) = outputs_with::<()>(true);
        let specs = group_of(&outputs, "DP-1");

        // each surface of the group resolves back to the owning output, and
        // each panel names its own edge
        assert_eq!(outputs.screen_of(specs.id), Some(Some("DP-1")));
        assert_eq!(
            outputs.screen_of(specs.frame_id.expect("the ring stands")),
            Some(Some("DP-1"))
        );
        assert_eq!(
            outputs.screen_of(specs.left_panel_id.expect("the strip stands")),
            Some(Some("DP-1"))
        );
        assert_eq!(
            outputs.panel_edge_of(specs.left_panel_id.expect("the strip stands")),
            Some(PanelEdge::Left)
        );
        assert_eq!(
            outputs.panel_edge_of(specs.bottom_panel_id.expect("the strip stands")),
            Some(PanelEdge::Bottom)
        );
        assert_eq!(outputs.output_name(iced::OutputId::for_tests()), None);
        assert!(outputs.screen_of(iced::SurfaceId::unique()).is_none());
    }
}
