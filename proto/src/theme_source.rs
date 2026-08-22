//! Reader for the desktop theme published by the [HyDE Project](https://github.com/HyDE-Project).
//!
//! The bar follows the desktop instead of restating it: the colours, the font
//! and the corner radius are read from what `HyDE` and the compositor already
//! know about themselves.
//!
//! **Where the values come from.** Only the `HyDE` directories are a source:
//! `~/.cache/hyde/wall.dcol` for the palette, `~/.config/hyde/config.toml`,
//! `~/.config/hyde/themes/` and `~/.local/state/hyde/staterc` for the theme in
//! force and the font, and the compositor itself for the corner radius. The
//! stylesheets under `~/.config/waybar` are read only when a `HyDE` install
//! answers for nothing, because they are generated files that stop being
//! updated the moment the generator is not running — see [`waybar`].
//!
//! Nothing read here outranks the bar's own configuration file. The snapshot is
//! overlaid onto the configuration and only fills the fields the user left
//! unset, so a `HyDE` theme switch can never undo something written in
//! `~/.config/hydebar/config.toml`.
//!
//! The reader is forgiving throughout: every file is optional, unknown
//! declarations are ignored and a value that cannot be understood stays
//! [`None`] so the next source down can answer instead.
//!
//! The colour type lives in [`color`], the wallbash palette in [`dcol`], the
//! font chain in [`font`], the fallback stylesheet reader in [`waybar`] and the
//! snapshot with its entry points in [`theme`].

mod color;
mod css;
pub mod dcol;
mod extract;
mod font;
mod length;
mod preview;
mod swatch;
mod theme;
mod waybar;

#[cfg(test)]
mod fixtures;

pub use color::Rgba;
pub use dcol::{DcolMode, DcolPalette};
pub use preview::{picture_preview, theme_preview};
pub use swatch::{ThemeSwatch, theme_swatch};
pub use theme::{HydeTheme, load, load_from};
