//! Message folding and the label's dissolve for the keyboard layout
//! indicator.

use std::time::Duration;

use log::error;

use super::{KeyboardLayout, Message};
use crate::config::KeyboardLayoutModuleConfig;

impl KeyboardLayout {
    /// `animated` decides whether the shown label dissolves into its
    /// replacement or swaps outright.
    pub fn update(
        &mut self,
        message: Message,
        config: &KeyboardLayoutModuleConfig,
        animated: bool
    ) {
        match message {
            Message::ActiveLayoutChanged(layout) => {
                self.active = layout;
            }
            Message::LayoutConfigChanged(layout_flag) => self.multiple_layout = layout_flag,
            Message::ChangeLayout => {
                if let Err(err) = self.hyprland.switch_keyboard_layout() {
                    error!("failed to switch keyboard layout: {err}");
                }
            }
        }

        let label = match config.labels.get(&self.active) {
            Some(value) => value.clone(),
            None => self.active.clone()
        };
        self.shown.set(label, animated);
    }

    /// Advances the dissolve of the shown label.
    pub fn tick_fade(&mut self, elapsed: Duration) -> bool {
        self.shown.advance(elapsed)
    }

    /// Whether the shown label is still dissolving.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.shown.is_animating()
    }

    /// Layout currently in force, as the compositor names it.
    #[must_use]
    pub fn active_layout(&self) -> &str {
        &self.active
    }

    #[cfg(test)]
    pub(crate) const fn has_multiple_layouts(&self) -> bool {
        self.multiple_layout
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::{KeyboardLayout, Message};
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn change_layout_invokes_port_command() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let mut module = KeyboardLayout::new(port_trait);

        module.update(
            Message::ChangeLayout,
            &hydebar_proto::config::KeyboardLayoutModuleConfig::default(),
            false
        );

        assert_eq!(port.switch_layout_calls(), 1);
    }
}
