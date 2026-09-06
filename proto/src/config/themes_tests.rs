#![allow(clippy::float_cmp)]
use hex_color::HexColor;

use super::themes::{preset_appearance, preset_names};
use crate::config::{Appearance, AppearanceColor, AppearanceStyle};

/// Background, primary and text shades plus the workspace palette length of
/// a shipped preset, pinned so a transcription slip in a theme file fails
/// loudly.
struct PaletteCheck {
    name:       &'static str,
    background: (u8, u8, u8),
    primary:    (u8, u8, u8),
    text:       (u8, u8, u8),
    workspaces: usize
}

/// Spot-checks of every shipped preset.
const PALETTE_CHECKS: &[PaletteCheck] = &[
    PaletteCheck {
        name:       "catppuccin-mocha",
        background: (30, 30, 46),
        primary:    (203, 166, 247),
        text:       (205, 214, 244),
        workspaces: 10
    },
    PaletteCheck {
        name:       "catppuccin-macchiato",
        background: (36, 39, 58),
        primary:    (198, 160, 246),
        text:       (202, 211, 245),
        workspaces: 10
    },
    PaletteCheck {
        name:       "catppuccin-frappe",
        background: (48, 52, 70),
        primary:    (202, 158, 230),
        text:       (198, 208, 245),
        workspaces: 10
    },
    PaletteCheck {
        name:       "catppuccin-latte",
        background: (239, 241, 245),
        primary:    (136, 57, 239),
        text:       (76, 79, 105),
        workspaces: 10
    },
    PaletteCheck {
        name:       "dracula",
        background: (40, 42, 54),
        primary:    (189, 147, 249),
        text:       (248, 248, 242),
        workspaces: 6
    },
    PaletteCheck {
        name:       "nord",
        background: (46, 52, 64),
        primary:    (136, 192, 208),
        text:       (236, 239, 244),
        workspaces: 6
    },
    PaletteCheck {
        name:       "gruvbox-dark",
        background: (40, 40, 40),
        primary:    (211, 134, 155),
        text:       (235, 219, 178),
        workspaces: 6
    },
    PaletteCheck {
        name:       "gruvbox-light",
        background: (251, 241, 199),
        primary:    (157, 0, 6),
        text:       (60, 56, 54),
        workspaces: 6
    },
    PaletteCheck {
        name:       "tokyo-night",
        background: (26, 27, 38),
        primary:    (187, 154, 247),
        text:       (192, 202, 245),
        workspaces: 8
    },
    PaletteCheck {
        name:       "tokyo-night-storm",
        background: (36, 40, 59),
        primary:    (187, 154, 247),
        text:       (166, 173, 200),
        workspaces: 8
    },
    PaletteCheck {
        name:       "tokyo-night-light",
        background: (213, 214, 219),
        primary:    (121, 94, 172),
        text:       (60, 62, 73),
        workspaces: 8
    }
];

#[test]
fn eleven_presets_ship_with_the_bar() {
    let names = preset_names();

    assert_eq!(names.len(), PALETTE_CHECKS.len());
    assert!(names.contains(&"dracula"));
    assert!(names.contains(&"tokyo-night-storm"));
}

#[test]
fn every_preset_matches_its_published_palette() {
    for check in PALETTE_CHECKS {
        let appearance = preset_appearance(check.name)
            .unwrap_or_else(|| panic!("shipped theme '{}'", check.name));

        let shade = |shade: (u8, u8, u8)| {
            AppearanceColor::Simple(HexColor::rgb(shade.0, shade.1, shade.2))
        };

        assert_eq!(
            appearance.background_color,
            shade(check.background),
            "{}",
            check.name
        );
        assert_eq!(
            appearance.primary_color,
            shade(check.primary),
            "{}",
            check.name
        );
        assert_eq!(appearance.text_color, shade(check.text), "{}", check.name);
        assert_eq!(
            appearance.workspace_colors.len(),
            check.workspaces,
            "{}",
            check.name
        );
        assert_eq!(
            appearance
                .special_workspace_colors
                .as_ref()
                .map_or(0, Vec::len),
            1,
            "{}",
            check.name
        );
    }
}

