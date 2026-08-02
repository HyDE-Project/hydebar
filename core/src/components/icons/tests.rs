//! Regression tests of the glyph catalogue and its overrides.

use hydebar_proto::config::IconsConfig;

use super::{catalog::Icons, theme::IconTheme};

#[test]
fn default_theme_keeps_built_in_glyphs() {
    let theme = IconTheme::default();

    assert!(theme.is_default());
    for icon in Icons::ALL {
        assert_eq!(theme.glyph(icon), icon.default_glyph());
    }
}

#[test]
fn absent_override_keeps_built_in_glyph() {
    let config = IconsConfig::from_iter([("cpu", "\u{f035b}")]);
    let theme = IconTheme::from_config(&config);

    assert_eq!(theme.glyph(Icons::Mem), Icons::Mem.default_glyph());
    assert_eq!(theme.glyph(Icons::Temp), Icons::Temp.default_glyph());
}

#[test]
fn override_replaces_glyph() {
    let config = IconsConfig::from_iter([
        ("cpu", "\u{f035b}"),
        ("mem", "\u{f0f86}"),
        ("battery4", "\u{f0079}")
    ]);
    let theme = IconTheme::from_config(&config);

    assert_eq!(theme.glyph(Icons::Cpu), "\u{f035b}");
    assert_eq!(theme.glyph(Icons::Mem), "\u{f0f86}");
    assert_eq!(theme.glyph(Icons::Battery4), "\u{f0079}");
    assert!(!theme.is_default());
}

#[test]
fn unknown_override_is_ignored() {
    let config = IconsConfig::from_iter([("not_an_icon", "X"), ("cpu", "Y")]);
    let theme = IconTheme::from_config(&config);

    assert_eq!(theme.glyph(Icons::Cpu), "Y");
    assert_eq!(Icons::from_name("not_an_icon"), None);
}

#[test]
fn every_icon_has_a_unique_name() {
    let mut names = std::collections::HashSet::new();

    for icon in Icons::ALL {
        assert!(names.insert(icon.name()), "duplicate name {}", icon.name());
        assert_eq!(Icons::from_name(icon.name()), Some(icon));
    }

    assert_eq!(names.len(), Icons::ALL.len());
}

#[test]
fn set_overrides_a_single_icon() {
    let mut theme = IconTheme::default();
    theme.set(Icons::Bluetooth, "Z");

    assert_eq!(theme.glyph(Icons::Bluetooth), "Z");
    assert_eq!(theme.glyph(Icons::Vpn), Icons::Vpn.default_glyph());
}

/// The theme module draws its bar entry from the shared catalogue, so its
/// glyph is overridable from `[icons]` like every other one.
#[test]
fn the_theme_icon_is_part_of_the_overridable_catalogue() {
    assert!(Icons::ALL.contains(&Icons::Themes));

    let theme = IconTheme::from_config(&IconsConfig::from_iter([(Icons::Themes.name(), "X")]));

    assert_eq!(theme.glyph(Icons::Themes), "X");
}
