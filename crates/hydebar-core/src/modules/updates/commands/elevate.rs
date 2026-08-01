//! The password dialog an update without a terminal asks through.

/// The dialog sudo raises when it has no terminal to ask on.
///
/// The update runs without a terminal, and sudo in that position turns to
/// the helper `SUDO_ASKPASS` names — once per elevated run, with its usual
/// grace period after, unlike polkit's per-command prompting that turned a
/// long install into a hail of dialogs.
///
/// The question is asked by rofi where it exists: on a `HyDE` desktop rofi
/// already wears the theme in force, where the GTK dialogs answer to
/// nobody's palette. Zenity stays as the fallback.
const ASKPASS: &str = concat!(
    "#!/usr/bin/env bash\n",
    "if command -v rofi >/dev/null 2>&1; then\n",
    "  exec rofi -dmenu -password -p \"${1:-Password}\" </dev/null\n",
    "fi\n",
    "exec zenity --password --title='Updates' \"$@\"\n"
);

/// Writes the password dialog helper into `dir` and returns its path.
fn write_askpass(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(dir).ok()?;

    let helper = dir.join("askpass");
    std::fs::write(&helper, ASKPASS).ok()?;

    let mut permissions = std::fs::metadata(&helper).ok()?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&helper, permissions).ok()?;

    Some(helper)
}

/// The password helper the update should run with, when one is needed.
///
/// An environment that already names one is respected; otherwise the
/// zenity dialog is offered, provided zenity is installed at all.
pub(super) fn askpass_helper() -> Option<std::path::PathBuf> {
    if let Some(existing) = std::env::var_os("SUDO_ASKPASS")
        && !existing.is_empty()
    {
        return Some(std::path::PathBuf::from(existing));
    }

    let path = std::env::var_os("PATH")?;

    if !std::env::split_paths(&path)
        .any(|dir| dir.join("rofi").exists() || dir.join("zenity").exists())
    {
        return None;
    }

    let dir = dirs::runtime_dir()
        .or_else(dirs::cache_dir)?
        .join("hydebar")
        .join("elevate");

    write_askpass(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_password_dialog_lands_executable() {
        let dir = std::env::temp_dir().join(format!(
            "hydebar-askpass-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));

        let helper = write_askpass(&dir).expect("the helper is written");

        let mode = {
            use std::os::unix::fs::PermissionsExt;

            std::fs::metadata(&helper)
                .expect("the helper exists")
                .permissions()
                .mode()
        };
        let content = std::fs::read_to_string(&helper).expect("the helper reads back");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(mode & 0o111, 0o111);
        assert!(content.contains("rofi -dmenu -password"));
        assert!(content.contains("zenity --password"));
    }
}
