//! Failure modes of a custom module listener process.

use std::sync::Arc;

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
                write!(f, "failed to spawn custom module listener process: {err}")
            }
            Self::MissingStdout => write!(f, "custom module listener did not expose stdout"),
            Self::Read(err) => {
                write!(f, "failed to read line from custom module output: {err}")
            }
            Self::Parse(snippet, err) => {
                write!(f, "failed to parse custom module output: {snippet} ({err})")
            }
            Self::Wait(err) => write!(f, "failed to wait for custom module process: {err}"),
            Self::NonZeroExit {
                status
            } => write!(
                f,
                "custom module process exited unsuccessfully ({status:?})"
            ),
            Self::Signal(offset, err) => write!(
                f,
                "failed to listen for the custom module refresh signal SIGRTMIN+{offset}: {err}"
            ),
            Self::UnsupportedSignal(offset) => write!(
                f,
                "custom module refresh signal SIGRTMIN+{offset} is outside the real time range"
            ),
            Self::ChannelClosed => write!(f, "custom module updates channel closed")
        }
    }
}

impl std::error::Error for CustomCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(err) | Self::Read(err) | Self::Wait(err) | Self::Signal(_, err) => {
                Some(err.as_ref())
            }
            Self::Parse(_, err) => Some(err.as_ref()),
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
            } => status.map_or_else(
                || String::from("Listener exited due to signal"),
                |code| format!("Listener exited with status {code}")
            ),
            Self::Signal(offset, _) => format!("Cannot watch SIGRTMIN+{offset}"),
            Self::UnsupportedSignal(offset) => {
                format!("Signal SIGRTMIN+{offset} out of range")
            }
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
    Command(CustomCommandError)
}

impl std::fmt::Display for CustomListenerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(err) => write!(f, "{err}")
        }
    }
}

impl std::error::Error for CustomListenerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Command(err) => Some(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, sync::Arc};

    use super::*;

    fn io_error() -> Arc<std::io::Error> {
        Arc::new(std::io::Error::other("boom"))
    }

    fn json_error() -> Arc<serde_json::Error> {
        Arc::new(
            serde_json::from_str::<serde_json::Value>("not json").expect_err("json must not parse")
        )
    }

    #[test]
    fn a_parse_failure_shows_the_offending_line() {
        let error = CustomCommandError::Parse("not json".to_owned(), json_error());

        assert_eq!(error.to_display_message(), "Invalid output: not json");
    }

    #[test]
    fn an_exit_status_is_named_and_a_signal_death_is_called_one() {
        let with_status = CustomCommandError::NonZeroExit {
            status: Some(3)
        };
        let by_signal = CustomCommandError::NonZeroExit {
            status: None
        };

        assert_eq!(
            with_status.to_display_message(),
            "Listener exited with status 3"
        );
        assert_eq!(
            by_signal.to_display_message(),
            "Listener exited due to signal"
        );
    }

    #[test]
    fn every_io_failure_shares_one_terse_message() {
        for error in [
            CustomCommandError::Spawn(io_error()),
            CustomCommandError::Read(io_error()),
            CustomCommandError::Wait(io_error())
        ] {
            assert_eq!(error.to_display_message(), "Listener IO failure");
        }
    }

    #[test]
    fn signal_troubles_name_their_offset() {
        let unwatchable = CustomCommandError::Signal(20, io_error());
        let out_of_range = CustomCommandError::UnsupportedSignal(64);

        assert_eq!(unwatchable.to_display_message(), "Cannot watch SIGRTMIN+20");
        assert_eq!(
            out_of_range.to_display_message(),
            "Signal SIGRTMIN+64 out of range"
        );
    }

    #[test]
    fn the_long_display_names_the_failing_stage() {
        let spawn = CustomCommandError::Spawn(io_error());
        let parse = CustomCommandError::Parse("not json".to_owned(), json_error());

        assert_eq!(
            spawn.to_string(),
            "failed to spawn custom module listener process: boom"
        );
        assert!(
            parse
                .to_string()
                .starts_with("failed to parse custom module output: not json (")
        );
    }

    #[test]
    fn io_and_parse_errors_expose_their_source() {
        for error in [
            CustomCommandError::Spawn(io_error()),
            CustomCommandError::Read(io_error()),
            CustomCommandError::Wait(io_error()),
            CustomCommandError::Signal(20, io_error()),
            CustomCommandError::Parse("not json".to_owned(), json_error())
        ] {
            assert!(error.source().is_some());
        }
    }

    #[test]
    fn errors_without_a_cause_report_none() {
        for error in [
            CustomCommandError::MissingStdout,
            CustomCommandError::NonZeroExit {
                status: Some(1)
            },
            CustomCommandError::UnsupportedSignal(64),
            CustomCommandError::ChannelClosed
        ] {
            assert!(error.source().is_none());
        }
    }

    #[test]
    fn a_short_line_survives_truncation_untouched() {
        let exactly_at_the_limit = "a".repeat(120);

        assert_eq!(truncate_snippet("ok"), "ok");
        assert_eq!(truncate_snippet(&exactly_at_the_limit), exactly_at_the_limit);
    }

    #[test]
    fn a_long_line_is_cut_and_marked_with_an_ellipsis() {
        let truncated = truncate_snippet(&"a".repeat(200));

        assert_eq!(truncated.chars().count(), 121);
        assert!(truncated.ends_with('…'));
        assert!(truncated.starts_with(&"a".repeat(120)));
    }

    #[test]
    fn truncation_never_splits_a_multibyte_character() {
        let truncated = truncate_snippet(&"é".repeat(100));

        assert!(truncated.ends_with('…'));
        assert!(truncated.chars().rev().skip(1).all(|ch| ch == 'é'));
    }

    #[test]
    fn a_character_straddling_the_limit_is_kept_whole() {
        let line = format!("{}€", "a".repeat(119));

        assert_eq!(truncate_snippet(&line), line);
    }

    #[test]
    fn a_listener_error_wears_its_command_error() {
        let listener = CustomListenerError::Command(CustomCommandError::ChannelClosed);

        assert_eq!(
            listener.to_string(),
            CustomCommandError::ChannelClosed.to_string()
        );
        assert!(listener.source().is_some());
    }
}
