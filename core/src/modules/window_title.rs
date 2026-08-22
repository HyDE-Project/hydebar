//! The focused window's title, drawn in the bar.
//!
//! One folder, three rooms: [`state`] decides what the bar shows and folds
//! title changes in, [`listener`] follows the compositor's window events in
//! the background and [`module`] wires the module to the bar. The root holds
//! the state the rooms share.

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{HyprlandPort, HyprlandWindowInfo};
use tokio::task::JoinHandle;

use crate::{ModuleEventSender, config::WindowTitleConfig, utils::truncate_text};

mod listener;
mod module;
mod state;

/// Title of the focused window, as the compositor reports it.
///
/// The whole title is kept rather than the shortened one the bar draws: a
/// module the user is looking at shows what it has in full, and a title
/// shortened on the way in could never be restored.
pub struct WindowTitle {
    hyprland:  Arc<dyn HyprlandPort>,
    value:     Option<String>,
    /// The shortened spelling, cut once per focus change.
    ///
    /// Cut in the update rather than per frame: the title moves on focus
    /// events, the bar repaints far more often than that.
    shortened: Option<String>,
    sender:    Option<ModuleEventSender<Message>>,
    task:      Option<JoinHandle<()>>
}

impl std::fmt::Debug for WindowTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowTitle")
            .field("hyprland", &"<HyprlandPort>")
            .field("shortened", &self.shortened)
            .field("value", &self.value)
            .field("sender", &self.sender)
            .field("task", &self.task)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(Option<HyprlandWindowInfo>)
}

impl WindowTitle {
    /// The whole title of the focused window, as the client set it.
    ///
    /// Not the shortened spelling the strip draws: the canvas has a column
    /// to write in and a title cut to fit a bar entry says less than nothing
    /// there.
    #[must_use]
    pub fn full(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn new(hyprland: Arc<dyn HyprlandPort>, config: &WindowTitleConfig) -> Self {
        let init = state::get_window(hyprland.as_ref(), config);
        let shortened = init
            .as_deref()
            .map(|value| truncate_text(value, config.truncate_title_after_length));

        Self {
            hyprland,
            value: init,
            shortened,
            sender: None,
            task: None
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_proto::config::{WindowTitleConfig, WindowTitleMode};

    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_title_from_port() {
        let port = Arc::new(MockHyprlandPort::with_active_window("Demo", "Class"));
        let port_trait: Arc<dyn HyprlandPort> = port;
        let config = WindowTitleConfig {
            mode: WindowTitleMode::Title,
            ..Default::default()
        };

        let module = WindowTitle::new(port_trait, &config);

        assert_eq!(module.current_value(), Some("Demo"));
    }
}
