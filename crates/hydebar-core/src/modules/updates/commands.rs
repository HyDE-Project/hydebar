//! Shell work behind the updates module: the checks, the updates and
//! the plumbing they narrate through.

mod check;
mod elevate;
mod error;
mod run;
mod stream;

pub(super) use check::{check_for_updates, check_hyde};
pub(super) use error::CheckFailure;
pub(super) use run::{apply_updates, update_hyde};
