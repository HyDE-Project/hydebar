//! Cached answer about the current icon theme.

use std::{
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant}
};

use linicon_theme::get_icon_theme;

/// How long one answer about the current icon theme stays fresh.
///
/// Asking means reading desktop settings files; a chatty tray application
/// must not turn that into a file walk per signal. Five seconds keeps a
/// theme switch visible on the next icon change without the walk.
const THEME_FRESHNESS: Duration = Duration::from_secs(5);

/// One answer about the icon theme: when it was read and what it said.
type ThemeAnswer = (Instant, Option<String>);

/// The icon theme last read from the desktop settings, with its read time.
static THEME_CACHE: LazyLock<Mutex<Option<ThemeAnswer>>> = LazyLock::new(Mutex::default);

/// The current icon theme, read from the settings at most once per window.
pub(super) fn cached_icon_theme() -> Option<String> {
    let mut cache = THEME_CACHE.lock().unwrap_or_else(PoisonError::into_inner);

    if let Some((asked, theme)) = cache.as_ref()
        && asked.elapsed() < THEME_FRESHNESS
    {
        return theme.clone();
    }

    let theme = get_icon_theme();
    *cache = Some((Instant::now(), theme.clone()));

    theme
}
