use std::process::{ExitStatus, Stdio};

use tokio::process;

use super::state::Update;

/// Errors that can occur while executing an update-related shell command.
#[derive(Debug)]
pub(super) enum CommandError {
    /// Failed to spawn the command.
    Io(std::io::Error),
    /// The command exited with a non-zero status.
    Status(ExitStatus)
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(_) => write!(f, "failed to execute command"),
            Self::Status(status) => write!(f, "command exited with failure status: {status}")
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Status(_) => None
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl CommandError {
    pub(super) fn or_log(self, context: &str) {
        log::warn!("{context}: {self}");
    }
}

/// Why a check could not be trusted.
#[derive(Debug)]
pub(super) enum CheckFailure {
    /// The check cannot be run on this machine at all.
    ///
    /// A configuration naming a package manager the machine does not have is
    /// not a fault to report every hour; it means the bar has nothing to show.
    Unavailable(CommandError),
    /// The check ran but this particular run said nothing usable.
    Transient(CommandError)
}

impl std::fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(err) | Self::Transient(err) => write!(f, "{err}")
        }
    }
}

/// Exit status a shell reports when the command it was asked to run is missing.
const COMMAND_NOT_FOUND: i32 = 127;

/// Asks the package manager what is out of date.
///
/// The command is spawned into a process group of its own, so a check still
/// talking to a mirror when the schedule is torn down goes with it instead of
/// outliving the task that started it.
///
/// A failing exit status alone does not make the answer worthless: the usual
/// check pipelines end in a query that reports "nothing to do" by failing, and
/// treating that as an error left the bar showing a list it had already been
/// told was empty. Output that parses is therefore believed whatever the
/// status, and a status without output is only a failure when the command also
/// complained about something.
pub(super) async fn check_for_updates(command: &str) -> Result<Vec<Update>, CheckFailure> {
    let mut spawner = process::Command::new("bash");
    spawner
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = crate::utils::process_group::guarded_output(&mut spawner)
        .await
        .map_err(|err| CheckFailure::Unavailable(CommandError::Io(err)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let updates = parse_updates(stdout.trim_end_matches('\n'));

    classify(&output.status, &updates, &output.stderr)?;

    Ok(updates)
}

/// Decides whether a finished check is worth believing.
///
/// Split out of the spawn so the policy can be exercised without a shell.
fn classify(status: &ExitStatus, updates: &[Update], stderr: &[u8]) -> Result<(), CheckFailure> {
    if status.success() || !updates.is_empty() {
        return Ok(());
    }

    if status.code() == Some(COMMAND_NOT_FOUND) {
        return Err(CheckFailure::Unavailable(CommandError::Status(*status)));
    }

    if stderr.iter().any(|byte| !byte.is_ascii_whitespace()) {
        return Err(CheckFailure::Transient(CommandError::Status(*status)));
    }

    Ok(())
}

pub(super) async fn apply_updates(command: &str) -> Result<(), CommandError> {
    let output = process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !output.success() {
        return Err(CommandError::Status(output));
    }

    Ok(())
}

fn parse_updates(output: &str) -> Vec<Update> {
    output.lines().filter_map(parse_update_line).collect()
}

fn parse_update_line(line: &str) -> Option<Update> {
    let mut tokens = line.split_whitespace();
    let package = tokens.next()?;
    let from = tokens.next()?;
    let separator = tokens.next()?;
    let to = tokens.next()?;

    if separator != "->" {
        return None;
    }

    Some(Update {
        package: package.to_owned(),
        from:    from.to_owned(),
        to:      to.to_owned()
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::ExitStatusExt;

    use super::*;

    fn status(code: i32) -> ExitStatus {
        ExitStatus::from_raw(code << 8)
    }

    fn one_update() -> Vec<Update> {
        vec![Update {
            package: "pkg".to_owned(),
            from:    "1".to_owned(),
            to:      "2".to_owned()
        }]
    }

    #[test]
    fn a_successful_check_is_believed() {
        assert!(classify(&status(0), &[], b"").is_ok());
    }

    /// `checkupdates; paru -Qua` ends in a query that fails when the AUR has
    /// nothing, and silence from both is the shape of "everything is current".
    #[test]
    fn a_silent_failure_means_nothing_is_out_of_date() {
        assert!(classify(&status(1), &[], b"\n").is_ok());
    }

    #[test]
    fn output_that_parses_outweighs_a_failing_status() {
        assert!(classify(&status(1), &one_update(), b"mirror is slow").is_ok());
    }

    #[test]
    fn a_missing_command_makes_the_check_unavailable() {
        assert!(matches!(
            classify(
                &status(COMMAND_NOT_FOUND),
                &[],
                b"bash: checkupdates: not found"
            ),
            Err(CheckFailure::Unavailable(_))
        ));
    }

    #[test]
    fn a_complaint_without_output_is_a_passing_failure() {
        assert!(matches!(
            classify(&status(1), &[], b"could not reach the mirror"),
            Err(CheckFailure::Transient(_))
        ));
    }

    #[test]
    fn parse_updates_skips_malformed_lines() {
        let output = "pkg1 1 -> 2\ninvalid line\npkg2 3 -> 4";

        let updates = parse_updates(output);

        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].package, "pkg1");
        assert_eq!(updates[1].package, "pkg2");
    }

    #[test]
    fn parse_updates_handles_empty_input() {
        let updates = parse_updates("");

        assert!(updates.is_empty());
    }
}
