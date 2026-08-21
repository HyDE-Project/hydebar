//! Configuration of the desk: the bar's other form.
//!
//! The bar and the desk are one thing in two shapes. While a window is mapped
//! on a screen the bar is a strip along its edge; the moment the workspace is
//! cleared the very same modules, in the very same layout, come down off the
//! strip and stand over the wallpaper at a size the whole room can read. No
//! second set of readouts and no second arrangement: whatever the layout says
//! is on the bar is what the desk unfolds into.

use serde::Deserialize;

/// How the desk unfolds.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct DeskConfig {
    /// Whether the bar unfolds at all.
    pub enabled: bool
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_desk_stays_folded_until_it_is_asked_for() {
        assert!(!DeskConfig::default().enabled);
    }

    #[test]
    fn the_desk_is_asked_for_by_one_key() {
        let config: DeskConfig = toml::from_str("enabled = true").expect("desk config");

        assert!(config.enabled);
    }
}
