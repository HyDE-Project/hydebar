//! Reader for the wallbash palette HyDE generates from the wallpaper.
//!
//! HyDE extracts four dominant colours from the wallpaper, derives a text
//! colour and a nine-step accent ramp for each, and writes the lot to
//! `~/.cache/hyde/dcols/<hash>.dcol`, with `~/.cache/hyde/wall.dcol` pointing
//! at the one in force. Every consumer on the desktop — the notification
//! daemon, the terminal, the launcher — is coloured from that file through a
//! template rendered for it.
//!
//! The bar reads the palette directly instead of waiting for a template to be
//! rendered on its behalf. That is what lets it follow a theme switch on its
//! own, with no other process involved.
//!
//! The grammar and the palette live in [`palette`], the rule deciding which
//! palette applies in [`selection`], the bar's share of the colours in
//! [`roles`] and the disk access in [`reader`].

mod palette;
mod reader;
mod roles;
mod selection;

#[cfg(test)]
pub(super) mod fixtures;

pub use palette::{DcolMode, DcolPalette};
pub(super) use reader::read;
