//! Per-glyph font choice and size correction, resolved through fontconfig.
//!
//! The icon glyphs live in the private use area, and half a dozen installed
//! fonts may carry any one of them — regular symbol fonts drawn to fill the
//! box, mono variants scaled down into one cell. Which one the renderer's own
//! fallback lands on is a lottery, and a size correction measured from a
//! different face than the one that draws is worse than none. So both
//! decisions are taken in one place: fontconfig names the face for the glyph,
//! exactly the way the reference waybar resolves its icons, and the ink share
//! is measured from that same file. The two cannot disagree.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex}
};

use iced::Font;
use log::debug;

/// First codepoint of the private use area the icon fonts live in.
///
/// Ordinary text is left untouched: its sizes are the typographer's business,
/// and only the pictographic glyphs need evening out.
const PRIVATE_USE_START: char = '\u{E000}';

/// Share of its box the average icon inks, and the share every icon is
/// brought to.
const TARGET_INK_EM: f32 = 0.85;

/// Correction bounds, so a decorative outlier cannot explode its stated size.
const MIN_FACTOR: f32 = 0.7;
const MAX_FACTOR: f32 = 1.8;

/// The face a glyph is drawn with, and the size correction it calls for.
#[derive(Clone, Copy)]
pub(super) struct IconFace {
    /// Font the glyph is stated in, when fontconfig named one.
    pub(super) font:   Option<Font>,
    /// Factor the wanted size is multiplied by so the ink reaches it.
    pub(super) factor: f32
}

impl IconFace {
    const PLAIN: Self = Self {
        font:   None,
        factor: 1.0
    };
}

/// Faces already resolved, one entry per distinct glyph.
static RESOLVED: LazyLock<Mutex<HashMap<char, IconFace>>> = LazyLock::new(Mutex::default);

/// Families already leaked for the renderer, which wants them immortal.
static FAMILIES: LazyLock<Mutex<HashMap<String, &'static str>>> = LazyLock::new(Mutex::default);

/// Face and size correction for `glyph`.
pub(super) fn resolved(glyph: &str) -> IconFace {
    let Some(first) = glyph.chars().next() else {
        return IconFace::PLAIN;
    };

    if first < PRIVATE_USE_START {
        return IconFace::PLAIN;
    }

    *RESOLVED
        .lock()
        .expect("icon face cache poisoned")
        .entry(first)
        .or_insert_with(|| resolve(first))
}

/// Asks fontconfig for the face carrying `symbol` and measures its ink.
fn resolve(symbol: char) -> IconFace {
    let Some((family, file, index)) = fontconfig_match(symbol) else {
        return IconFace::PLAIN;
    };

    let factor = match measured_share(symbol, &file, index) {
        Some(share) => (TARGET_INK_EM / share).clamp(MIN_FACTOR, MAX_FACTOR),
        None => 1.0
    };

    debug!(
        "glyph U+{:04X} drawn by `{family}` at {factor:.2}",
        u32::from(symbol)
    );

    IconFace {
        font: Some(Font::with_name(immortal(family))),
        factor
    }
}

/// Best face for `symbol` by the system's own fallback rules.
fn fontconfig_match(symbol: char) -> Option<(String, String, u32)> {
    let output = std::process::Command::new("fc-match")
        .arg("-f")
        .arg("%{family[0]}\n%{file}\n%{index}")
        .arg(format!(":charset={:x}", u32::from(symbol)))
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let family = lines.next()?.trim().to_owned();
    let file = lines.next()?.trim().to_owned();
    let index = lines
        .next()
        .and_then(|line| line.trim().parse().ok())
        .unwrap_or(0);

    if family.is_empty() || file.is_empty() {
        return None;
    }

    Some((family, file, index))
}

/// Share of its box the glyph for `symbol` inks in the named face.
fn measured_share(symbol: char, file: &str, index: u32) -> Option<f32> {
    let data = std::fs::read(file).ok()?;
    let face = ttf_parser::Face::parse(&data, index).ok()?;
    let glyph = face.glyph_index(symbol)?;
    let bounds = face.glyph_bounding_box(glyph)?;

    Some(f32::from(bounds.y_max - bounds.y_min) / f32::from(face.units_per_em()))
}

/// The family name with the lifetime the renderer demands.
///
/// Each distinct family is leaked once and reused; a handful of icon fonts
/// exist on any system, so the leak is bounded.
fn immortal(family: String) -> &'static str {
    let mut families = FAMILIES.lock().expect("family cache poisoned");

    if let Some(&name) = families.get(&family) {
        return name;
    }

    let name: &'static str = Box::leak(family.clone().into_boxed_str());
    families.insert(family, name);

    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_text_is_left_alone() {
        assert!(resolved("5").font.is_none());
        assert_eq!(resolved("5").factor, 1.0);
        assert!(resolved("").font.is_none());
    }

    #[test]
    fn a_glyph_is_never_corrected_outside_the_bounds() {
        let face = resolved("\u{eb94}");

        assert!(face.factor >= MIN_FACTOR);
        assert!(face.factor <= MAX_FACTOR);
    }

    #[test]
    fn the_same_glyph_resolves_once_and_reuses() {
        let first = resolved("\u{f011}").factor;
        let second = resolved("\u{f011}").factor;

        assert_eq!(first, second);
    }
}
