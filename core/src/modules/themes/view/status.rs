//! The row naming what the desktop is on, and what it is on its way to.

use hydebar_proto::hyde_state::HydeState;

use super::{SWITCHING_TO, UNKNOWN};

/// Renders what the desktop is on, and what it is on its way to.
///
/// The theme in force always comes from `HyDE`'s own state file; a theme the
/// bar asked for is drawn beside it rather than in its place, because
/// until the switch has finished the desktop is still on the old one
/// and a menu that already named the new one would be reporting
/// something that may yet fail.
pub(super) fn active_label(state: &HydeState, switching: Option<&str>) -> String {
    let active = state.theme.as_deref().unwrap_or(UNKNOWN);

    match switching {
        Some(pending) if !state.is_active(pending) => {
            format!("{active}{SWITCHING_TO}{pending}")
        }
        _ => active.to_owned()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn state(themes: &[&str], active: Option<&str>) -> HydeState {
        HydeState {
            theme:            active.map(str::to_owned),
            themes:           themes.iter().map(|name| (*name).to_owned()).collect(),
            wallpaper_colors: true,
            shader:           Some("wallbash".to_owned())
        }
    }

    #[test]
    fn a_menu_that_is_not_switching_names_the_theme_in_force() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));

        assert_eq!(active_label(&state, None), "Nord");
    }

    #[test]
    fn a_running_switch_names_both_themes_and_keeps_the_old_one_first() {
        let state = state(&["Nord", "Mocha"], Some("Nord"));

        let label = active_label(&state, Some("Mocha"));

        assert!(label.starts_with("Nord"), "{label}");
        assert!(label.ends_with("Mocha"), "{label}");
    }

    /// `HyDE` writes the new name into its state file long before the switch
    /// is over, so the menu has to stop drawing an arrow that
    /// points at the theme it already reports.
    #[test]
    fn a_switch_the_state_file_already_reports_is_named_only_once() {
        let state = state(&["Nord", "Mocha"], Some("Mocha"));

        assert_eq!(active_label(&state, Some("Mocha")), "Mocha");
    }

    #[test]
    fn a_desktop_without_a_theme_still_names_the_one_being_switched_to() {
        let state = state(&["Nord"], None);

        assert_eq!(
            active_label(&state, Some("Nord")),
            format!("{UNKNOWN}{SWITCHING_TO}Nord")
        );
    }
}
