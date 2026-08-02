//! The `HyDE` overlay on a freshly parsed configuration.
//!
//! The bar lives inside the `HyDE` Project's desktop, whose theme and bar
//! layout are on file outside the bar's own configuration. This module fills
//! what the user left unset from what the desktop has on file, and nothing
//! more — the precedence it fixes is: the user's file wins, `HyDE` fills the
//! rest, the bar's defaults answer for whatever is left.

use hydebar_proto::{bar_layout, config::Config, theme_source::HydeTheme};

/// Overlays the `HyDE` theme and bar layout onto a freshly parsed
/// configuration.
///
/// The overlay runs *after* the file has been parsed and only fills what the
/// user left unset, which is what fixes the precedence for good: what is
/// written in `~/.config/hydebar/config.toml` wins, what `HyDE` says fills the
/// rest, and the bar's own defaults answer for whatever is left. No theme or
/// layout switch can undo a value the user wrote.
///
/// Runs on every read, so a hot reload picks up a switch that happened while
/// the bar was running. Opting out with `appearance.follow_hyde = false`
/// skips reading `HyDE` entirely.
pub(super) fn follow_hyde<F, G>(
    mut config: Config,
    manual_layout: bool,
    theme: F,
    layout: G
) -> Config
where
    F: FnOnce() -> HydeTheme,
    G: FnOnce(&[String]) -> Option<bar_layout::RestatedLayout>
{
    if !config.appearance.follow_hyde {
        return config;
    }

    config.appearance.apply_hyde_theme(&theme());

    if !manual_layout {
        let custom_names: Vec<String> = config
            .custom_modules
            .iter()
            .map(|definition| definition.name.clone())
            .collect();

        if let Some(restated) = layout(&custom_names) {
            config.modules = restated.modules;

            for definition in restated.synthesized {
                if !config
                    .custom_modules
                    .iter()
                    .any(|existing| existing.name == definition.name)
                {
                    config.custom_modules.push(definition);
                }
            }
        }
    }

    config
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::fs;

    use hydebar_proto::{
        bar_layout,
        config::{ModuleDef, ModuleName, Modules},
        theme_source::HydeTheme
    };
    use tempfile::TempDir;

    use crate::config::read_config_with;

    #[expect(
        clippy::unnecessary_wraps,
        reason = "the helper matches the layout callback signature"
    )]
    fn hyde_layout(_custom_names: &[String]) -> Option<bar_layout::RestatedLayout> {
        Some(bar_layout::RestatedLayout {
            modules:     Modules {
                left:   vec![ModuleDef::Single(ModuleName::Clock)],
                center: Vec::new(),
                right:  Vec::new()
            },
            synthesized: Vec::new()
        })
    }

    /// A configuration that writes no module layout takes the one `HyDE` has
    /// on file, exactly as the bar `HyDE` started with would.
    #[test]
    fn an_undeclared_layout_follows_the_desktop() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "position = \"Top\"\n").expect("config");

        let config =
            read_config_with(&config_path, HydeTheme::default, hyde_layout).expect("config reads");

        assert_eq!(
            config.modules.left,
            vec![ModuleDef::Single(ModuleName::Clock)]
        );
        assert!(config.modules.center.is_empty());
    }

    /// A hand-written `[modules]` section is manual control and no desktop
    /// layout may displace it.
    #[test]
    fn a_declared_layout_outranks_the_desktop() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "[modules]\nleft = [\"Battery\"]\n").expect("config");

        let config =
            read_config_with(&config_path, HydeTheme::default, hyde_layout).expect("config reads");

        assert_eq!(
            config.modules.left,
            vec![ModuleDef::Single(ModuleName::Battery)]
        );
    }

    /// Opting out of following `HyDE` opts out of its layout with it.
    #[test]
    fn opting_out_of_hyde_keeps_the_default_layout() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "[appearance]\nfollow_hyde = false\n").expect("config");

        let config =
            read_config_with(&config_path, HydeTheme::default, hyde_layout).expect("config reads");

        assert_eq!(config.modules, Modules::default());
    }

    /// A desktop with no readable layout leaves the default in place.
    #[test]
    fn a_missing_desktop_layout_leaves_the_default() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "position = \"Top\"\n").expect("config");

        let config =
            read_config_with(&config_path, HydeTheme::default, |_| None).expect("config reads");

        assert_eq!(config.modules, Modules::default());
    }
}
