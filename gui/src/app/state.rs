//! Application state of the GUI layer.

mod app;
mod appearance;
pub mod history;
mod init;
mod message;
#[cfg(test)]
pub(in crate::app) mod test_support;

pub use app::App;
pub(super) use app::GREETING_LIFETIME;
pub use message::Message;