#[test]
fn every_preset_shares_the_reference_shell() {
    for name in preset_names() {
        let appearance =
            preset_appearance(name).unwrap_or_else(|| panic!("shipped theme '{name}'"));

        assert_eq!(appearance.style, AppearanceStyle::Islands, "{name}");
        assert_eq!(appearance.opacity, 0.95, "{name}");
        assert_eq!(appearance.bar_opacity, 0.0, "{name}");
        assert!(appearance.greeting, "{name}");
        assert!(appearance.follow_hyde, "{name}");
        assert!(!appearance.auto_scale, "{name}");
        assert_eq!(appearance.scale_factor, 1.0, "{name}");
        assert!(!appearance.island_borders, "{name}");
        assert_eq!(appearance.menu.opacity, 0.95, "{name}");
        assert_eq!(appearance.menu.backdrop, 0.3, "{name}");
        assert!(appearance.animations.enabled, "{name}");
        assert_eq!(appearance.animations.menu_fade_duration_ms, 200, "{name}");
        assert_eq!(appearance.animations.hover_duration_ms, 100, "{name}");
    }
}

#[test]
fn deserialize_preset_theme_from_string() {
    #[derive(serde::Deserialize)]
    struct TestConfig {
        #[serde(deserialize_with = "super::themes::deserialize_theme_or_appearance")]
        appearance: Appearance
    }

    let toml_content = r#"
        appearance = "catppuccin-mocha"
    "#;

    let config: TestConfig = ::toml::from_str(toml_content).expect("Failed to deserialize");
    assert_eq!(
        config.appearance.background_color,
        AppearanceColor::Simple(HexColor::rgb(30, 30, 46))
    );
}

#[test]
fn deserialize_custom_appearance() {
    #[derive(serde::Deserialize)]
    struct TestConfig {
        #[serde(deserialize_with = "super::themes::deserialize_theme_or_appearance")]
        appearance: Appearance
    }

    let toml_content = r##"
        [appearance]
        opacity = 0.85
        background_color = "#1a1b26"
    "##;

    let config: TestConfig = ::toml::from_str(toml_content).expect("Failed to deserialize");
    assert_eq!(config.appearance.opacity, 0.85);
    assert_eq!(
        config.appearance.background_color,
        AppearanceColor::Simple(HexColor::rgb(26, 27, 38))
    );
}

#[test]
fn preset_theme_takes_precedence_over_appearance_fields() {
    #[derive(serde::Deserialize)]
    struct TestConfig {
        #[serde(deserialize_with = "super::themes::deserialize_theme_or_appearance")]
        appearance: Appearance
    }

    let toml_content = r#"
        appearance = "dracula"
    "#;

    let config: TestConfig = ::toml::from_str(toml_content).expect("Failed to deserialize");
    assert_eq!(
        config.appearance.background_color,
        AppearanceColor::Simple(HexColor::rgb(40, 42, 54))
    );
}

#[test]
fn an_unknown_theme_name_says_what_ships() {
    #[derive(Debug, serde::Deserialize)]
    struct TestConfig {
        #[serde(deserialize_with = "super::themes::deserialize_theme_or_appearance")]
        appearance: Appearance
    }

    assert!(preset_appearance("mocha").is_none());

    let err = ::toml::from_str::<TestConfig>("appearance = \"mocha\"")
        .expect_err("an unknown theme should error");
    let message = err.to_string();
    assert!(message.contains("unknown theme 'mocha'"), "{message}");
    assert!(message.contains("catppuccin-mocha"), "{message}");

    let known = ::toml::from_str::<TestConfig>("appearance = \"nord\"")
        .expect("a shipped theme should parse");
    assert_eq!(
        known.appearance.background_color,
        AppearanceColor::Simple(HexColor::rgb(46, 52, 64))
    );
}
