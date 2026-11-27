//! Runtime state and listener wiring for a custom module.
//!
//! Three rooms. [`custom`] is what the module knows and what it was told
//! last, [`message`] is what it answers to, and [`registration`] is the
//! starting and stopping of the shell behind it.

mod custom;
mod message;
mod registration;

pub use self::{
    custom::Custom,
    message::{CustomCommandService, Message}
};
