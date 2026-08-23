//! What the bar shows for the focused window, and how changes fold in.

use hydebar_proto::ports::hyprland::{HyprlandPort, HyprlandWindowInfo};
use log::error;

use super::{Message, WindowTitle};
use crate::{
    config::{WindowTitleConfig, WindowTitleMode},
    utils::truncate_text
};

/// Asks the port for the focused window, `None` on failure.
pub(super) fn get_window(port: &dyn HyprlandPort, config: &WindowTitleConfig) -> Option<String> {
    match port.active_window() {
        Ok(window) => window.map(|window| shown_field(window, config)),
        Err(err) => {
            error!("failed to retrieve active window: {err}");
            None
        }
    }
}

/// Field of the focused window the configured mode shows.
fn shown_field(window: HyprlandWindowInfo, config: &WindowTitleConfig) -> String {
    match config.mode {
        WindowTitleMode::Title => window.title,
        WindowTitleMode::Class => window.class
    }
}

/// The title as the bar draws it.
///
/// Whole while the module is attended, shortened to the configured length
/// otherwise: a title that had to be cut is the one the user leans in to read,
/// so looking at the module is taken as asking for the rest of it.
pub(super) fn shown_title(value: &str, config: &WindowTitleConfig, attended: bool) -> String {
    if attended {
        value.to_owned()
    } else {
        truncate_text(value, config.truncate_title_after_length)
    }
}

impl WindowTitle {
    /// Folds one message into the entry.
    pub fn update(&mut self, message: Message, config: &WindowTitleConfig) {
        match message {
            Message::TitleChanged(window) => {
                self.value = window.map(|window| shown_field(window, config));
                self.shortened = self
                    .value
                    .as_deref()
                    .map(|value| truncate_text(value, config.truncate_title_after_length));
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn current_value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::{
        config::WindowTitleConfig,
        ports::hyprland::{HyprlandPort, HyprlandWindowInfo}
    };

    use super::{Message, WindowTitle, shown_title};
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn update_handles_absent_window() {
        let port = Arc::new(MockHyprlandPort::default());
        *port
            .active_window
            .lock()
            .expect("active window lock poisoned") = None;
        let port_trait: Arc<dyn HyprlandPort> = port;
        let config = WindowTitleConfig::default();

        let mut module = WindowTitle::new(port_trait, &config);
        module.update(Message::TitleChanged(None), &config);

        assert_eq!(module.current_value(), None);
    }

    #[test]
    fn a_long_title_is_shortened_until_the_module_is_attended() {
        let config = WindowTitleConfig {
            truncate_title_after_length: 10,
            ..Default::default()
        };
        let title = "a window with a very long title indeed";

        assert_ne!(shown_title(title, &config, false), title);
        assert_eq!(shown_title(title, &config, true), title);
    }

    #[test]
    fn a_short_title_reads_the_same_either_way() {
        let config = WindowTitleConfig {
            truncate_title_after_length: 150,
            ..Default::default()
        };

        assert_eq!(shown_title("short", &config, false), "short");
        assert_eq!(shown_title("short", &config, true), "short");
    }

    #[test]
    fn the_whole_title_survives_the_update() {
        let port = Arc::new(MockHyprlandPort::with_active_window(
            "a window with a very long title indeed",
            "Class"
        ));
        let port_trait: Arc<dyn HyprlandPort> = port;
        let config = WindowTitleConfig {
            truncate_title_after_length: 10,
            ..Default::default()
        };

        let mut module = WindowTitle::new(port_trait, &config);
        module.update(
            Message::TitleChanged(Some(HyprlandWindowInfo {
                title: "a window with a very long title indeed".to_owned(),
                class: "Class".to_owned()
            })),
            &config
        );

        assert_eq!(
            module.current_value(),
            Some("a window with a very long title indeed"),
            "a title shortened on the way in could never be shown in full"
        );
    }
}
