//! Commands the theme module drives the `HyDE` desktop with.
//!
//! `HyDE` exposes every desktop action through the `hyde-shell` dispatcher, so
//! the bar asks for a change the same way the user's own keybindings do instead
//! of writing `HyDE`'s state files behind its back. That keeps the whole switch
//! — wallpaper, colours, the other clients — in `HyDE`'s hands, and it means
//! the bar stays correct when `HyDE` changes how a switch is performed.
//!
//! A theme switch is the one exception. It goes through the bar's own script
//! (see [`theme_script`]), which runs the very same `HyDE` switch but leaves
//! waybar's wallbash template out of it, so the bar that replaced waybar does
//! not pay for restyling and restarting it on every switch.
//!
//! The commands are built here, as plain strings, so the shape of an invocation
//! can be checked without a `HyDE` install present.
//!
//! Running them lives here too, in [`run`], because every caller wants the same
//! thing out of a desktop command: to hear that it ended, and why it failed
//! when it did.

mod theme_script;

use std::{path::Path, sync::Arc};

pub use theme_script::ScriptNotFound;

use crate::utils::launcher;

/// Command switching the desktop to `theme`.
///
/// # Errors
///
/// Returns [`ScriptNotFound`] when the switch script is not installed, in which
/// case no command is issued at all — a half-performed switch would be worse
/// than none.
pub fn switch_theme(theme: &str) -> Result<String, ScriptNotFound> {
    let script = theme_script::locate()?;

    Ok(switch_theme_with(&script, theme))
}

/// Command running `script` for `theme`.
///
/// Theme names carry spaces and accents (`Rosé Pine`), so the name is quoted
/// rather than interpolated: unquoted, the script would see a theme named
/// `Rosé` followed by a stray argument. `--` closes the option list, so a theme
/// whose name starts with a dash is still a theme name. The script's own path
/// is quoted for the same reason — it is discovered, not spelled out, and may
/// sit under a directory with a space in it.
fn switch_theme_with(script: &Path, theme: &str) -> String {
    format!("{} -- {}", quote(&script.to_string_lossy()), quote(theme))
}

/// Command moving the desktop to the next wallpaper of the active theme.
#[must_use]
pub fn next_wallpaper() -> String {
    "hyde-shell wallpaper --next".to_owned()
}

/// Command asking the desktop for the previous wallpaper of the theme in force.
#[must_use]
pub fn previous_wallpaper() -> String {
    "hyde-shell wallpaper --previous".to_owned()
}

/// Command stepping the desktop to the next bar layout.
///
/// The desktop's tool restarts its own bar as part of every switch; this
/// bar is the bar here, so the switch is followed by putting that one back
/// down. The state record survives, and this bar re-reads it on its watch.
#[must_use]
pub fn next_bar_layout() -> String {
    "hyde-shell waybar --next && hyde-shell waybar --kill".to_owned()
}

/// Command stepping the desktop to the previous bar layout.
#[must_use]
pub fn previous_bar_layout() -> String {
    "hyde-shell waybar --prev && hyde-shell waybar --kill".to_owned()
}

/// Command arranging the bar by the named layout.
#[must_use]
pub fn set_bar_layout(name: &str) -> String {
    format!(
        "hyde-shell waybar --set {} && hyde-shell waybar --kill",
        quote(name)
    )
}

/// Wraps `value` so a shell passes it on as a single, literal argument.
///
/// Single quotes are used because they suppress every expansion; the only
/// character they cannot carry is the single quote itself, which is spliced
/// back in the way a shell requires.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Runs a desktop command, reporting why it failed if it did.
///
/// The command is run detached rather than with its output collected: a `HyDE`
/// switch leaves background children behind that would keep a collected stream
/// open long after the switch itself is over, and the bar has to hear that the
/// switch ended when it ends. What the command printed goes to the bar's own
/// log, which is where the detail belongs.
pub async fn run(command: String) -> Option<String> {
    let command: Arc<str> = Arc::from(command);

    log::info!("desktop command runs: {command}");

    launcher::run_detached(&command)
        .await
        .err()
        .map(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_name_is_passed_as_one_quoted_argument() {
        assert_eq!(
            switch_theme_with(Path::new("/usr/bin/hydebar-theme-switch"), "Gruvbox Retro"),
            "'/usr/bin/hydebar-theme-switch' -- 'Gruvbox Retro'"
        );
    }

    #[test]
    fn an_accented_theme_name_survives_quoting() {
        assert_eq!(
            switch_theme_with(Path::new("/usr/bin/hydebar-theme-switch"), "Rosé Pine"),
            "'/usr/bin/hydebar-theme-switch' -- 'Rosé Pine'"
        );
    }

    #[test]
    fn a_theme_name_starting_with_a_dash_stays_a_theme_name() {
        assert_eq!(
            switch_theme_with(Path::new("/usr/bin/hydebar-theme-switch"), "-l"),
            "'/usr/bin/hydebar-theme-switch' -- '-l'"
        );
    }

    #[test]
    fn a_script_path_with_a_space_stays_one_argument() {
        assert_eq!(
            switch_theme_with(Path::new("/opt/my bar/theme-switch"), "Tokyo Night"),
            "'/opt/my bar/theme-switch' -- 'Tokyo Night'"
        );
    }

    #[test]
    fn a_quote_inside_a_theme_name_cannot_escape_the_argument() {
        assert_eq!(quote("it's; rm -rf /"), r"'it'\''s; rm -rf /'");
    }

    #[test]
    fn the_wallpaper_command_asks_for_the_next_one() {
        assert_eq!(next_wallpaper(), "hyde-shell wallpaper --next");
        assert_eq!(previous_wallpaper(), "hyde-shell wallpaper --previous");
    }
}
