//! Where the HyDE clone this machine was installed from stands.

/// Finds the HyDE clone this machine was installed from, if any.
///
/// The path is what `version.sh --cache` recorded in the state directory,
/// with the conventional `~/HyDE` as the fallback; either way it only
/// counts when a git repository actually stands there.
pub(super) fn find_hyde_clone() -> Option<std::path::PathBuf> {
    let recorded = dirs::state_dir()
        .map(|state| state.join("hyde").join("version"))
        .and_then(|version| std::fs::read_to_string(version).ok())
        .and_then(|content| clone_path_from(&content));

    recorded
        .into_iter()
        .chain(dirs::home_dir().map(|home| home.join("HyDE")))
        .find(|path| path.join(".git").exists())
}

/// Reads `HYDE_CLONE_PATH` out of the cached version file.
pub(super) fn clone_path_from(content: &str) -> Option<std::path::PathBuf> {
    content.lines().find_map(|line| {
        line.strip_prefix("HYDE_CLONE_PATH=")
            .map(|value| value.trim().trim_matches('\'').trim_matches('"'))
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
    })
}
