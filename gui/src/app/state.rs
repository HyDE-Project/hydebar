//! Application state of the GUI layer.

mod app;
mod appearance;
pub mod history;
mod init;
mod message;
mod runtime;
mod screen;
#[cfg(test)]
pub(in crate::app) mod test_support;

pub use app::App;
pub(super) use app::GREETING_LIFETIME;
pub use message::Message;
