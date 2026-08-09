//! Blur the compositor paints behind the bar.
//!
//! A layer surface is not blurred because it asks the Wayland compositor for
//! it: the protocol has no such request. Hyprland blurs a surface only when a
//! layer rule in its configuration names the namespace the surface was created
//! with, so a bar that ships no rule of its own is at the mercy of whatever the
//! desktop happens to have written down. The `HyDE` Project blurs the
//! namespaces of the programs it ships and nothing else, and its rules moved
//! from `windowrules.conf` to a Lua configuration during the Hyprland 0.55
//! migration, which drops any rule a user had added beside them.
//!
//! So the bar states the rule itself, once, before the first surface is
//! created. That leaves the desktop configuration untouched and works on a
//! machine that has never heard of the bar.
//!
//! How the rule is handed over depends on how the session was configured. A
//! Hyprland reading a Lua configuration refuses the `keyword` request outright
//! — it answers `keyword can't work with non-legacy parsers` — and takes rules
//! only through `eval`, which runs a line of Lua against the running
//! configuration. A Hyprland reading the older configuration format has no
//! evaluator at all. Both spellings are therefore offered, the Lua one first.

use std::sync::atomic::{AtomicU8, Ordering};

use hydebar_proto::compositor_ipc;

use super::wayland::MAIN_NAMESPACE;

/// Marker for a session whose accepted spelling is not yet known.
const SPELLING_UNKNOWN: u8 = u8::MAX;

/// Index of the spelling this session accepted, once one has been.
///
/// The parser a session runs does not change while it runs, and the bar
/// restates its rules after every compositor reload — a burst of them on a
/// theme switch. Remembering the accepted spelling turns each restatement
/// into one request instead of a probe of up to three per rule, the same
/// bargain the dispatch dialect strikes.
static ACCEPTED_SPELLING: AtomicU8 = AtomicU8::new(SPELLING_UNKNOWN);

/// Alpha at or below which a pixel of the bar is not worth blurring behind.
///
/// Set low enough that the whole strip is blurred, islands and the space
/// between them alike: a bar blurred only under its islands reads as a row of
/// pills floating over a sharp wallpaper, which is not what a blurred bar looks
/// like anywhere else on the desktop. Anything fully clear is still skipped, so
/// a bar configured with no background at all costs the compositor nothing.
const IGNORED_ALPHA: f32 = 0.0;

/// One rule, stated in every syntax the compositor has had.
///
/// The Lua form is what a session configured in Lua takes; the two keyword
/// forms are what the older configuration parser takes, the named one from
/// Hyprland 0.53 and the positional one from before it.
#[derive(Debug, PartialEq, Eq)]
struct Rule {
    /// The rule as a line of Lua, for a session with the Lua parser.
    lua:        String,
    /// The rule as Hyprland 0.53 and later spell it for the keyword command.
    keyword:    String,
    /// The rule as Hyprland spelled it before 0.53.
    positional: String
}

/// Rules that make the compositor blur what shows through `namespace`.
///
/// Only the surface the bar itself is drawn on is named. The menus, the
/// tooltips and the notifications live on surfaces that span the screen so
/// their content can be placed anywhere on it, and a blur rule matching those
/// would blur the whole desktop for as long as one of them is up.
fn rules(namespace: &str) -> [Rule; 2] {
    let matched = format!("^({namespace})$");

    [
        Rule {
            lua:        format!(
                "hl.layer_rule({{ name = \"hydebar_blur\", match = {{ namespace = \"{matched}\" \
                 }}, blur = true }})"
            ),
            keyword:    format!("blur true, match:namespace {matched}"),
            positional: format!("blur, {matched}")
        },
        Rule {
            lua:        format!(
                "hl.layer_rule({{ name = \"hydebar_ignore_alpha\", match = {{ namespace = \
                 \"{matched}\" }}, ignore_alpha = {IGNORED_ALPHA:.1} }})"
            ),
            keyword:    format!("ignore_alpha {IGNORED_ALPHA:.1}, match:namespace {matched}"),
            positional: format!("ignore_alpha {IGNORED_ALPHA:.1}, {matched}")
        }
    ]
}

/// Reads whether the compositor took a rule.
///
/// It answers `ok` and nothing else when it did; a refusal carries the reason,
/// which is worth nothing here beyond meaning the next spelling is due.
fn accepted(answer: &str) -> bool {
    answer.trim().eq_ignore_ascii_case("ok")
}

/// Whether the bar is running on the compositor these rules are written for.
fn on_hyprland() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

/// States one rule and returns what the compositor answered.
fn state_rule(command: &str, argument: &str) -> Option<String> {
    compositor_ipc::request(&format!("{command} {argument}"))
}

