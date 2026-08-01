//! Listing of the themes installed on this machine.
//!
//! `HyDE` installs a theme as a directory named after it, so the listing is the
//! directory listing. The only file read inside a theme is `.sort`, because
//! that is what decides the order `HyDE` itself presents the themes in.

use std::{cmp::Ordering, fs, path::Path};

/// File a theme states its place in the list in.
///
/// `HyDE` reads its first line as a number and orders the themes by it, so a
/// listing that ignores the file puts the themes in an order no other `HyDE`
/// surface shows.
const SORT_FILE: &str = ".sort";

/// Place a theme without a `.sort` file takes.
///
/// The scripts spell this as `thmSortS+=("0")`, so an unsorted theme sorts
/// ahead of every theme that numbered itself from one.
const DEFAULT_SORT: i64 = 0;

/// A theme together with the place it asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    /// Number read from the theme's `.sort` file.
    sort: i64,
    /// Directory name, which is the name `HyDE` addresses the theme by.
    name: String
}

/// Names of the themes installed under `dir`.
///
/// A directory that cannot be read yields an empty list rather than an error:
/// the machine may simply not have `HyDE` installed, and that is not a failure
/// the bar has anything to say about.
#[must_use]
pub(super) fn installed(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let entries = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .map(|name| Entry {
            sort: sort_key(&dir.join(&name)),
            name
        })
        .collect();

    order(entries)
}

/// Place the theme in `directory` asked for.
///
/// Anything the file cannot be read as a number leaves the theme at
/// [`DEFAULT_SORT`], which is what the scripts do with a `.sort` they cannot
/// read either.
fn sort_key(directory: &Path) -> i64 {
    fs::read_to_string(directory.join(SORT_FILE))
        .ok()
        .and_then(|source| {
            source
                .lines()
                .next()
                .map(str::trim)
                .and_then(|line| line.parse().ok())
        })
        .unwrap_or(DEFAULT_SORT)
}

/// Puts theme entries into the order `HyDE` itself walks them.
///
/// The `.sort` number leads and the alphabet breaks its ties, exactly the
/// order `hydectl theme next` and `prev` traverse. A list sorted any other
/// way had the keybinding jump to a theme other than the next chip in the
/// bar's own grid.
#[must_use]
fn order(entries: Vec<Entry>) -> Vec<String> {
    let mut entries = entries;

    entries.sort_by(|left, right| {
        left.sort
            .cmp(&right.sort)
            .then_with(|| compare_names(&left.name, &right.name))
    });
    entries.dedup();
    entries.into_iter().map(|entry| entry.name).collect()
}

/// Orders two theme names, ignoring case first so the list reads the way a user
/// would write it down.
fn compare_names(left: &str, right: &str) -> Ordering {
    left.to_lowercase()
        .cmp(&right.to_lowercase())
        .then_with(|| left.cmp(right))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn entries(values: &[(i64, &str)]) -> Vec<Entry> {
        values
            .iter()
            .map(|(sort, name)| Entry {
                sort: *sort,
                name: (*name).to_owned()
            })
            .collect()
    }

    /// `HyDE`'s `.sort` numbers lead, so the bar's grid and the desktop's
    /// next/prev keybinding walk the very same order.
    #[test]
    fn themes_follow_the_sort_numbers_hyde_walks() {
        assert_eq!(
            order(entries(&[
                (4, "Tokyo Night"),
                (1, "Catppuccin Mocha"),
                (3, "Rosé Pine"),
                (2, "Catppuccin Latte")
            ])),
            vec![
                "Catppuccin Mocha".to_owned(),
                "Catppuccin Latte".to_owned(),
                "Rosé Pine".to_owned(),
                "Tokyo Night".to_owned(),
            ]
        );
    }

    #[test]
    fn the_alphabet_reads_past_letter_case() {
        assert_eq!(
            order(entries(&[
                (0, "tokyo night"),
                (0, "Catppuccin Mocha"),
                (0, "decay green"),
                (0, "Nordic Blue")
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
    fn an_empty_directory_lists_no_themes() {
        assert!(order(Vec::new()).is_empty());
    }

    #[test]
    fn a_missing_directory_lists_no_themes() {
        assert!(installed(Path::new("/nonexistent/hyde/themes")).is_empty());
    }

    #[test]
    fn a_hidden_directory_is_not_a_theme() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempdir()?;
        fs::create_dir(root.path().join(".git"))?;
        fs::create_dir(root.path().join("Gruvbox Retro"))?;

        assert_eq!(installed(root.path()), vec!["Gruvbox Retro".to_owned()]);

        Ok(())
    }

    #[test]
    fn an_installed_listing_follows_the_sort_files_on_disk()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempdir()?;

        for (sort, name) in [
            (Some("3"), "Rosé Pine"),
            (Some("1"), "Mocha"),
            (None, "Loose")
        ] {
            let directory = root.path().join(name);
            fs::create_dir(&directory)?;

            if let Some(sort) = sort {
                fs::write(directory.join(SORT_FILE), format!("{sort}\n"))?;
            }
        }

        assert_eq!(
            installed(root.path()),
            vec![
                "Loose".to_owned(),
                "Mocha".to_owned(),
                "Rosé Pine".to_owned()
            ]
        );

        Ok(())
    }

    #[test]
    fn an_unreadable_sort_file_leaves_the_theme_unnumbered()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempdir()?;
        let directory = root.path().join("Broken");
        fs::create_dir(&directory)?;
        fs::write(directory.join(SORT_FILE), "not a number\n")?;

        assert_eq!(sort_key(&directory), DEFAULT_SORT);

        Ok(())
    }

    #[test]
    fn a_file_is_not_mistaken_for_a_theme() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempdir()?;
        fs::write(root.path().join("README.md"), "")?;
        fs::create_dir(root.path().join("Synth Wave"))?;

        assert_eq!(installed(root.path()), vec!["Synth Wave".to_owned()]);

        Ok(())
    }
}
