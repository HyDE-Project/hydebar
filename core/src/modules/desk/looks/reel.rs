//! A reel of pictures, centred on the one in force.
//!
//! The shape a picker takes when it has one row and no scrollbar: the picture
//! in force stands in the middle at full size, its neighbours narrow away from
//! it to either side, and the row is a ring — walk far enough either way and
//! the first picture comes round again, which is exactly what the desktop's
//! own next and previous keys do.

use std::path::{Path, PathBuf};

use iced::widget::image::Handle;

/// How many pictures stand on either side of the one in force.
///
/// Three: enough that the row reads as a row rather than as one picture with
/// a frame, few enough that the narrow ones stay wide enough to be pictures.
pub const REACH: usize = 3;

/// Longest edge a slide is decoded to, in pixels.
///
/// The widest a slide is ever drawn is about half a column, and a column is a
/// third of a screen; anything sharper than this is pixels decoded to be
/// thrown away.
const SIDE: u32 = 256;

/// One picture of a reel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    /// What the desktop calls it, which is what the user calls it.
    pub name:    String,
    /// The picture itself, decoded ready to draw.
    pub picture: Handle,
    /// Whether this is the one in force.
    pub active:  bool
}

/// A row of pictures with the one in force in the middle of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reel {
    /// The slides the row draws, in drawing order.
    pub shown: Vec<Slide>,
    /// Which of all of them is in force, counted from one.
    pub at:    usize,
    /// How many there are altogether, shown or not.
    pub total: usize
}

impl Reel {
    /// Whether the reel has anything to draw.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.shown.is_empty()
    }

    /// The name of the picture in force, when the reel holds one.
    #[must_use]
    pub fn in_force(&self) -> Option<&str> {
        self.shown
            .iter()
            .find(|slide| slide.active)
            .map(|slide| slide.name.as_str())
    }
}

/// Builds the reel `all` makes when the picture at `at` is the one in force.
///
/// Only the slides the row will draw are decoded: a machine with a hundred
/// wallpapers pays for seven pictures, not a hundred, and pays again only when
/// the wallpaper moves.
pub(super) fn reel(all: &[(String, PathBuf)], at: usize) -> Reel {
    let shown = centred(all.len(), at, REACH)
        .into_iter()
        .filter_map(|index| {
            let (name, path) = all.get(index)?;

            Some(Slide {
                name:    name.clone(),
                picture: decode(path)?,
                active:  index == at
            })
        })
        .collect();

    Reel {
        shown,
        at: at.saturating_add(1).min(all.len()),
        total: all.len()
    }
}

/// Decodes one slide down to the size a reel draws it at.
fn decode(path: &Path) -> Option<Handle> {
    crate::modules::wallpaper::current::thumbnail(path, SIDE)
}

/// Which of `total` things a row of `reach` either side draws, with `at` in
/// the middle of them.
///
/// The row is a ring, so a thing near either end of the list is still drawn in
/// the middle with neighbours either side — the list has no ends, the same way
/// the desktop's own next and previous keys have none. It never repeats one: a
/// short list is drawn whole, rotated so the one in force stands in the middle
/// of it.
///
/// Answered here rather than at each drawing so every row of the canvas —
/// themes, wallpapers, the workspaces of a screen — is centred the same way.
#[must_use]
pub fn centred(total: usize, at: usize, reach: usize) -> Vec<usize> {
    if total == 0 || at >= total {
        return Vec::new();
    }

    let shown = (reach * 2 + 1).min(total);
    let half = shown / 2;
    let first = (at + total - half) % total;

    (0..shown).map(|step| (first + step) % total).collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_one_in_force_stands_in_the_middle_of_a_long_list() {
        let drawn = centred(20, 10, REACH);

        assert_eq!(drawn.len(), REACH * 2 + 1);
        assert_eq!(drawn[REACH], 10);
    }

    /// The list has no ends: the picture before the first is the last one.
    #[test]
    fn the_row_is_a_ring() {
        let drawn = centred(20, 0, REACH);

        assert_eq!(drawn[REACH], 0);
        assert_eq!(drawn[REACH - 1], 19);
        assert_eq!(drawn[REACH + 1], 1);
    }

    #[test]
    fn a_short_list_is_drawn_whole_and_never_twice() {
        let drawn = centred(4, 3, REACH);
        let mut seen = drawn.clone();
        seen.sort_unstable();
        seen.dedup();

        assert_eq!(drawn.len(), 4);
        assert_eq!(seen.len(), 4, "a picture is drawn once: {drawn:?}");
        assert_eq!(drawn[4 / 2], 3);
    }

    #[test]
    fn a_list_of_one_draws_that_one() {
        assert_eq!(centred(1, 0, REACH), vec![0]);
    }

    #[test]
    fn nothing_is_drawn_for_nothing() {
        assert!(centred(0, 0, REACH).is_empty());
        assert!(centred(3, 7, REACH).is_empty());
    }
}
