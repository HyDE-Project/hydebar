//! Where the bar's theme switch script lives, and how it is found.
//!
//! The bar does not switch a `HyDE` theme through `hyde-shell` any more: the
//! stock switch regenerates waybar's stylesheet and restarts waybar, which
//! costs a visible delay on every switch and makes no sense on a desktop that
//! replaced waybar with this bar. The script shipped in `scripts/theme-switch`
//! performs the same switch with that one step left out.
//!
//! The script is a separate file, so its path has to be discovered rather than
//! spelled out: an installed bar keeps it next to the binary, a packaged one
//! under `share/hydebar/scripts`, a user-local install under the XDG data
//! directory, and a checkout keeps it in the repository. Every location is
//! tried in that order and the first executable file wins;
//! `HYDEBAR_THEME_SWITCH` names one outright and is tried before all of them.

use std::{
    env, fmt,
    path::{Path, PathBuf}
};

/// Name the script carries when it is installed next to the bar binary.
///
/// The binary directory is shared with the rest of the system, so the script
/// is named after the bar there instead of by what it does.
const EXECUTABLE_NAME: &str = "hydebar-theme-switch";

/// Name the script carries inside a directory that is already the bar's own.
const SCRIPT_NAME: &str = "theme-switch";

/// Environment variable naming the script outright.
const OVERRIDE_VARIABLE: &str = "HYDEBAR_THEME_SWITCH";

/// Directories a distribution package may install the script into.
const SYSTEM_DATA_DIRECTORIES: [&str; 2] = ["/usr/local/share", "/usr/share"];

/// Failure to find the theme switch script.
///
/// The error carries every path that was tried, because the fix is always to
/// put the script in one of them — or to point [`OVERRIDE_VARIABLE`] at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNotFound {
    /// Paths that were probed, in the order they were probed.
    candidates: Vec<PathBuf>
}

impl fmt::Display for ScriptNotFound {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "the theme switch script was not found; looked for an executable at "
        )?;

        for (index, candidate) in self.candidates.iter().enumerate() {
            if index > 0 {
                write!(formatter, ", ")?;
            }

            write!(formatter, "`{}`", candidate.display())?;
        }

        write!(
            formatter,
            "; install it or set {OVERRIDE_VARIABLE} to its path"
        )
    }
}

impl std::error::Error for ScriptNotFound {}

/// Path of the theme switch script.
///
/// # Errors
///
/// Returns [`ScriptNotFound`] when none of the searched locations holds an
/// executable file.
pub fn locate() -> Result<PathBuf, ScriptNotFound> {
    let executable = env::current_exe().ok();
    let candidates = candidates(
        env::var_os(OVERRIDE_VARIABLE)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
        executable.as_deref().and_then(Path::parent),
        data_home().as_deref()
    );

    pick(candidates, is_executable_file)
}

/// Every path the script may live at, most specific first.
///
/// `binary_directory` is the directory the running bar was started from and
/// `data_home` the user's XDG data directory; either may be absent, in which
/// case the locations derived from it are simply not searched.
fn candidates(
    override_path: Option<PathBuf>,
    binary_directory: Option<&Path>,
    data_home: Option<&Path>
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    candidates.extend(override_path);

    if let Some(directory) = binary_directory {
        candidates.push(directory.join(EXECUTABLE_NAME));
        candidates.push(directory.join("../share/hydebar/scripts").join(SCRIPT_NAME));
    }

    if let Some(directory) = data_home {
        candidates.push(directory.join("hydebar/scripts").join(SCRIPT_NAME));
    }

    for directory in SYSTEM_DATA_DIRECTORIES {
        candidates.push(
            Path::new(directory)
                .join("hydebar/scripts")
                .join(SCRIPT_NAME)
        );
    }

    if let Some(directory) = binary_directory {
        candidates.push(directory.join("../../scripts").join(SCRIPT_NAME));
    }

    candidates
}

/// First candidate `probe` accepts, or the whole list as an error.
fn pick(
    candidates: Vec<PathBuf>,
    probe: impl Fn(&Path) -> bool
) -> Result<PathBuf, ScriptNotFound> {
    if let Some(found) = candidates.iter().find(|candidate| probe(candidate)) {
        return Ok(found.clone());
    }

    Err(ScriptNotFound {
        candidates
    })
}

