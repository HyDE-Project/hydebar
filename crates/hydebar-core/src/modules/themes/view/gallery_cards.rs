//! The gallery section: cards for the themes the desktop could install.

use std::collections::HashMap;

use iced::Element;

use super::{canonical, sizing::card_rows};
use crate::{
    components::{
        icons::Icons,
        page::widgets::{ChipPaint, ThemeChip, grid, group, theme_chip}
    },
    modules::themes::{Message, Spinner, gallery::GalleryTheme}
};

/// Renders the gallery as a grid of chips painted in announced colours.
///
/// One install at a time, and none beside a running switch: a chip being
/// installed carries the spinner, and every other chip waits unpressable
/// rather than starting a second writer over the same directories.
#[expect(
    clippy::too_many_arguments,
    reason = "view helper mirrors the fields of the state it renders"
)]
pub(super) fn offer<'a>(
    names: &[String],
    catalogue: &[GalleryTheme],
    catalogue_index: &HashMap<String, usize>,
    screenshots: &HashMap<String, std::path::PathBuf>,
    author: Option<&str>,
    switching: Option<&str>,
    installing: Option<&str>,
    spinner: Spinner,
    opacity: f32,
    font_size: f32,
    available_width: f32,
    cell: f32,
    list_layout: bool
) -> Element<'a, Message> {
    let busy = switching.is_some() || installing.is_some();
    let mut block = grid(font_size);

    for indices in card_rows(names, list_layout, font_size, available_width) {
        let mut row = group(font_size);

        for index in indices {
            let name = &names[index];
            let entry = catalogue_index
                .get(&canonical(name))
                .map(|&index| &catalogue[index]);

            let chip_look = if installing == Some(name.as_str()) {
                ThemeChip::Applying(spinner)
            } else {
                ThemeChip::Inert
            };

            row = row.push(theme_chip(
                name.clone(),
                authored_badge(entry, author),
                Message::Install(name.clone()),
                chip_look,
                font_size,
                opacity,
                cell,
                entry.map(offer_paint),
                screenshots.get(&canonical(name)).cloned(),
                vec![(
                    Icons::Download.default_glyph(),
                    Message::Install(name.clone()),
                    !busy
                )],
                list_layout
            ));
        }

        block = block.push(row);
    }

    block.into()
}

/// The mark a card earns when its theme is the user's own work.
///
/// Ownership comes from the gallery index, and "the user" is whoever the
/// git identity of this machine names — the one signal that is already
/// there and already theirs.
pub(super) fn authored_badge(
    entry: Option<&GalleryTheme>,
    author: Option<&str>
) -> Option<&'static str> {
    let owner = entry.map(|entry| entry.owner.as_str())?;
    let author = author?;

    owner
        .eq_ignore_ascii_case(author)
        .then(|| Icons::Authored.default_glyph())
}

/// Paint for a gallery chip, from the two colours the index announces.
///
/// The index does not promise an order, so the darker of the two is taken
/// as the surface and the lighter as the ink — a chip the other way round
/// would be a swatch nobody can read.
pub(super) fn offer_paint(entry: &GalleryTheme) -> ChipPaint {
    let luma = |color: iced::Color| {
        0.0722f32.mul_add(color.b, 0.7152f32.mul_add(color.g, 0.2126 * color.r))
    };

    let [first, second] = entry.colors;
    let (surface, ink) = if luma(first) <= luma(second) {
        (first, second)
    } else {
        (second, first)
    };

    ChipPaint {
        background: surface,
        text:       ink,
        accent:     ink,
        palette:    entry.colors.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::{GalleryTheme, authored_badge};

    fn entry(owner: &str) -> GalleryTheme {
        GalleryTheme {
            name:        "One Dark".to_owned(),
            link:        String::new(),
            owner:       owner.to_owned(),
            description: String::new(),
            colors:      [iced::Color::BLACK, iced::Color::WHITE]
        }
    }

    #[test]
    fn a_theme_of_the_local_author_is_marked() {
        let theme = entry("RAprogramm");

        assert!(authored_badge(Some(&theme), Some("raprogramm")).is_some());
    }

    #[test]
    fn foreign_and_unknown_work_stays_unmarked() {
        let theme = entry("someone-else");

        assert!(authored_badge(Some(&theme), Some("raprogramm")).is_none());
        assert!(authored_badge(Some(&theme), None).is_none());
        assert!(authored_badge(None, Some("raprogramm")).is_none());
    }
}
