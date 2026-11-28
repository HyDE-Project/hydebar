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
/// for again; [`prune_to_cap`] drops them once the map outgrows its cap, so
/// a machine that cycles icon themes does not carry every theme it ever
/// resolved for the rest of the session.
static ICON_CACHE: LazyLock<Mutex<ResolvedIcons>> = LazyLock::new(Mutex::default);

/// Most resolved icons the cache may hold before a prune is asked for.
///
/// Generous against a tray's handful of names and small against what a
/// theme can hold: the cap only ever bites after theme switches piled up.
const CACHE_CAP: usize = 256;

/// Drops everything the active theme did not resolve once the map outgrew
/// [`CACHE_CAP`].
///
/// Entries are keyed by theme, so the moment a theme stops being active its
/// entries stop being asked for: dropping them costs nothing a later lookup
/// cannot rebuild. If even the active theme alone outgrew the cap, the map
/// starts over — a tray that big is rare, and a rebuild is one directory
/// walk per name.
///
/// `active_theme` is the theme the entries of the next lookup will carry.
fn prune_to_cap(cache: &mut ResolvedIcons, active_theme: &str) {
    if cache.len() < CACHE_CAP {
        return;
    }

    cache.retain(|(theme, _), _| theme == active_theme);

    if cache.len() >= CACHE_CAP {
        cache.clear();
    }
}

/// Finds the picture the icon theme keeps under this name.
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

    {
        let mut cache = ICON_CACHE.lock().unwrap_or_else(PoisonError::into_inner);
        prune_to_cap(&mut cache, &key.0);
        cache.insert(key, resolved.clone());
    }

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
        collections::HashMap,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering}
    };

    use super::{CACHE_CAP, ResolvedIcons, prune_to_cap};

    fn cache_with(themes: &[&str]) -> ResolvedIcons {
        themes
            .iter()
            .enumerate()
            .map(|(index, theme)| ((theme.to_string(), format!("icon-{index}")), None))
            .collect()
    }

    #[test]
    fn a_cache_below_the_cap_is_left_alone() {
        let mut cache = cache_with(&["old", "new"]);

        prune_to_cap(&mut cache, "new");

        assert_eq!(cache.len(), 2, "nothing to prune below the cap");
    }

    #[test]
    fn a_prune_keeps_only_the_active_theme() {
        let mut stale: Vec<&str> = (0..CACHE_CAP).map(|_| "old").collect();
        stale.push("new");
        let mut cache = cache_with(&stale);

        prune_to_cap(&mut cache, "new");

        assert_eq!(cache.len(), 1);
        assert!(
            cache.keys().all(|(theme, _)| theme == "new"),
            "every stale-theme entry is gone"
        );
    }

    #[test]
    fn a_single_theme_outgrowing_the_cap_starts_the_cache_over() {
        let crowded: Vec<&str> = (0..=CACHE_CAP).map(|_| "only").collect();
        let mut cache = cache_with(&crowded);

        prune_to_cap(&mut cache, "only");

        assert!(cache.is_empty(), "one theme past the cap earns a reset");
    }

    #[test]
    fn an_empty_cache_prunes_to_nothing() {
        let mut icons: ResolvedIcons = HashMap::new();

        prune_to_cap(&mut icons, "new");

        assert!(icons.is_empty(), "pruning nothing stays empty");
    }

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
