//! Row shapes shared by the pages of the settings window.
//!
//! Every shape reads its sizes from [`super::style`] and every caller passes
//! the page text size rather than a scaled one, so a row drawn on one tab is
//! the same height, carries the same text size and starts its controls at the
//! same x as the matching row on the next tab.
//!
//! The seams follow what the shapes do: [`scaffold`] frames a page,
//! [`controls`] holds the rows that act, and the theme card is spread over
//! [`theme_card`] for what it knows, [`theme_faces`] for how it looks and
//! [`theme_chip`] for the card itself.

mod controls;
mod scaffold;
mod theme_card;
mod theme_chip;
mod theme_faces;

pub use self::{
    controls::{chip, choice_button, choice_row, status_row, stepper_row},
    scaffold::{card, grid, group, labelled_row, note, outlined, page, rows, section},
    theme_card::{ChipPaint, DOT_ROW_EM, ThemeChip},
    theme_chip::theme_chip
};
