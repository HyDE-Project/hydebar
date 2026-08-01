//! The line the bar greets its user with while it is born.

use std::path::Path;

use chrono::Timelike;

/// Claims the one greeting of this login session.
///
/// The claim is a marker in the session runtime directory, which the login
/// manager creates at login and erases at logout — the marker scopes to the
/// session by construction, with nothing to clean up. The compositor instance
/// is written into it: entering a fresh desktop within the same login re-arms
/// the greeting, while a mere restart of the bar stays silent. A machine
/// without a runtime directory greets on every start, which is the least
/// surprising way to degrade.
#[must_use]
pub fn claim_first_entry() -> bool {
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return true;
    };

    let instance = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default();

    claim(
        &Path::new(&runtime).join("hydebar").join("greeted"),
        &instance
    )
}

/// Takes the claim at `marker` for `instance`, true when it was still free.
fn claim(marker: &Path, instance: &str) -> bool {
    if std::fs::read_to_string(marker).is_ok_and(|claimed| claimed == instance) {
        return false;
    }

    if let Some(dir) = marker.parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    let _ = std::fs::write(marker, instance);

    true
}

/// The greeting for the moment the bar comes up.
///
/// Addressed to whoever the session belongs to, by the hour their clock
/// shows.
#[must_use]
pub fn current() -> String {
    line(
        chrono::Local::now().hour(),
        std::env::var("USER").ok().as_deref()
    )
}

/// The greeting for `hour`, addressed to `user` when one is known.
fn line(hour: u32, user: Option<&str>) -> String {
    let phrase = match hour {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=22 => "Good evening",
        _ => "Good night"
    };

    user.map(str::trim)
        .filter(|user| !user.is_empty())
        .map_or_else(
            || phrase.to_owned(),
            |user| format!("{phrase}, {}", capitalized(user))
        )
}

/// `name` with its first letter raised, the way a greeting addresses one.
fn capitalized(name: &str) -> String {
    let mut chars = name.chars();

    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hour_of_the_day_has_a_greeting() {
        assert_eq!(line(7, None), "Good morning");
        assert_eq!(line(13, None), "Good afternoon");
        assert_eq!(line(20, None), "Good evening");
        assert_eq!(line(2, None), "Good night");
        assert_eq!(line(23, None), "Good night");
    }

    #[test]
    fn the_user_is_addressed_by_a_raised_name() {
        assert_eq!(line(7, Some("ra")), "Good morning, Ra");
    }

    #[test]
    fn a_blank_user_is_not_addressed() {
        assert_eq!(line(7, Some("  ")), "Good morning");
        assert_eq!(line(7, Some("")), "Good morning");
    }

    fn scratch_marker(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("hydebar-greeting-{}-{tag}", std::process::id()))
            .join("greeted")
    }

    #[test]
    fn the_first_entry_takes_the_claim_and_a_restart_does_not() {
        let marker = scratch_marker("restart");

        assert!(claim(&marker, "session-a"));
        assert!(!claim(&marker, "session-a"), "a bar restart stays silent");

        let _ = std::fs::remove_dir_all(marker.parent().expect("parent"));
    }

    #[test]
    fn a_fresh_desktop_within_the_same_login_re_arms_the_claim() {
        let marker = scratch_marker("fresh");

        assert!(claim(&marker, "session-a"));
        assert!(claim(&marker, "session-b"), "a new compositor greets again");
        assert!(!claim(&marker, "session-b"));

        let _ = std::fs::remove_dir_all(marker.parent().expect("parent"));
    }

    #[test]
    fn the_claim_creates_its_own_directory() {
        let marker = scratch_marker("mkdir");
        let _ = std::fs::remove_dir_all(marker.parent().expect("parent"));

        assert!(claim(&marker, "session-a"));
        assert!(marker.exists());

        let _ = std::fs::remove_dir_all(marker.parent().expect("parent"));
    }
}
