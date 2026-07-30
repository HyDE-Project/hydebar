//! Per-glyph size correction, read out of the font that draws the glyph.
//!
//! The symbol font gathers icons drawn by different hands: one glyph inks
//! nearly its whole box, its neighbour barely half. Stated at one common size
//! they render as visibly different sizes, and no table of ranges can fix it —
//! the spread exists inside every range. The only honest source is the font
//! itself: the bounding box of the glyph's outline says exactly how much of
//! the box it inks, and dividing that out renders every icon at one apparent
//! size.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex}
};

use fontdb::Database;

/// First codepoint of the private use area the icon fonts live in.
///
/// Ordinary text is left untouched: its sizes are the typographer's business,
/// and only the pictographic glyphs need evening out.
const PRIVATE_USE_START: char = '\u{E000}';

/// Share of its box the average icon inks, and the share every icon is
/// brought to.
///
/// The median of the symbol font shipped with the reference desktop; picking
/// the middle keeps the correction factors close to one.
const TARGET_INK_EM: f32 = 0.85;

/// Correction bounds, so a decorative outlier cannot explode its stated size.
const MIN_FACTOR: f32 = 0.7;
const MAX_FACTOR: f32 = 1.6;

/// Every font the system knows, loaded once.
static FONTS: LazyLock<Database> = LazyLock::new(|| {
    let mut db = Database::new();
    db.load_system_fonts();
    db
});

/// Ink shares already measured, one entry per distinct glyph.
static MEASURED: LazyLock<Mutex<HashMap<char, f32>>> = LazyLock::new(Mutex::default);

/// Size to state `glyph` at so its ink comes out at the icon size `base`.
pub(super) fn stated_size(glyph: &str, base: f32) -> f32 {
    let Some(first) = glyph.chars().next() else {
        return base;
    };

    if first < PRIVATE_USE_START {
        return base;
    }

    let share = *MEASURED
        .lock()
        .expect("ink share cache poisoned")
        .entry(first)
        .or_insert_with(|| measured_share(first));

    base * (TARGET_INK_EM / share).clamp(MIN_FACTOR, MAX_FACTOR)
}

/// Share of its box the glyph for `symbol` inks, read from the first system
/// font that carries it.
///
/// The first carrier is the one the renderer's own fallback lands on for a
/// glyph the text font lacks, which is exactly the case for every icon.
fn measured_share(symbol: char) -> f32 {
    FONTS
        .faces()
        .find_map(|face| {
            FONTS
                .with_face_data(face.id, |data, index| {
                    let face = ttf_parser::Face::parse(data, index).ok()?;
                    let glyph = face.glyph_index(symbol)?;
                    let bounds = face.glyph_bounding_box(glyph)?;

                    Some(f32::from(bounds.y_max - bounds.y_min) / f32::from(face.units_per_em()))
                })
                .flatten()
        })
        .unwrap_or(TARGET_INK_EM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_text_is_left_alone() {
        assert_eq!(stated_size("5", 20.0), 20.0);
        assert_eq!(stated_size("", 20.0), 20.0);
    }

    #[test]
    fn a_glyph_is_never_stated_outside_the_bounds() {
        let size = stated_size("\u{eb94}", 20.0);

        assert!(size >= 20.0 * MIN_FACTOR);
        assert!(size <= 20.0 * MAX_FACTOR);
    }

    #[test]
    fn the_same_glyph_is_measured_once_and_reused() {
        let first = stated_size("\u{f011}", 20.0);
        let second = stated_size("\u{f011}", 20.0);

        assert_eq!(first, second);
    }
}