/// The user's XDG data directory, if one can be worked out.
fn data_home() -> Option<PathBuf> {
    if let Some(value) = env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(value));
    }

    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| Path::new(&home).join(".local/share"))
}

/// Whether `path` is a file the bar is allowed to run.
///
/// A readable but non-executable file is rejected on purpose: it is a script
/// that was copied without its mode bits, and running it would fail later with
/// a far less obvious message.
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt};

    use tempfile::tempdir;

    use super::*;

    fn paths(candidates: &[PathBuf]) -> Vec<String> {
        candidates
            .iter()
            .map(|candidate| candidate.display().to_string())
            .collect()
    }

    #[test]
    fn an_installed_bar_looks_next_to_itself_first() {
        let candidates = candidates(None, Some(Path::new("/usr/bin")), None);

        assert_eq!(
            candidates.first(),
            Some(&PathBuf::from("/usr/bin/hydebar-theme-switch"))
        );
    }

    #[test]
    fn the_data_directory_is_searched_before_the_system_ones() {
        let candidates = paths(&candidates(
            None,
            Some(Path::new("/usr/bin")),
            Some(Path::new("/home/user/.local/share"))
        ));

        let user = candidates
            .iter()
            .position(|candidate| {
                candidate == "/home/user/.local/share/hydebar/scripts/theme-switch"
            })
            .expect("the data directory is searched");
        let system = candidates
            .iter()
            .position(|candidate| candidate == "/usr/share/hydebar/scripts/theme-switch")
            .expect("the system directory is searched");

        assert!(user < system);
    }

    #[test]
    fn a_checkout_is_searched_last() {
        let candidates = candidates(
            None,
            Some(Path::new("/src/hydebar/target/debug")),
            Some(Path::new("/home/user/.local/share"))
        );

        assert_eq!(
            candidates.last(),
            Some(&PathBuf::from(
                "/src/hydebar/target/debug/../../scripts/theme-switch"
            ))
        );
    }

    #[test]
    fn an_override_outranks_every_other_location() {
        let candidates = candidates(
            Some(PathBuf::from("/opt/switch")),
            Some(Path::new("/usr/bin")),
            Some(Path::new("/home/user/.local/share"))
        );

        assert_eq!(candidates.first(), Some(&PathBuf::from("/opt/switch")));
    }

    #[test]
    fn a_missing_binary_directory_only_drops_its_own_locations() {
        let candidates = paths(&candidates(None, None, Some(Path::new("/data"))));

        assert_eq!(
            candidates,
            vec![
                "/data/hydebar/scripts/theme-switch".to_owned(),
                "/usr/local/share/hydebar/scripts/theme-switch".to_owned(),
                "/usr/share/hydebar/scripts/theme-switch".to_owned(),
            ]
        );
    }

    #[test]
    fn the_first_accepted_candidate_is_taken() {
        let chosen = pick(
            vec![PathBuf::from("/first"), PathBuf::from("/second")],
            |candidate| candidate == Path::new("/second")
        );

        assert_eq!(chosen, Ok(PathBuf::from("/second")));
    }

    #[test]
    fn the_error_names_every_location_that_was_tried() {
        let error = pick(
            vec![PathBuf::from("/first"), PathBuf::from("/second")],
            |_| false
        )
        .expect_err("no candidate is accepted");
        let message = error.to_string();

        assert!(message.contains("`/first`"), "{message}");
        assert!(message.contains("`/second`"), "{message}");
        assert!(message.contains(OVERRIDE_VARIABLE), "{message}");
    }

    #[test]
    fn a_script_without_its_mode_bits_is_not_accepted() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        let script = directory.path().join(SCRIPT_NAME);
        fs::write(&script, "#!/usr/bin/env bash\n")?;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o644))?;

        assert!(!is_executable_file(&script));

        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;

        assert!(is_executable_file(&script));

        Ok(())
    }

    #[test]
    fn a_directory_is_not_mistaken_for_the_script() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;

        assert!(!is_executable_file(directory.path()));
        assert!(!is_executable_file(&directory.path().join("absent")));

        Ok(())
    }
}
