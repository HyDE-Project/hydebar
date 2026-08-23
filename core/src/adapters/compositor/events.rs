//! What the compositor announces, read off the socket it announces on.
//!
//! The compositor writes one line per thing that happens to a second socket,
//! `NAME>>DATA`, and keeps writing for as long as the connection is held. The
//! bar reads that socket itself: connecting costs one open, and the events the
//! bar draws nothing for are dropped where they are read rather than being
//! turned into a callback that does nothing.

use hydebar_proto::{compositor_ipc, ports::hyprland::HyprlandError};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    net::UnixStream
};

mod parse;

/// The name this connection is reported under when it cannot be made.
const CONNECT_OP: &str = "compositor_events";

/// One thing the compositor announced that the bar draws something for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositorEvent {
    /// The session moved to another workspace.
    WorkspaceChanged,
    /// A workspace came into being.
    WorkspaceAdded,
    /// A workspace ceased to exist.
    WorkspaceRemoved,
    /// A workspace moved to another screen.
    WorkspaceMoved,
    /// The session moved to another screen.
    ActiveMonitorChanged,
    /// A screen went away.
    MonitorRemoved,
    /// A window was mapped.
    WindowOpened,
    /// A window was unmapped.
    WindowClosed,
    /// A window moved to another workspace.
    WindowMoved,
    /// Focus moved to another window.
    ActiveWindowChanged,
    /// A window renamed itself.
    WindowTitleChanged,
    /// A special workspace was opened over a screen.
    SpecialChanged,
    /// The special workspace over a screen was closed.
    SpecialRemoved,
    /// The keyboard moved to another layout.
    LayoutChanged,
    /// The keyboard entered a submap, or left the one it was in.
    SubmapChanged(Option<String>),
    /// A window asked for attention.
    Urgent {
        /// Address of the window that asked.
        address: String
    },
    /// The compositor read its configuration again.
    ConfigReloaded
}

/// A connection to the compositor's announcements.
#[derive(Debug)]
pub struct EventStream {
    lines: Lines<BufReader<UnixStream>>
}

impl EventStream {
    /// Opens a connection to the socket the compositor announces on.
    ///
    /// # Errors
    ///
    /// Returns [`HyprlandError::Message`] when there is no session to listen
    /// to, or when the socket refuses the connection.
    pub async fn connect() -> Result<Self, HyprlandError> {
        let path = compositor_ipc::event_socket_path().ok_or(HyprlandError::Message {
            operation: CONNECT_OP,
            message:   String::from("no compositor session to listen to")
        })?;

        let stream = UnixStream::connect(&path)
            .await
            .map_err(|err| HyprlandError::Message {
                operation: CONNECT_OP,
                message:   format!("{} could not be opened: {err}", path.display())
            })?;

        Ok(Self {
            lines: BufReader::new(stream).lines()
        })
    }

    /// The next event the bar draws something for.
    ///
    /// Lines the bar has no readout for are skipped here rather than handed
    /// on. Returns [`None`] when the compositor closed the connection, which
    /// is the signal to reconnect.
    pub async fn next(&mut self) -> Option<CompositorEvent> {
        loop {
            let line = self.lines.next_line().await.ok()??;

            if let Some(event) = parse::line(&line) {
                return Some(event);
            }
        }
    }
}
