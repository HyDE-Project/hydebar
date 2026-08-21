//! Configuration of the desk: the canvas the bar unfolds into on an empty
//! workspace.
//!
//! The bar stays a strip for as long as a window is mapped on the screen it
//! stands on. The moment the workspace is cleared it has the whole wallpaper
//! to itself, and the readouts that do not fit a strip — the machine, the
//! link, the mounts — are drawn there instead of being hidden behind a menu.

use serde::Deserialize;

/// One block of readouts drawn on the desk.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeskPanel {
    /// Kernel, processor model and the firmware steering it.
    System,
    /// Address of the link and what is crossing it.
    Network,
    /// Load, temperature and clock of the processor.
    Processor,
    /// Load, temperature and memory of the graphics device.
    Graphics,
    /// Memory and swap in use, against what is installed.
    Memory,
    /// Every mounted filesystem, with what is left on it.
    Storage,
    /// The hour, large, with the date under it.
    Clock,
    /// The sky over the configured location.
    Weather
}

/// Where the desk draws its panels.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct DeskConfig {
    /// Whether the bar unfolds at all.
    pub enabled: bool,
    /// Panels drawn down the left edge.
    pub left:    Vec<DeskPanel>,
    /// Panels drawn down the middle of the screen.
    pub center:  Vec<DeskPanel>,
    /// Panels drawn down the right edge.
    pub right:   Vec<DeskPanel>
}

impl DeskConfig {
    /// Every panel the desk draws, in no particular order.
    pub fn panels(&self) -> impl Iterator<Item = DeskPanel> + '_ {
        self.left
            .iter()
            .chain(self.center.iter())
            .chain(self.right.iter())
            .copied()
    }

    /// Reports whether the desk draws `panel` anywhere.
    #[must_use]
    pub fn draws(&self, panel: DeskPanel) -> bool {
        self.enabled && self.panels().any(|drawn| drawn == panel)
    }

    /// Reports whether any panel of the desk renders a system sample.
    ///
    /// The sampler behind those readouts is otherwise started only for the
    /// bar entries that show them; a desk drawing the machine has to keep it
    /// running on its own account.
    #[must_use]
    pub fn wants_system_sample(&self) -> bool {
        self.enabled
            && self.panels().any(|panel| {
                matches!(
                    panel,
                    DeskPanel::System
                        | DeskPanel::Network
                        | DeskPanel::Processor
                        | DeskPanel::Graphics
                        | DeskPanel::Memory
                        | DeskPanel::Storage
                )
            })
    }
}

impl Default for DeskConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            left:    vec![DeskPanel::System, DeskPanel::Network],
            center:  vec![DeskPanel::Clock, DeskPanel::Weather],
            right:   vec![
                DeskPanel::Processor,
                DeskPanel::Graphics,
                DeskPanel::Memory,
                DeskPanel::Storage,
            ]
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_desk_stays_folded_until_it_is_asked_for() {
        let config = DeskConfig::default();

        assert!(!config.enabled);
        assert!(!config.draws(DeskPanel::Clock));
        assert!(!config.wants_system_sample());
    }

    #[test]
    fn the_stock_desk_reads_like_the_screenshot_it_replaces() {
        let config = DeskConfig {
            enabled: true,
            ..DeskConfig::default()
        };

        assert!(config.draws(DeskPanel::System));
        assert!(config.draws(DeskPanel::Storage));
        assert!(config.wants_system_sample());
    }

    #[test]
    fn a_desk_of_hours_alone_leaves_the_sampler_asleep() {
        let config: DeskConfig = toml::from_str(
            r#"
            enabled = true
            left = []
            center = ["clock"]
            right = []
            "#
        )
        .expect("desk config");

        assert!(config.draws(DeskPanel::Clock));
        assert!(!config.wants_system_sample());
    }

    #[test]
    fn the_panels_of_every_column_are_read_as_one_roster() {
        let config = DeskConfig {
            enabled: true,
            ..DeskConfig::default()
        };

        assert_eq!(config.panels().count(), 8);
    }
}
