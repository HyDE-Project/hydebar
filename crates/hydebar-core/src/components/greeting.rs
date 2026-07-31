//! The line the bar greets its user with while it is born.

use chrono::Timelike;

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

    match user.map(str::trim).filter(|user| !user.is_empty()) {
        Some(user) => format!("{phrase}, {}", capitalized(user)),
        None => phrase.to_owned()
    }
}

/// `name` with its first letter raised, the way a greeting addresses one.
fn capitalized(name: &str) -> String {
    let mut chars = name.chars();

    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new()
    }
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
}
