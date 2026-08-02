//! Named icon lookup through the theme directories, memoised per theme.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, PoisonError}
};

use freedesktop_icons::lookup;
use iced::widget::svg;
use log::debug;

use super::{
    super::TrayIcon,
    raster::{rasterized_svg, trimmed_raster},
    theme::cached_icon_theme
};

/// A resolved icon under its theme-and-name key.
type ResolvedIcons = HashMap<(String, String), Option<TrayIcon>>;

/// Icons already resolved, keyed by the theme and the icon name.
///
/// A theme switch changes the key, so stale entries are simply never asked
/// for again; the map stays small — a tray holds a handful of names.
static ICON_CACHE: LazyLock<Mutex<ResolvedIcons>> = LazyLock::new(Mutex::default);

pub fn icon_from_name(icon_name: &str) -> Option<TrayIcon> {
    let theme = cached_icon_theme();
    let key = (theme.clone().unwrap_or_default(), icon_name.to_owned());

    if let Some(hit) = ICON_CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
    {
        return hit.clone();
    }

    let resolved = resolve_icon(theme.as_deref(), icon_name);

    ICON_CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, resolved.clone());

    resolved
}

/// Walks the theme directories and decodes the named icon.
fn resolve_icon(theme: Option<&str>, icon_name: &str) -> Option<TrayIcon> {
    debug!("resolving icon from name {icon_name}");

    let icon_path = theme.map_or_else(
        || lookup(icon_name).with_cache().find(),
        |theme_name| {
            lookup(icon_name)
                .with_cache()
                .with_theme(theme_name)
                .find()
                .or_else(|| lookup(icon_name).with_cache().find())
        }
    )?;

    if icon_path.extension().is_some_and(|ext| ext == "svg") {
        Some(
            rasterized_svg(&icon_path)
                .unwrap_or_else(|| TrayIcon::Svg(svg::Handle::from_path(icon_path)))
        )
    } else {
        Some(trimmed_raster(&icon_path))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering}
    };

    fn icon_path_with_theme_fallback<F, G>(
        theme: Option<&str>,
        mut themed_lookup: F,
        mut default_lookup: G
    ) -> Option<std::path::PathBuf>
    where
        F: FnMut(&str) -> Option<std::path::PathBuf>,
        G: FnMut() -> Option<std::path::PathBuf>
    {
        if let Some(theme_name) = theme
            && let Some(path) = themed_lookup(theme_name)
        {
            return Some(path);
        }

        default_lookup()
    }

    #[test]
    fn uses_theme_when_available() {
        let theme_calls = AtomicUsize::new(0);
        let default_calls = AtomicUsize::new(0);

        let expected = PathBuf::from("/tmp/themed.svg");

        let result = icon_path_with_theme_fallback(
            Some("test"),
            |_| {
                theme_calls.fetch_add(1, Ordering::Relaxed);
                Some(expected.clone())
            },
            || {
                default_calls.fetch_add(1, Ordering::Relaxed);
                Some(PathBuf::from("/tmp/default.svg"))
            }
        );

        assert_eq!(theme_calls.load(Ordering::Relaxed), 1);
        assert_eq!(default_calls.load(Ordering::Relaxed), 0);
        assert_eq!(result.as_deref(), Some(expected.as_path()));
    }

    #[test]
    fn falls_back_to_default_when_theme_missing() {
        let theme_calls = AtomicUsize::new(0);
        let default_calls = AtomicUsize::new(0);

        let expected = PathBuf::from("/tmp/default.svg");

        let result = icon_path_with_theme_fallback(
            Some("test"),
            |_| {
                theme_calls.fetch_add(1, Ordering::Relaxed);
                None
            },
            || {
                default_calls.fetch_add(1, Ordering::Relaxed);
                Some(expected.clone())
            }
        );

        assert_eq!(theme_calls.load(Ordering::Relaxed), 1);
        assert_eq!(default_calls.load(Ordering::Relaxed), 1);
        assert_eq!(result.as_deref(), Some(expected.as_path()));
    }
}
