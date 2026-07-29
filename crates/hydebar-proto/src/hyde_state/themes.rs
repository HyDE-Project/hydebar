//! Listing of the themes installed on this machine.
//!
//! HyDE installs a theme as a directory named after it, so the listing is the
//! directory listing. Nothing inside a theme is read: the settings window only
//! needs a name to hand back to `hyde-shell`.

use std::{fs, path::Path};

/// Names of the themes installed under `dir`.
///
/// A directory that cannot be read yields an empty list rather than an error:
/// the machine may simply not have HyDE installed, and that is not a failure
/// the bar has anything to say about.
#[must_use]
pub(super) fn installed(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let names = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    order(names)
}

/// Puts theme names into the order the settings window lists them.
///
/// The order is alphabetical and ignores case, so the list reads the way a user
/// would write it down; the filesystem hands the entries over in whatever order
/// it stored them, which would otherwise make the list jump between reloads.
/// Hidden entries are dropped because HyDE never names a theme with a leading
/// dot, so such a directory is bookkeeping rather than a theme.
#[must_use]
fn order(names: Vec<String>) -> Vec<String> {
    let mut names: Vec<String> = names
        .into_iter()
        .filter(|name| !name.starts_with('.'))
        .collect();

    names.sort_by_key(|name| name.to_lowercase());
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: [&str; 4]) -> Vec<String> {
        values.into_iter().map(str::to_owned).collect()
    }

    #[test]
    fn themes_are_listed_alphabetically_regardless_of_case() {
        assert_eq!(
            order(names([
                "tokyo night",
                "Catppuccin Mocha",
                "decay green",
                "Nordic Blue"
            ])),
            vec![
                "Catppuccin Mocha".to_owned(),
                "decay green".to_owned(),
                "Nordic Blue".to_owned(),
                "tokyo night".to_owned(),
            ]
        );
    }

    #[test]
    fn a_hidden_directory_is_not_a_theme() {
        assert_eq!(
            order(names([".git", "Gruvbox Retro", ".cache", "Synth Wave"])),
            vec!["Gruvbox Retro".to_owned(), "Synth Wave".to_owned()]
        );
    }

    #[test]
    fn an_empty_directory_lists_no_themes() {
        assert!(order(Vec::new()).is_empty());
    }

    #[test]
    fn a_missing_directory_lists_no_themes() {
        assert!(installed(Path::new("/nonexistent/hyde/themes")).is_empty());
    }
}
