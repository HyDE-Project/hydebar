//! The catalogue fetch: the network read, the cache it serves from, and the
//! probe that decides whether fetching is worth anything at all.

use std::time::Duration;

use super::{GalleryTheme, index::parse};

/// Where the catalogue lives.
const INDEX_URL: &str =
    "https://raw.githubusercontent.com/HyDE-Project/hyde-gallery/hyde-gallery/hyde-themes.json";

/// How long a fetched catalogue serves before it is fetched again.
const CACHE_LIFE: Duration = Duration::from_hours(24);

/// Where the fetched catalogue is kept between fetches.
fn cache_path() -> Option<std::path::PathBuf> {
    Some(dirs::cache_dir()?.join("hydebar").join("hyde-gallery.json"))
}

/// Whether the installer the import runs through exists at all.
async fn importer_present() -> bool {
    if let Some(present) = PRESENT.get() {
        return *present;
    }

    let probe = tokio::process::Command::new("hydectl")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let probed = (tokio::time::timeout(std::time::Duration::from_secs(10), probe).await)
        .is_ok_and(|outcome| outcome.is_ok_and(|status| status.success()));

    PRESENT.get_or_init(|| probed).to_owned()
}

/// The one answer of the importer probe, asked once per process.
static PRESENT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Reads the catalogue, from the cache while it is fresh.
///
/// Every failure — no installer, no network, a bad response — answers
/// with an empty catalogue, and the window simply shows no gallery.
pub async fn load() -> Vec<GalleryTheme> {
    if !importer_present().await {
        return Vec::new();
    }

    let cache = cache_path();

    if let Some(path) = &cache
        && let Ok(metadata) = tokio::fs::metadata(path).await
        && metadata
            .modified()
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < CACHE_LIFE)
        && let Ok(raw) = tokio::fs::read_to_string(path).await
    {
        let themes = parse(&raw);

        if !themes.is_empty() {
            return themes;
        }
    }

    let Ok(response) = crate::utils::http_client().get(INDEX_URL).send().await else {
        return stale(cache.as_deref()).await;
    };
    let Ok(raw) = response.text().await else {
        return stale(cache.as_deref()).await;
    };

    let themes = parse(&raw);

    if themes.is_empty() {
        return stale(cache.as_deref()).await;
    }

    if let Some(path) = &cache {
        if let Some(dir) = path.parent() {
            let _ = tokio::fs::create_dir_all(dir).await;
        }
        let _ = tokio::fs::write(path, &raw).await;
    }

    themes
}

/// Whatever the cache still holds, fresh or not.
async fn stale(cache: Option<&std::path::Path>) -> Vec<GalleryTheme> {
    let Some(path) = cache else {
        return Vec::new();
    };

    tokio::fs::read_to_string(path)
        .await
        .map_or_else(|_| Vec::new(), |raw| parse(&raw))
}
