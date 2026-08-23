//! The bar's schema: what a configuration may say, what the desktop's theme
//! offers, and the ports through which the compositor is asked.
//!
//! Nothing here draws, and nothing here depends on the toolkit that does. The
//! crate is the vocabulary every other crate reads a configuration in, so a
//! type that lands here is one the bar agrees on rather than one a renderer
//! happens to hold.

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![warn(missing_docs)]
pub mod bar_layout;
pub mod compositor_ipc;
pub mod compositor_look;
pub mod config;
pub mod hyde_dirs;
mod hyde_files;
pub mod hyde_state;
pub(crate) mod hypr_vars;
pub mod ports;
pub mod shell_vars;
pub mod theme_source;
