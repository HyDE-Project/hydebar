use std::{
    process::{ExitStatus, Output, Stdio},
    sync::Arc
};

use log::error;
use tokio::process::Command;

use crate::utils::process_group;

/// Error type emitted when launching shell commands fails.
///
/// The error keeps a shared reference to the original command string so callers
/// can differentiate failures per command without cloning large buffers.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
///
/// use hydebar_core::utils::launcher::{LauncherError, run_shell_command_with_output};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let runtime = tokio::runtime::Runtime::new()?;
/// let command = Arc::from("true");
/// runtime.block_on(async move {
///     let output = run_shell_command_with_output(&command).await?;
///     assert!(output.status.success());
///     Ok::<(), LauncherError>(())
/// })?;
/// Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherError {
    /// The command could not be spawned by the operating system.
    Spawn {
        /// The attempted command string.
        command: Arc<str>,
        /// Additional context provided by the OS error.
        context: Arc<str>
    },
    /// The command executed but returned a non-zero exit status.
    NonZeroExit {
        /// The attempted command string.
        command: Arc<str>,
        /// The exit status returned by the process.
        status:  ExitStatus
    }
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn {
                command,
                context
            } => {
                write!(f, "failed to spawn `{command}`: {context}")
            }
            Self::NonZeroExit {
                command,
                status
            } => {
                write!(f, "command `{command}` exited with status {status}")
            }
        }
    }
}

impl std::error::Error for LauncherError {}

impl LauncherError {
    fn spawn_error(command: Arc<str>, error: &std::io::Error) -> Self {
        Self::Spawn {
            command,
            context: Arc::from(error.to_string())
        }
    }

    const fn exit_error(command: Arc<str>, status: ExitStatus) -> Self {
        Self::NonZeroExit {
            command,
            status
        }
    }
}

/// Execute the given command and return its stdout/stderr output.
///
/// # Errors
///
/// Returns [`LauncherError::Spawn`] if the process cannot be created or
/// [`LauncherError::NonZeroExit`] when the command finishes unsuccessfully.
pub async fn run_shell_command_with_output(command: &Arc<str>) -> Result<Output, LauncherError> {
    let mut process = Command::new("bash");
    process.arg("-c").arg(command.as_ref());

    let output = process
        .output()
        .await
        .map_err(|error| LauncherError::spawn_error(command.clone(), &error))?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(LauncherError::exit_error(command.clone(), output.status))
    }
}

/// Run `command` to completion without collecting what it printed.
///
/// This is what every fire-and-forget action of the bar goes through, and it
/// differs from [`run_shell_command_with_output`] in the two ways that matter
/// for a command that starts a whole desktop action:
///
/// * the standard streams are inherited rather than piped. A piped stream is
///   only at end of file once every descendant holding it has exited, and a
///   desktop script routinely leaves background children behind — waiting for
///   them would keep this future, and the report of what the command did,
///   pending for as long as the longest of them lives. Inheriting also puts the
///   command's own diagnostics straight into the bar's log, where a user
///   looking for the reason a desktop action failed will find them.
/// * the command leads a process group of its own, so a signal aimed at the bar
///   does not tear a half-performed desktop change apart.
///
/// # Errors
///
/// Returns [`LauncherError::Spawn`] if the process cannot be created or
/// [`LauncherError::NonZeroExit`] when it finishes unsuccessfully.
pub async fn run_detached(command: &Arc<str>) -> Result<(), LauncherError> {
    let mut process = Command::new("bash");
    process.arg("-c").arg(command.as_ref()).stdin(Stdio::null());
    process_group::in_own_group(&mut process);

    let mut child = process
        .spawn()
        .map_err(|error| LauncherError::spawn_error(command.clone(), &error))?;

    let status = child
        .wait()
        .await
        .map_err(|error| LauncherError::spawn_error(command.clone(), &error))?;

    if status.success() {
        Ok(())
    } else {
        Err(LauncherError::exit_error(command.clone(), status))
    }
}

fn spawn_and_log(command: String, context: &'static str) {
    tokio::spawn(async move {
        let command_arc: Arc<str> = Arc::from(command);

        log::info!("{context} runs: {command_arc}");

        if let Err(error) = run_detached(&command_arc).await {
            error!("{context} command failed: {error}");
            crate::services::hyprland_notify::post_to_bus(&format!(
                "the {context} command failed: {error}"
            ));
        }
    });
}

