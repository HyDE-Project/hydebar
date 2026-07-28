//! Failure modes of a custom module listener process.

use std::sync::Arc;

use crate::modules::ModuleError;

/// Something that went wrong while running or reading a listener process.
#[derive(Debug, Clone)]
pub enum CustomCommandError {
    Spawn(Arc<std::io::Error>),
    MissingStdout,
    Read(Arc<std::io::Error>),
    Parse(String, Arc<serde_json::Error>),
    Wait(Arc<std::io::Error>),
    NonZeroExit { status: Option<i32> },
    Signal(u8, Arc<std::io::Error>),
    UnsupportedSignal(u8),
    ChannelClosed
}

impl std::fmt::Display for CustomCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(err) => {
                write!(f, "failed to spawn custom module listener process: {}", err)
            }
            Self::MissingStdout => write!(f, "custom module listener did not expose stdout"),
            Self::Read(err) => {
                write!(f, "failed to read line from custom module output: {}", err)
            }
            Self::Parse(snippet, err) => {
                write!(
                    f,
                    "failed to parse custom module output: {} ({})",
                    snippet, err
                )
            }
            Self::Wait(err) => write!(f, "failed to wait for custom module process: {}", err),
            Self::NonZeroExit {
                status
            } => write!(
                f,
                "custom module process exited unsuccessfully ({:?})",
                status
            ),
            Self::Signal(offset, err) => write!(
                f,
                "failed to listen for the custom module refresh signal SIGRTMIN+{}: {}",
                offset, err
            ),
            Self::UnsupportedSignal(offset) => write!(
                f,
                "custom module refresh signal SIGRTMIN+{} is outside the real time range",
                offset
            ),
            Self::ChannelClosed => write!(f, "custom module updates channel closed")
        }
    }
}

impl std::error::Error for CustomCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(err) => Some(err.as_ref()),
            Self::Read(err) => Some(err.as_ref()),
            Self::Parse(_, err) => Some(err.as_ref()),
            Self::Wait(err) => Some(err.as_ref()),
            Self::Signal(_, err) => Some(err.as_ref()),
            _ => None
        }
    }
}

impl CustomCommandError {
    /// Short message rendered in place of the module content.
    pub(super) fn to_display_message(&self) -> String {
        match self {
            Self::Parse(snippet, ..) => format!("Invalid output: {snippet}"),
            Self::NonZeroExit {
                status
            } => match status {
                Some(code) => format!("Listener exited with status {code}"),
                None => String::from("Listener exited due to signal")
            },
            Self::Signal(offset, _) => format!("Cannot watch SIGRTMIN+{offset}"),
            Self::UnsupportedSignal(offset) => format!("Signal SIGRTMIN+{offset} out of range"),
            Self::ChannelClosed => String::from("Listener updates queue closed"),
            Self::MissingStdout => String::from("Listener stdout unavailable"),
            Self::Spawn(_) | Self::Read(_) | Self::Wait(_) => String::from("Listener IO failure")
        }
    }
}

/// Trims a listener output line so error messages stay readable.
pub(super) fn truncate_snippet(line: &str) -> String {
    const MAX_LEN: usize = 120;

    if line.len() <= MAX_LEN {
        return line.to_owned();
    }

    let mut truncated = String::with_capacity(MAX_LEN + 1);
    for (idx, ch) in line.char_indices() {
        if idx >= MAX_LEN {
            truncated.push('…');
            break;
        }
        truncated.push(ch);
    }
    truncated
}

/// Error raised by the listener task itself.
#[derive(Debug, Clone)]
pub(super) enum CustomListenerError {
    Command(CustomCommandError),
    Module(ModuleError)
}

impl std::fmt::Display for CustomListenerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(err) => write!(f, "{}", err),
            Self::Module(err) => write!(f, "{}", err)
        }
    }
}

impl std::error::Error for CustomListenerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Command(err) => Some(err),
            Self::Module(err) => Some(err)
        }
    }
}
