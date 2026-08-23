//! The media player: the leading MPRIS player, on the bar and in a menu.
//!
//! One folder, five rooms: [`messages`] names what the module accepts,
//! [`commands`] dispatches player commands to the service, [`state`] folds
//! messages in, [`module`] wires the module to the bar and [`view`] draws
//! the menu. The root holds the state the rooms share, and the bridge that
//! carries listener events onto the module event bus.

use tokio::{runtime::Handle, task::JoinHandle};

use crate::{ModuleEventSender, services::mpris::MprisPlayerService};

mod commands;
mod messages;
mod module;
mod publisher;
mod state;
mod view;

pub use messages::Message;
use publisher::MediaPlayerPublisher;

/// Bar entry naming what is playing, and driving it.
#[derive(Debug, Default)]
pub struct MediaPlayer {
    service:   Option<MprisPlayerService>,
    sender:    Option<ModuleEventSender<Message>>,
    runtime:   Option<Handle>,
    tasks:     Vec<JoinHandle<()>>,
    /// The bar line for the leading player, rendered when the state moves.
    ///
    /// Composed once per service event instead of per frame: the join,
    /// the format and the truncation ran on every repaint for a value
    /// that only changes on a track or player change.
    bar_title: Option<String>
}
