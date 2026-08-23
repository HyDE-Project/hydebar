//! The bar's own client for the compositor it runs under.
//!
//! Everything here speaks the socket the compositor already listens on, the
//! one [`hydebar_proto::compositor_ipc`] opens: questions in JSON, commands as
//! text, answers read as only the fields the bar draws. Owning the client is
//! what lets a record carry a field the bar needs rather than the fields a
//! general purpose crate chose to model, and it keeps the bar honest about how
//! little of the compositor it actually asks for.

pub mod command;
pub mod query;
pub mod records;
