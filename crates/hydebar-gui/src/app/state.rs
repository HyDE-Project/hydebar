//! Application state of the GUI layer.

mod app;
mod appearance;
mod init;
mod message;
#[cfg(test)]
mod test_support;

pub use app::App;
pub(super) use app::GREETING_LIFETIME;
pub use message::Message;
