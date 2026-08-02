//! Watcher for the desktop theme published by the `HyDE` Project.
//!
//! The bar reads its colours and font from the `HyDE` directories, which live
//! outside the bar configuration; the configuration watcher never sees them, so
//! without this the bar kept its old palette until it was restarted.
//!
//! This module closes that gap: it follows every file the theme is read from
//! and, on any change, re-runs the very same load the configuration watcher
//! runs. The reload emits a [`ConfigEvent::Applied`], so the palette
//! cross-fades through the path a hot reload already takes instead of through a
//! second, parallel one. Because the reload goes through the configuration file
//! first, anything the user wrote there still wins over the theme.
//!
//! The files to follow live in [`sources`], the event rules in [`interpret`]
//! and the subscription in [`recipe`].

mod interpret;
mod recipe;
mod sources;

#[cfg(test)]
mod tests;

pub use recipe::theme_subscription;
pub use sources::{ThemeRoots, ThemeWatchTarget, watch_targets};
