//! Lookups by surface id and monitor name.

use iced::window::Id;

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
    /// # use iced::window::Id;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// let unknown = Id::unique();
    /// assert!(outputs.has(unknown).is_none());
    /// ```
    pub fn has(&self, id: Id) -> Option<HasOutput<'_>> {
        self.0.iter().find_map(|(_, info, _)| {
            if let Some(info) = info {
                if info.id == id {
                    Some(HasOutput::Main)
                } else if info.menu.id == id {
                    Some(HasOutput::Menu(info.menu.menu_info.as_ref()))
                } else if info.tooltip_id == id {
                    Some(HasOutput::Tooltip)
                } else {
                    None
                }
            } else {
                None
            }
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
    /// # use iced::window::Id;
    /// let config = Config::default();
    /// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(outputs.get_monitor_name(Id::unique()).is_none());
    /// ```
    pub fn get_monitor_name(&self, id: Id) -> Option<&str> {
        self.0.iter().find_map(|(name, info, _)| {
            if let Some(info) = info {
                if info.id == id { name.as_deref() } else { None }
            } else {
                None
            }
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
    pub fn has_name(&self, name: &str) -> bool {
        self.0
            .iter()
            .any(|(n, info, _)| info.is_some() && n.as_deref() == Some(name))
    }
}
