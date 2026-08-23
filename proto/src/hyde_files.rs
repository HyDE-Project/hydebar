//! Reading the files the desktop keeps, and saying so when one refuses.
//!
//! Every one of these files is optional: a machine with no `HyDE` install, or
//! one that has never been themed, is missing most of them, and the readers
//! above fall through to their own defaults. So an absent file is silence.
//!
//! A file that is there and still will not be read is a different thing
//! entirely — a wrong owner, a directory where a file belongs, a mount that
//! went away — and it leaves the bar unthemed for a reason the user cannot
//! see. Those are named in the journal, once, at the path they happened to.

use std::{fs, io, path::Path};

use log::warn;

/// The contents of `path`, or [`None`] where there is nothing to read.
///
/// Warns when the file exists and refuses to be read, so a permission or a
/// device error is not mistaken for a desktop that was never themed.
pub fn text(path: &Path) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(source) => Some(source),
        Err(err) => {
            report(path, &err);

            None
        }
    }
}

/// The contents of `path`, empty where there is nothing to read.
///
/// For the readers that walk a chain of files and take the first answer any
/// of them gives: an empty source states nothing, which is exactly what a
/// file that is not there states.
pub fn text_or_empty(path: &Path) -> String {
    text(path).unwrap_or_default()
}

/// The entries of the directory at `path`, or [`None`] where there are none
/// to read.
pub fn entries(path: &Path) -> Option<fs::ReadDir> {
    match fs::read_dir(path) {
        Ok(entries) => Some(entries),
        Err(err) => {
            report(path, &err);

            None
        }
    }
}

/// Names a refusal that is not simply an absence.
fn report(path: &Path, err: &io::Error) {
    if err.kind() == io::ErrorKind::NotFound {
        return;
    }

    warn!(
        "the desktop file at {} exists and could not be read: {err}",
        path.display()
    );
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn a_file_that_is_there_reads_whole() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("staterc");
        fs::write(&path, "HYDE_THEME=\"Catppuccin\"\n").expect("write");

        assert_eq!(text(&path).as_deref(), Some("HYDE_THEME=\"Catppuccin\"\n"));
    }

    #[test]
    fn a_file_that_is_not_there_reads_as_nothing() {
        let dir = tempfile::tempdir().expect("temporary directory");

        assert!(text(&dir.path().join("absent")).is_none());
        assert!(text_or_empty(&dir.path().join("absent")).is_empty());
    }

    #[test]
    fn a_directory_where_a_file_belongs_is_a_refusal_not_an_absence() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("staterc");
        fs::create_dir(&path).expect("directory");

        assert!(text(&path).is_none());
        assert_ne!(
            fs::read_to_string(&path).unwrap_err().kind(),
            io::ErrorKind::NotFound,
            "the case under test must not be a plain absence"
        );
    }

    #[test]
    fn a_directory_that_is_there_lists_its_entries() {
        let dir = tempfile::tempdir().expect("temporary directory");
        fs::create_dir(dir.path().join("Catppuccin")).expect("theme directory");

        let listed = entries(dir.path()).expect("the directory reads").count();

        assert_eq!(listed, 1);
    }

    #[test]
    fn a_directory_that_is_not_there_lists_nothing() {
        let dir = tempfile::tempdir().expect("temporary directory");

        assert!(entries(&dir.path().join("absent")).is_none());
    }
}
