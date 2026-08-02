//! The bar's theme: its palette, its fades and its colour effects.
//!
//! The palette derivation from the configured appearance lives in
//! [`palette`], the whole-theme alpha fade in [`fade`] and the backdrop,
//! darkening and text-input helpers in [`effects`].

mod effects;
mod fade;
mod palette;

pub use effects::{backdrop_color, darken_color, text_input_style};
pub use fade::faded_theme;
pub use palette::hydebar_theme;
