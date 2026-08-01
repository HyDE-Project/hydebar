use std::time::Duration;

use serde::Deserialize;

/// Where the catalogue lives.
const INDEX_URL: &str = "https://raw.githubusercontent.com/HyDE-Project/hyde-gallery/hyde-gallery/hyde-themes.json";

/// How long a fetched catalogue serves before it is fetched again.
const CACHE_LIFE: Duration = Duration::from_hours(24);

/// One theme the gallery offers.
#[derive(Debug, Clone, PartialEq)]
pub struct GalleryTheme {
    pub name:        String,
    pub link:        String,
    pub owner:       String,
    pub description: String,
    /// The two colours the index announces the theme with.
    pub colors:      [iced::Color; 2]
}

/// One entry as the index spells it.
#[derive(Debug, Deserialize)]
struct Entry {
    #[serde(rename = "THEME")]
    theme:       String,
    #[serde(rename = "LINK")]
    link:        String,
    #[serde(rename = "OWNER", default)]
    owner:       String,
    #[serde(rename = "DESCRIPTION", default)]
    description: String,
    #[serde(rename = "COLORSCHEME", default)]
    colors:      Vec<String>
}

/// Parses the catalogue, which is several JSON arrays laid end to end.
///
/// The published file is not one document — strict parsing rejects it —
/// so the arrays are decoded in sequence and joined, and entries whose
/// colours do not parse are dropped rather than failing the rest.
pub fn parse(raw: &str) -> Vec<GalleryTheme> {
    let mut themes = Vec::new();

    for chunk in serde_json::Deserializer::from_str(raw).into_iter::<Vec<Entry>>() {
        let Ok(entries) = chunk else {
            break;
        };

        for entry in entries {
            let Some(colors) = announced_colors(&entry.colors) else {
                continue;
            };

            themes.push(GalleryTheme {
                name: entry.theme,
                link: entry.link,
                owner: entry.owner,
                description: entry.description,
                colors
            });
        }
    }

    themes
}

/// The two announced colours, when both spell valid hex.
fn announced_colors(colors: &[String]) -> Option<[iced::Color; 2]> {
    let first = hex(colors.first()?)?;
    let second = hex(colors.get(1)?)?;

    Some([first, second])
}

/// A colour as the index spells it.
fn hex(value: &str) -> Option<iced::Color> {
    let parsed = hex_color::HexColor::parse(value).ok()?;

    Some(iced::Color::from_rgb8(parsed.r, parsed.g, parsed.b))
}

/// Screenshots `HyDE`'s local gallery database keeps, by canonical theme name.
///
/// The database is written by the desktop's own gallery tooling — one
/// directory per theme with a `screenshot.png` of the desktop wearing it —
/// so the cards can show the real look from disk, no network involved.
pub(super) fn local_screenshots() -> std::collections::HashMap<String, std::path::PathBuf> {
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

            shot.is_file()
                .then(|| (super::view::canonical(&name), shot))
        })
        .collect()
}

/// The command the install runs, quoted for the shell.
pub fn import_command(name: &str, link: &str) -> String {
    format!(
        "hydectl theme import --name '{}' --url '{}'",
        name.replace('\'', "'\\''"),
        link.replace('\'', "'\\''")
    )
}

/// Where the fetched catalogue is kept between fetches.
fn cache_path() -> Option<std::path::PathBuf> {
    Some(dirs::cache_dir()?.join("hydebar").join("hyde-gallery.json"))
}

/// Whether the installer the import runs through exists at all.
async fn importer_present() -> bool {
    tokio::process::Command::new("hydectl")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .is_ok_and(|status| status.success())
}

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

/// The name this machine signs its work with.
///
/// Asked of git — the GitHub account name when one is configured, the
/// commit author name otherwise — so a gallery entry owned by the same
/// name can be marked as the user's own.
pub(super) async fn local_author() -> Option<String> {
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

/// Whatever the cache still holds, fresh or not.
async fn stale(cache: Option<&std::path::Path>) -> Vec<GalleryTheme> {
    match cache {
        Some(path) => match tokio::fs::read_to_string(path).await {
            Ok(raw) => parse(&raw),
            Err(_) => Vec::new()
        },
        None => Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_is_read_even_when_it_is_two_arrays_end_to_end() {
        let raw = r##"[{"THEME":"A","LINK":"l","OWNER":"o","DESCRIPTION":"d",
            "COLORSCHEME":["#111111","#222222"]}]
            [{"THEME":"B","LINK":"m","OWNER":"p","DESCRIPTION":"e",
            "COLORSCHEME":["#333333","#444444"]}]"##;

        let themes = parse(raw);

        assert_eq!(themes.len(), 2);
        assert_eq!(themes[0].name, "A");
        assert_eq!(themes[1].name, "B");
    }

    #[test]
    fn an_entry_with_broken_colours_is_dropped_alone() {
        let raw = r##"[{"THEME":"A","LINK":"l","OWNER":"o","DESCRIPTION":"d",
            "COLORSCHEME":["nope","#222222"]},
            {"THEME":"B","LINK":"m","OWNER":"p","DESCRIPTION":"e",
            "COLORSCHEME":["#333333","#444444"]}]"##;

        let themes = parse(raw);

        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].name, "B");
    }

    #[test]
    fn one_theme_is_recognised_under_every_spelling() {
        use crate::modules::themes::view::same_theme;

        assert!(same_theme("Catppuccin Mocha", "Catppuccin-Mocha"));
        assert!(same_theme("one_dark", "One Dark"));
        assert!(!same_theme("Tokyo Night", "Nordic Blue"));
    }

    #[test]
    fn the_import_command_survives_a_quoted_name() {
        let command = import_command("O'Dark", "https://x/y");

        assert!(command.starts_with("hydectl theme import --name "));
        assert!(command.contains("https://x/y"));
    }
}