/// Hands one rule over in whichever spelling the session takes.
///
/// The evaluator is tried first because a session that has one refuses the
/// keyword command outright, while a session that has none simply answers that
/// it does not know the command, which costs a moment and nothing else.
fn state(rule: &Rule) -> bool {
    let spellings = [
        ("eval", rule.lua.as_str()),
        ("keyword", rule.keyword.as_str()),
        ("keyword", rule.positional.as_str())
    ];

    let known = ACCEPTED_SPELLING.load(Ordering::Relaxed);
    if let Some(&(command, argument)) = spellings.get(usize::from(known)) {
        if try_spelling(command, argument) {
            return true;
        }

        ACCEPTED_SPELLING.store(SPELLING_UNKNOWN, Ordering::Relaxed);
    }

    for (index, (command, argument)) in spellings.into_iter().enumerate() {
        if index == usize::from(known) {
            continue;
        }

        if try_spelling(command, argument) {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the spelling table holds far fewer than 256 entries"
            )]
            ACCEPTED_SPELLING.store(index as u8, Ordering::Relaxed);
            return true;
        }
    }

    false
}

/// Hands one spelling over and reads whether the compositor took it.
fn try_spelling(command: &str, argument: &str) -> bool {
    let spelled = if command == "keyword" {
        format!("layerrule {argument}")
    } else {
        argument.to_owned()
    };

    state_rule(command, &spelled).is_some_and(|answer| accepted(&answer))
}

/// Asks the compositor to blur what shows through the bar.
///
/// Does nothing anywhere the compositor is not Hyprland, and nothing beyond
/// wasting a moment where the desktop already states the same rule or has blur
/// switched off altogether.
pub fn request() {
    if !on_hyprland() {
        return;
    }

    if hydebar_proto::compositor_look::CompositorLook::read().blur == Some(false) {
        return;
    }

    for rule in rules(MAIN_NAMESPACE) {
        state(&rule);
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_bar_asks_for_blur_and_for_its_clear_background_to_be_left_alone() {
        let [blur, ignore_alpha] = rules("hydebar-main-layer");

        assert_eq!(
            blur.keyword,
            "blur true, match:namespace ^(hydebar-main-layer)$"
        );
        assert_eq!(
            ignore_alpha.keyword,
            "ignore_alpha 0.0, match:namespace ^(hydebar-main-layer)$"
        );
    }

    #[test]
    fn a_lua_session_is_handed_a_line_of_lua() {
        let [blur, ignore_alpha] = rules("hydebar-main-layer");

        assert_eq!(
            blur.lua,
            "hl.layer_rule({ name = \"hydebar_blur\", match = { namespace = \
             \"^(hydebar-main-layer)$\" }, blur = true })"
        );
        assert_eq!(
            ignore_alpha.lua,
            "hl.layer_rule({ name = \"hydebar_ignore_alpha\", match = { namespace = \
             \"^(hydebar-main-layer)$\" }, ignore_alpha = 0.0 })"
        );
    }

    #[test]
    fn every_rule_is_also_stated_the_way_hyprland_used_to() {
        let [blur, ignore_alpha] = rules("hydebar-main-layer");

        assert_eq!(blur.positional, "blur, ^(hydebar-main-layer)$");
        assert_eq!(
            ignore_alpha.positional,
            "ignore_alpha 0.0, ^(hydebar-main-layer)$"
        );
    }

    #[test]
    fn only_the_surface_the_bar_is_drawn_on_is_named() {
        // a rule matching the menu, tooltip or notification surface would blur
        // the whole desktop, all three span the screen
        for rule in rules(MAIN_NAMESPACE) {
            for spelling in [&rule.lua, &rule.keyword, &rule.positional] {
                assert!(spelling.contains("^(hydebar-main-layer)$"));
                assert!(!spelling.contains("hydebar-menu-layer"));
                assert!(!spelling.contains("hydebar-tooltip-layer"));
                assert!(!spelling.contains("hydebar-notifications-layer"));
            }
        }
    }

    #[test]
    #[expect(
        clippy::assertions_on_constants,
        reason = "the test pins the constant below the themed background alpha"
    )]
    fn the_whole_strip_is_blurred_not_only_the_islands() {
        // the HyDE theme states the bar background at 0.01: a threshold above
        // it would leave the space between the islands sharp
        assert!(IGNORED_ALPHA < 0.01);
    }

    #[test]
    fn a_taken_rule_is_told_apart_from_a_refused_one() {
        assert!(accepted("ok"));
        assert!(accepted("ok\n"));
        assert!(accepted("OK"));
        assert!(!accepted(
            "keyword can't work with non-legacy parsers. Use eval."
        ));
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
