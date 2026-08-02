//! Entry point of the bar binary.
//!
//! The pieces of startup each live in a module of their own: the async
//! runtime is sized and built in [`runtime`], the rotating logger and the
//! panic hook in [`logging`], the sweeping of strays and arming of the
//! guards in [`housekeeping`], and the iced application is assembled and run
//! in [`application`]. This file only strings them together and turns the
//! outcome into an exit code.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![allow(mismatched_lifetime_syntaxes)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::double_ended_iterator_last)]

mod application;
mod error;
mod housekeeping;
mod instance;
mod logging;
mod runtime;
mod startup_scale;

use std::process::ExitCode;

use log::error;

use crate::error::MainError;

/// Starts the async runtime and hands its handle to the bar.
///
/// The event loop must not run inside the runtime: the graphics layer blocks
/// the calling thread while creating the compositor, and blocking a thread that
/// is already driving tasks aborts the process.
fn main() -> ExitCode {
    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err}");
            eprintln!("hydebar: {err}");

            ExitCode::FAILURE
        }
    }
}

/// Builds the runtime and runs the bar on it.
fn start() -> Result<(), MainError> {
    let runtime = runtime::build()?;

    application::run(runtime.handle().clone())
}
