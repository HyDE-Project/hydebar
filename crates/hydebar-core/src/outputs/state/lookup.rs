//! Lookups by surface id and monitor name.

use iced::SurfaceId as Id;

use super::{HasOutput, Outputs};

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
    /// assert!(outputs.get_monitor_name(Id::unique()).is_none());
    /// ```
    #[must_use]
    pub fn get_monitor_name(&self, id: Id) -> Option<&str> {
        self.0.iter().find_map(|(name, info, _)| {
            info.as_ref()
                .and_then(|info| if info.id == id { name.as_deref() } else { None })
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
