//! The two read-only checks: what is out of date, and how far the
//! `HyDE` clone fell behind.

use std::process::{ExitStatus, Stdio};

use tokio::process;

use super::{
    super::state::Update,
    error::{CheckFailure, CommandError}
};

/// Exit status a shell reports when the command it was asked to run is
/// missing.
const COMMAND_NOT_FOUND: i32 = 127;

/// Asks the package manager what is out of date.
///
/// The command is spawned into a process group of its own, so a check still
/// talking to a mirror when the schedule is torn down goes with it instead
/// of outliving the task that started it.
///
/// A failing exit status alone does not make the answer worthless: the
/// usual check pipelines end in a query that reports "nothing to do" by
/// failing, and treating that as an error left the bar showing a list
/// it had already been told was empty. Output that parses is therefore
/// believed whatever the status, and a status without output is only a
/// failure when the command also complained about something.
pub async fn check_for_updates(command: &str) -> Result<Vec<Update>, CheckFailure> {
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

    classify(output.status, &updates, &output.stderr)?;

    Ok(updates)
}

/// Decides whether a finished check is worth believing.
///
/// Split out of the spawn so the policy can be exercised without a shell.
fn classify(status: ExitStatus, updates: &[Update], stderr: &[u8]) -> Result<(), CheckFailure> {
    if status.success() || !updates.is_empty() {
        return Ok(());
    }

    if status.code() == Some(COMMAND_NOT_FOUND) {
        return Err(CheckFailure::Unavailable(CommandError::Status(status)));
    }

    if stderr.iter().any(|byte| !byte.is_ascii_whitespace()) {
        return Err(CheckFailure::Transient(CommandError::Status(status)));
    }

    Ok(())
}

/// Asks the `HyDE` clone how far behind upstream it is.
///
/// Only remote-tracking refs are touched: the fetch never rewrites the
/// working tree, so a clone holding local work is read, not disturbed.
pub async fn check_hyde(clone: &str, branch: &str) -> Result<(String, Vec<String>), CommandError> {
    git(clone, &["fetch", "--quiet", "origin", branch]).await?;

    let version = git(clone, &["describe", "--tags", "--always"]).await?;
    let range = format!("HEAD..origin/{branch}");
    let log = git(clone, &["log", "--pretty=format:%s", &range]).await?;

    Ok((version.trim().to_owned(), parse_hyde_commits(&log)))
}

/// Runs one git command inside `clone` and returns its stdout.
async fn git(clone: &str, args: &[&str]) -> Result<String, CommandError> {
    let mut spawner = process::Command::new("git");
    spawner
        .arg("-C")
        .arg(clone)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = crate::utils::process_group::guarded_output(&mut spawner).await?;

    if !output.status.success() {
        return Err(CommandError::Status(output.status));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_hyde_commits(log: &str) -> Vec<String> {
    log.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
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
#[cfg_attr(coverage_nightly, coverage(off))]
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
        assert!(classify(status(0), &[], b"").is_ok());
    }

    /// `checkupdates; paru -Qua` ends in a query that fails when the AUR
    /// has nothing, and silence from both is the shape of
    /// "everything is current".
    #[test]
    fn a_silent_failure_means_nothing_is_out_of_date() {
        assert!(classify(status(1), &[], b"\n").is_ok());
    }

    #[test]
    fn output_that_parses_outweighs_a_failing_status() {
        assert!(classify(status(1), &one_update(), b"mirror is slow").is_ok());
    }

    #[test]
    fn a_missing_command_makes_the_check_unavailable() {
        assert!(matches!(
            classify(
                status(COMMAND_NOT_FOUND),
                &[],
                b"bash: checkupdates: not found"
            ),
            Err(CheckFailure::Unavailable(_))
        ));
    }

    #[test]
    fn a_complaint_without_output_is_a_passing_failure() {
        assert!(matches!(
            classify(status(1), &[], b"could not reach the mirror"),
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

    #[test]
    fn hyde_commits_skip_blank_lines() {
        let commits = parse_hyde_commits("fix: one\n\n  feat: two  \n");

        assert_eq!(commits, vec!["fix: one".to_owned(), "feat: two".to_owned()]);
    }
}
