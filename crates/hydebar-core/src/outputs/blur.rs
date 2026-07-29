//! Blur the compositor paints behind the bar.
//!
//! A layer surface is not blurred because it asks the Wayland compositor for
//! it: the protocol has no such request. Hyprland blurs a surface only when a
//! `layerrule` in its configuration names the namespace the surface was
//! created with, so a bar that ships no rule of its own is at the mercy of
//! whatever the desktop happens to have written down. The HyDE Project blurs
//! the namespaces of the programs it ships and nothing else, and its rules
//! moved from `windowrules.conf` to a Lua configuration during the Hyprland
//! 0.55 migration, which drops any rule a user had added beside them.
//!
//! So the bar states the rule itself, once, before the first surface is
//! created. `hyprctl keyword` adds it to the running configuration the same
//! way the configuration file would, which leaves the desktop configuration
//! untouched and works on a machine that has never heard of the bar.

use std::process::Command;

use super::wayland::MAIN_NAMESPACE;

/// Alpha at or below which a pixel of the bar is not worth blurring behind.
///
/// The bar paints its own background nearly clear — the HyDE theme states it
/// at one hundredth of a unit, the built-in themes at nothing at all — and only
/// the islands drawn on top of it are meant to read as surfaces. Blurring
/// behind every pixel the surface covers would smear a band across the whole
/// width of the screen wherever the bar passes over a window.
const IGNORED_ALPHA: f32 = 0.1;

/// One rule, stated in both the syntaxes the compositor has had.
///
/// Hyprland 0.53 replaced the positional rule syntax with a named one and kept
/// the old spelling working only for a while, so the rule is offered in the
/// current spelling first and in the old one when the compositor refuses it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Rule {
    /// The rule as Hyprland 0.53 and later state it.
    pub(crate) current: String,
    /// The rule as Hyprland stated it before 0.53.
    pub(crate) legacy:  String
}

/// Rules that make the compositor blur what shows through `namespace`.
///
/// Only the surface the bar itself is drawn on is named. The menus, the
/// tooltips and the notifications live on surfaces that span the screen so
/// their content can be placed anywhere on it, and a blur rule matching those
/// would blur the whole desktop for as long as one of them is up.
pub(crate) fn rules(namespace: &str) -> [Rule; 2] {
    let matched = format!("^({namespace})$");

    [
        Rule {
            current: format!("blur true, match:namespace {matched}"),
            legacy:  format!("blur, {matched}")
        },
        Rule {
            current: format!("ignore_alpha {IGNORED_ALPHA}, match:namespace {matched}"),
            legacy:  format!("ignore_alpha {IGNORED_ALPHA}, {matched}")
        }
    ]
}

/// Reads whether the compositor took a rule.
///
/// It answers `ok` and nothing else when it did; a refusal carries the reason,
/// which is worth nothing here beyond meaning the other spelling is due.
pub(crate) fn accepted(answer: &str) -> bool {
    answer.trim().eq_ignore_ascii_case("ok")
}

/// Whether the bar is running on the compositor these rules are written for.
fn on_hyprland() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

/// Hands one rule to the compositor, returning its answer.
fn keyword(rule: &str) -> Option<String> {
    let output = Command::new("hyprctl")
        .args(["keyword", "layerrule", rule])
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// States one rule, falling back to the old spelling when the current one is
/// refused.
fn state(rule: &Rule) {
    if keyword(&rule.current).is_some_and(|answer| accepted(&answer)) {
        return;
    }

    keyword(&rule.legacy);
}

/// Asks the compositor to blur what shows through the bar.
///
/// Does nothing anywhere the compositor is not Hyprland, and nothing beyond
/// wasting a moment where the desktop already states the same rule or has blur
/// switched off altogether.
pub(crate) fn request() {
    if !on_hyprland() {
        return;
    }

    for rule in rules(MAIN_NAMESPACE) {
        state(&rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_asks_for_blur_and_for_its_clear_background_to_be_left_alone() {
        let [blur, ignore_alpha] = rules("hydebar-main-layer");

        assert_eq!(
            blur.current,
            "blur true, match:namespace ^(hydebar-main-layer)$"
        );
        assert_eq!(
            ignore_alpha.current,
            "ignore_alpha 0.1, match:namespace ^(hydebar-main-layer)$"
        );
    }

    #[test]
    fn every_rule_is_also_stated_the_way_hyprland_used_to() {
        let [blur, ignore_alpha] = rules("hydebar-main-layer");

        assert_eq!(blur.legacy, "blur, ^(hydebar-main-layer)$");
        assert_eq!(ignore_alpha.legacy, "ignore_alpha 0.1, ^(hydebar-main-layer)$");
    }

    #[test]
    fn only_the_surface_the_bar_is_drawn_on_is_named() {
        // a rule matching the menu, tooltip or notification surface would blur
        // the whole desktop, all three span the screen
        for rule in rules(MAIN_NAMESPACE) {
            assert!(rule.current.contains("^(hydebar-main-layer)$"));
            assert!(!rule.current.contains("hydebar-menu-layer"));
            assert!(!rule.current.contains("hydebar-tooltip-layer"));
            assert!(!rule.current.contains("hydebar-notifications-layer"));
        }
    }

    #[test]
    fn the_ignored_alpha_sits_between_the_bar_background_and_an_island() {
        // the HyDE theme states the bar background at 0.01 and an island at
        // 0.8: the strip must be skipped and the islands must not be
        assert!(IGNORED_ALPHA > 0.01);
        assert!(IGNORED_ALPHA < 0.8);
    }

    #[test]
    fn a_taken_rule_is_told_apart_from_a_refused_one() {
        assert!(accepted("ok"));
        assert!(accepted("ok\n"));
        assert!(accepted("OK"));
        assert!(!accepted("Config error: invalid rule"));
        assert!(!accepted(""));
    }

    #[test]
    fn nothing_is_asked_of_a_compositor_that_is_not_hyprland() {
        // the fallback spelling would otherwise be handed to whatever binary
        // happens to answer to the name
        assert_eq!(
            on_hyprland(),
            std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
        );
    }
}
