//! Lookups by surface id and monitor name.

use iced::{OutputId, SurfaceId as Id};

use super::{HasOutput, Outputs};
use crate::outputs::wayland::PanelEdge;

impl Outputs {
    /// Attempt to resolve a window [`Id`] to a tracked output or menu surface.
    ///
    /// Returns [`None`] when the identifier does not belong to the bar.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// # use iced::SurfaceId as Id;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// let unknown = Id::unique();
    /// assert!(outputs.has(unknown).is_none());
    /// ```
    #[must_use]
    pub fn has(&self, id: Id) -> Option<HasOutput<'_>> {
        self.0.iter().find_map(|(_, info, _)| {
            info.as_ref().and_then(|info| {
                if info.id == id {
                    Some(HasOutput::Main)
                } else if info.menu.id == id {
                    Some(HasOutput::Menu(info.menu.menu_info.as_ref()))
                } else if info.notifications_id == id {
                    Some(HasOutput::Notifications)
                } else if info.desk_id == id {
                    Some(HasOutput::Desk)
                } else if info.frame_id == Some(id) {
                    Some(HasOutput::Frame)
                } else if info.left_panel_id == Some(id)
                    || info.right_panel_id == Some(id)
                    || info.bottom_panel_id == Some(id)
                    || info.top_panel_id == Some(id)
                {
                    Some(HasOutput::Panel)
                } else if info.tooltip_id == id {
                    Some(HasOutput::Tooltip)
                } else {
                    None
                }
            })
        })
    }

    /// Retrieve the monitor name associated with a given surface identifier.
    ///
    /// Returns [`None`] when the identifier does not belong to a tracked output
    /// or the output has no reported name yet.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// # use iced::SurfaceId as Id;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(outputs.monitor_name(Id::unique()).is_none());
    /// ```
    #[must_use]
    pub fn monitor_name(&self, id: Id) -> Option<&str> {
        self.0.iter().find_map(|(name, info, _)| {
            info.as_ref()
                .and_then(|info| if info.id == id { name.as_deref() } else { None })
        })
    }

    /// Retrieve the monitor name behind a Wayland output identifier.
    ///
    /// Removal events name the output, not one of its surfaces: the caller
    /// asks what record the departing output owned before it takes the
    /// surfaces down. Returns [`None`] when no tracked output answers to
    /// the identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// # use iced::OutputId;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(outputs.output_name(OutputId::for_tests()).is_none());
    /// ```
    #[must_use]
    pub fn output_name(&self, id: OutputId) -> Option<&str> {
        self.0.iter().find_map(|(name, _, output)| {
            (*output == Some(id)).then_some(name.as_deref()).flatten()
        })
    }

    /// Retrieve the monitor name behind any surface of a tracked output.
    ///
    /// Unlike [`Outputs::monitor_name`], which answers for the bar surface
    /// alone, this one answers for every surface of the group — the desk asks
    /// what screen it is standing on, and it is not the bar surface asking.
    ///
    /// Returns [`None`] when the identifier does not belong to the bar, and
    /// [`Some(None)`] for the fallback group the bar runs on before the
    /// compositor reports its monitors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// # use iced::SurfaceId as Id;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(outputs.screen_of(Id::unique()).is_none());
    /// ```
    #[must_use]
    pub fn screen_of(&self, id: Id) -> Option<Option<&str>> {
        self.0.iter().find_map(|(name, info, _)| {
            info.as_ref()
                .filter(|info| info.owns(id) || info.notifications_id == id)
                .map(|_| name.as_deref())
        })
    }

    /// Every screen the bar currently draws on, the fallback one included.
    ///
    /// The fallback group reads back as [`None`]: it is the surface the bar
    /// runs on before the compositor reports its monitors, and it stands for
    /// whichever screen holds the focus.
    pub fn screens(&self) -> impl Iterator<Item = Option<&str>> {
        self.0
            .iter()
            .filter(|(_, info, _)| info.is_some())
            .map(|(name, _, _)| name.as_deref())
    }

    /// Every desk surface the bar owns, with the screen it stands on.
    ///
    /// The canvas is the one surface whose pointer input comes and goes: it
    /// covers the whole screen and must swallow nothing while it is folded
    /// away, so the caller states its region again on every fold.
    pub fn desk_surfaces(&self) -> impl Iterator<Item = (Id, Option<&str>)> {
        self.0.iter().filter_map(|(name, info, _)| {
            info.as_ref().map(|info| (info.desk_id, name.as_deref()))
        })
    }

    /// Every ring surface the bar owns, with the screen it stands on.
    ///
    /// The ring covers the whole screen and must swallow nothing, so the
    /// caller states its empty region again once the surface is mapped: the
    /// region stated together with the creation is dropped while the surface
    /// does not exist yet.
    pub fn frame_surfaces(&self) -> impl Iterator<Item = (Id, Option<&str>)> {
        self.0.iter().filter_map(|(name, info, _)| {
            let name = name.as_deref();

            info.as_ref()
                .and_then(|info| info.frame_id.map(|frame_id| (frame_id, name)))
        })
    }

    /// Which edge of the frame the panel surface `id` stands on.
    ///
    /// A panel is named by its edge because that is everything the caller
    /// needs to paint or place a widget on it; the strip's own geometry
    /// follows from the screen the edge runs along.
    #[must_use]
    pub fn panel_edge_of(&self, id: Id) -> Option<PanelEdge> {
        self.0.iter().find_map(|(_, info, _)| {
            info.as_ref().and_then(|info| {
                if info.top_panel_id == Some(id) {
                    Some(PanelEdge::Top)
                } else if info.left_panel_id == Some(id) {
                    Some(PanelEdge::Left)
                } else if info.right_panel_id == Some(id) {
                    Some(PanelEdge::Right)
                } else if info.bottom_panel_id == Some(id) {
                    Some(PanelEdge::Bottom)
                } else {
                    None
                }
            })
        })
    }

    /// Every side panel the bar owns, with the screen it stands on.
    ///
    /// The panels are real strips the user's desktop is pushed off of, so
    /// their identifiers come back through [`Outputs::screen_of`] whenever
    /// the caller has to know which screen a repaint belongs to.
    pub fn panel_surfaces(&self) -> impl Iterator<Item = (Id, Option<&str>)> {
        self.0.iter().flat_map(|(name, info, _)| {
            let name = name.as_deref();

            info.as_ref().into_iter().flat_map(move |info| {
                [
                    (info.top_panel_id, name),
                    (info.left_panel_id, name),
                    (info.right_panel_id, name),
                    (info.bottom_panel_id, name)
                ]
                .into_iter()
                .filter_map(|(id, name)| id.map(|id| (id, name)))
            })
        })
    }

    /// Check whether an output with the provided name is already tracked.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(!outputs.has_name("DP-1"));
    /// ```
    #[must_use]
    pub fn has_name(&self, name: &str) -> bool {
        self.0
            .iter()
            .any(|(n, info, _)| info.is_some() && n.as_deref() == Some(name))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::config::Config;

    fn tracked() -> Outputs {
        let mut config = Config::default();
        config.appearance.frame.enabled = true;
        config.outputs = crate::config::Outputs::Targets(vec!["DP-1".to_owned()]);

        let (mut outputs, _) =
            Outputs::new::<()>(config.appearance.style, config.position, &config);
        let task = outputs.add::<()>(
            config.appearance.style,
            &config.outputs,
            config.position,
            "DP-1",
            iced::OutputId::for_tests(),
            &config,
            1.0,
            None
        );
        let _ = task;

        outputs
    }

    fn group(outputs: &Outputs) -> &super::super::ShellInfo {
        outputs
            .0
            .iter()
            .find(|(name, _, _)| name.as_deref() == Some("DP-1"))
            .expect("the output keeps its record")
            .1
            .as_ref()
            .expect("the group stands")
    }

    #[test]
    fn every_surface_of_the_group_resolves() {
        let outputs = tracked();
        let specs = group(&outputs);

        assert_eq!(outputs.has(specs.id), Some(HasOutput::Main));
        assert!(matches!(
            outputs.has(specs.menu.id),
            Some(HasOutput::Menu(_))
        ));
        assert_eq!(outputs.has(specs.tooltip_id), Some(HasOutput::Tooltip));
        assert_eq!(outputs.has(specs.desk_id), Some(HasOutput::Desk));
        assert_eq!(
            outputs.has(specs.frame_id.expect("the ring stands")),
            Some(HasOutput::Frame)
        );
        assert_eq!(
            outputs.has(specs.left_panel_id.expect("the strip stands")),
            Some(HasOutput::Panel)
        );
        assert_eq!(
            outputs.has(specs.notifications_id),
            Some(HasOutput::Notifications)
        );
        assert!(outputs.has(Id::unique()).is_none());
    }

    #[test]
    fn every_surface_reports_its_screen() {
        let outputs = tracked();
        let specs = group(&outputs);

        assert_eq!(outputs.monitor_name(specs.id), Some("DP-1"));
        assert!(outputs.monitor_name(Id::unique()).is_none());

        for id in [
            specs.id,
            specs.tooltip_id,
            specs.desk_id,
            specs.frame_id.expect("the ring stands"),
            specs.left_panel_id.expect("the strip stands"),
            specs.notifications_id
        ] {
            assert_eq!(outputs.screen_of(id), Some(Some("DP-1")));
        }

        assert!(outputs.screen_of(Id::unique()).is_none());
    }

    #[test]
    fn every_group_lists_its_surfaces() {
        let outputs = tracked();

        assert!(outputs.has_name("DP-1"));
        assert!(!outputs.has_name("HDMI-1"));
        assert_eq!(outputs.screens().collect::<Vec<_>>(), vec![Some("DP-1")]);
        assert_eq!(outputs.desk_surfaces().count(), 1);
        assert_eq!(outputs.frame_surfaces().count(), 1);
        assert_eq!(outputs.panel_surfaces().count(), 3);

        assert_eq!(
            outputs.panel_edge_of(
                group(&outputs).top_panel_id.unwrap_or_else(|| {
                    group(&outputs).left_panel_id.expect("the strip stands")
                })
            ),
            Some(PanelEdge::Left)
        );
        assert!(outputs.panel_edge_of(Id::unique()).is_none());
    }
}
