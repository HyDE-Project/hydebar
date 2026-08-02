//! Text at the themed size, wherever no other size is stated.
//!
//! The windowing layer renders unsized text at its own built-in default and
//! offers no way to change it. The bar's size lives in the theme and moves
//! with every reload, so text is created through this helper instead: it
//! reads the size in force at draw time — see [`scale::base`] — and a caller
//! that wants a different size still chains its own on top.

use iced::widget::{Text, text as raw_text};
use iced_core::text::IntoFragment;

use super::scale;

/// Text at the themed size in force.
pub fn text<'a>(fragment: impl IntoFragment<'a>) -> Text<'a> {
    raw_text(fragment).size(scale::base())
}
