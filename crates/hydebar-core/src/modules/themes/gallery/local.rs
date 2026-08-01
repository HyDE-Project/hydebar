//! Facts about the gallery this machine already holds: the screenshots the
//! desktop's own tooling keeps, and the name its git identity signs with.

use crate::modules::themes::view;

/// Screenshots `HyDE`'s local gallery database keeps, by canonical theme name.
///
/// The database is written by the desktop's own gallery tooling — one
/// directory per theme with a `screenshot.png` of the desktop wearing it —
/// so the cards can show the real look from disk, no network involved.
pub(in crate::modules::themes) fn local_screenshots()
-> std::collections::HashMap<String, std::path::PathBuf> {
    let Some(base) = dirs::cache_dir().map(|cache| cache.join("hyde").join("gallery-database"))
    else {
        return std::collections::HashMap::new();
    };

    let Ok(entries) = std::fs::read_dir(base) else {
        return std::collections::HashMap::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let shot = entry.path().join("screenshot.png");

            shot.is_file().then(|| (view::canonical(&name), shot))
        })
        .collect()
}

/// The name this machine signs its work with.
///
/// Asked of git — the GitHub account name when one is configured, the
/// commit author name otherwise — so a gallery entry owned by the same
/// name can be marked as the user's own.
pub(in crate::modules::themes) async fn local_author() -> Option<String> {
    for key in ["github.user", "user.name"] {
        if let Ok(output) = tokio::process::Command::new("git")
            .args(["config", "--get", key])
            .output()
            .await
            && output.status.success()
        {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();

            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    None
}
