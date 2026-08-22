//! Building one shade of the palette out of what an entry names.
//!
//! An appearance entry may name a weak shade, a strong one and a text colour,
//! and leave the rest to the toolkit. These builders take what was named and
//! fall back to the shade the toolkit generated for everything else, so a
//! half-written entry lands in a complete palette rather than a blank one.

use iced::{Color, theme::palette};

use crate::{config::AppearanceColor, style::color::readable_pair};

/// The weak shade of an entry, paired with the text that has to read on it.
pub(super) fn weak_pair(color: &AppearanceColor, text_fallback: Color) -> Option<palette::Pair> {
    color
        .get_weak()
        .map(|weak| readable_pair(weak, color.get_text(), text_fallback))
}

/// The strong shade of an entry, paired with the text that has to read on it.
pub(super) fn strong_pair(color: &AppearanceColor, text_fallback: Color) -> Option<palette::Pair> {
    color
        .get_strong()
        .map(|strong| readable_pair(strong, color.get_text(), text_fallback))
}

pub(super) fn build_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    base: palette::Pair,
    _default_weak: palette::Pair,
    _default_strong: palette::Pair
) -> palette::Background {
    let mut bg = palette::Background::new(base.color, base.text);
    if let Some(weak) = weak_pair(color, text_fallback) {
        bg.weak = weak;
    }
    if let Some(strong) = strong_pair(color, text_fallback) {
        bg.strong = strong;
    }
    bg
}

pub(super) fn build_primary_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    defaults: palette::Primary
) -> palette::Primary {
    palette::Primary {
        base:   defaults.base,
        weak:   weak_pair(color, text_fallback).unwrap_or(defaults.weak),
        strong: strong_pair(color, text_fallback).unwrap_or(defaults.strong)
    }
}

pub(super) fn build_secondary_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    defaults: palette::Primary
) -> palette::Secondary {
    palette::Secondary {
        base:   defaults.base,
        weak:   weak_pair(color, text_fallback).unwrap_or(defaults.weak),
        strong: strong_pair(color, text_fallback).unwrap_or(defaults.strong)
    }
}

pub(super) fn build_success_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    defaults: palette::Success
) -> palette::Success {
    palette::Success {
        base:   defaults.base,
        weak:   weak_pair(color, text_fallback).unwrap_or(defaults.weak),
        strong: strong_pair(color, text_fallback).unwrap_or(defaults.strong)
    }
}

pub(super) fn build_danger_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    defaults: palette::Danger
) -> palette::Danger {
    palette::Danger {
        base:   defaults.base,
        weak:   weak_pair(color, text_fallback).unwrap_or(defaults.weak),
        strong: strong_pair(color, text_fallback).unwrap_or(defaults.strong)
    }
}
