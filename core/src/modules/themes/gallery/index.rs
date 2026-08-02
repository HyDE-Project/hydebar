//! The catalogue index as the gallery publishes it, and the parser that
//! reads it.

use serde::Deserialize;

use super::GalleryTheme;

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
pub(super) fn parse(raw: &str) -> Vec<GalleryTheme> {
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
}