/// Execute an arbitrary shell command without awaiting its completion.
///
/// The command is executed in a background Tokio task to preserve the
/// fire-and-forget semantics used throughout the UI.
///
/// # Examples
///
/// ```no_run
/// use hydebar_core::utils::launcher;
///
/// launcher::execute_command("notify-send hydebar 'Hello'".to_owned());
/// ```
pub fn execute_command(command: String) {
    spawn_and_log(command, "launcher");
}

/// Execute the configured suspend command in the background.
pub fn suspend(command: String) {
    spawn_and_log(command, "suspend");
}

/// Execute the configured shutdown command in the background.
pub fn shutdown(command: String) {
    spawn_and_log(command, "shutdown");
}

/// Execute the configured reboot command in the background.
pub fn reboot(command: String) {
    spawn_and_log(command, "reboot");
}

/// Execute the configured logout command in the background.
pub fn logout(command: String) {
    spawn_and_log(command, "logout");
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{LauncherError, run_detached, run_shell_command_with_output};

    #[tokio::test]
    async fn reports_successful_status() -> Result<(), Box<dyn std::error::Error>> {
        let command = Arc::from("true");

        let output = tokio::time::timeout(
            Duration::from_secs(5),
            run_shell_command_with_output(&command)
        )
        .await??;

        assert!(output.status.success());

        Ok(())
    }

    #[tokio::test]
    async fn reports_non_zero_exit_as_error() -> Result<(), Box<dyn std::error::Error>> {
        let command = Arc::from("exit 42");

        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            run_shell_command_with_output(&command)
        )
        .await?;

        match outcome {
            Err(LauncherError::NonZeroExit {
                status, ..
            }) => {
                assert_eq!(status.code(), Some(42));
            }
            other => {
                return Err(format!("unexpected outcome: {other:?}").into());
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn captures_command_output() -> Result<(), Box<dyn std::error::Error>> {
        let command = Arc::from("printf foo");

        let output = tokio::time::timeout(
            Duration::from_secs(5),
            run_shell_command_with_output(&command)
        )
        .await??;

        assert_eq!(output.stdout, b"foo");
        assert!(output.stderr.is_empty());
        assert!(output.status.success());

        Ok(())
    }

    #[tokio::test]
    async fn a_detached_run_reports_a_successful_command() -> Result<(), Box<dyn std::error::Error>>
    {
        let command = Arc::from("true");

        tokio::time::timeout(Duration::from_secs(5), run_detached(&command)).await??;

        Ok(())
    }

    #[tokio::test]
    async fn a_detached_run_reports_a_failing_command() -> Result<(), Box<dyn std::error::Error>> {
        let command = Arc::from("exit 42");

        let outcome = tokio::time::timeout(Duration::from_secs(5), run_detached(&command)).await?;

        match outcome {
            Err(LauncherError::NonZeroExit {
                status, ..
            }) => assert_eq!(status.code(), Some(42)),
            other => return Err(format!("unexpected outcome: {other:?}").into())
        }

        Ok(())
    }

    #[tokio::test]
    async fn a_detached_run_reports_a_command_that_cannot_start()
    -> Result<(), Box<dyn std::error::Error>> {
        let command = Arc::from("exec /nonexistent/hydebar/command");

        let outcome = tokio::time::timeout(Duration::from_secs(5), run_detached(&command)).await?;

        assert!(matches!(outcome, Err(LauncherError::NonZeroExit { .. })));

        Ok(())
    }

    /// A desktop script hands work to background children and returns; the bar
    /// has to hear about the outcome then, not once the last of them is gone.
    #[tokio::test]
    async fn a_detached_run_does_not_wait_for_the_children_the_command_left_behind()
    -> Result<(), Box<dyn std::error::Error>> {
        let command = Arc::from("sleep 5 & disown; exit 0");

        tokio::time::timeout(Duration::from_secs(2), run_detached(&command)).await??;

        Ok(())
    }

    /// The same command through the output-collecting runner keeps waiting,
    /// which is exactly why the detached one exists.
    #[tokio::test]
    async fn collecting_the_output_waits_for_those_children() {
        let command = Arc::from("sleep 5 & disown; exit 0");

        let outcome = tokio::time::timeout(
            Duration::from_millis(500),
            run_shell_command_with_output(&command)
        )
        .await;

        assert!(
            outcome.is_err(),
            "the output runner returned before the child was gone"
        );
    }
}
