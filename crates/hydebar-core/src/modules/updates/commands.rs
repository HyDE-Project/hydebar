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
    /// A configuration naming a package manager the machine does not have
    /// is not a fault to report every hour; it means the bar has
    /// nothing to show.
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
fn classify(
    status: &ExitStatus,
    updates: &[Update],
    stderr: &[u8]
) -> Result<(), CheckFailure> {
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

/// Asks the `HyDE` clone how far behind upstream it is.
///
/// Only remote-tracking refs are touched: the fetch never rewrites the
/// working tree, so a clone holding local work is read, not disturbed.
pub(super) async fn check_hyde(
    clone: &str,
    branch: &str
) -> Result<(String, Vec<String>), CommandError> {
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

/// Lines of output the update keeps for the window.
const LOG_TAIL: usize = 6;

/// Shortest pause between two log publications.
///
/// The installer prints in bursts, and forwarding every line as its own
/// message would redraw the window hundreds of times for one update.
const LOG_FLUSH: std::time::Duration = std::time::Duration::from_millis(150);

/// The dialog sudo raises when it has no terminal to ask on.
///
/// The update runs without a terminal, and sudo in that position turns to
/// the helper `SUDO_ASKPASS` names — once per elevated run, with its usual
/// grace period after, unlike polkit's per-command prompting that turned a
/// long install into a hail of dialogs.
///
/// The question is asked by rofi where it exists: on a `HyDE` desktop rofi
/// already wears the theme in force, where the GTK dialogs answer to
/// nobody's palette. Zenity stays as the fallback.
const ASKPASS: &str = concat!(
    "#!/usr/bin/env bash\n",
    "if command -v rofi >/dev/null 2>&1; then\n",
    "  exec rofi -dmenu -password -p \"${1:-Password}\" </dev/null\n",
    "fi\n",
    "exec zenity --password --title='Updates' \"$@\"\n"
);

/// Writes the password dialog helper into `dir` and returns its path.
fn write_askpass(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(dir).ok()?;

    let helper = dir.join("askpass");
    std::fs::write(&helper, ASKPASS).ok()?;

    let mut permissions = std::fs::metadata(&helper).ok()?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&helper, permissions).ok()?;

    Some(helper)
}

/// The password helper the update should run with, when one is needed.
///
/// An environment that already names one is respected; otherwise the
/// zenity dialog is offered, provided zenity is installed at all.
fn askpass_helper() -> Option<std::path::PathBuf> {
    if let Some(existing) = std::env::var_os("SUDO_ASKPASS")
        && !existing.is_empty()
    {
        return Some(std::path::PathBuf::from(existing));
    }

    let path = std::env::var_os("PATH")?;

    if !std::env::split_paths(&path)
        .any(|dir| dir.join("rofi").exists() || dir.join("zenity").exists())
    {
        return None;
    }

    let dir = dirs::runtime_dir()
        .or_else(dirs::cache_dir)?
        .join("hydebar")
        .join("elevate");

    write_askpass(&dir)
}

/// Brings the `HyDE` clone up to date the way upstream documents it,
/// narrating into `publish`.
///
/// No terminal opens: the output streams into the updates window instead,
/// as the tail of the last few lines, and anything that needs elevation
/// asks through the desktop's polkit agent. The script refuses to touch a
/// clone carrying uncommitted work — the documented path is a hard reset
/// that would discard it — and the refusal arrives through the same tail.
/// A clean clone standing on another branch is simply switched: the
/// branch it left keeps its commits.
pub(super) async fn update_hyde<F>(
    clone: &str,
    branch: &str,
    publish: F
) -> Result<(), CommandError>
where
    F: FnMut(Vec<String>)
{
    stream_shell(hyde_update_script(clone, branch), publish).await
}

/// Runs `script` without a terminal, narrating its output into `publish`
/// as the tail of the last few lines.
///
/// Anything in the script that calls for elevation asks through the
/// desktop's polkit agent, and a closed stdin answers anything that would
/// have been a prompt.
async fn stream_shell<F>(script: String, mut publish: F) -> Result<(), CommandError>
where
    F: FnMut(Vec<String>)
{
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut spawner = process::Command::new("bash");
    spawner
        .arg("-c")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    if let Some(helper) = askpass_helper() {
        spawner.env("SUDO_ASKPASS", helper);
    }

    let mut child = spawner.spawn()?;

    let stdout = child.stdout.take().ok_or_else(|| {
        CommandError::Io(std::io::Error::other("the update has no output pipe"))
    })?;

    let mut lines = BufReader::new(stdout).lines();
    let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let mut last_flush: Option<std::time::Instant> = None;

    while let Ok(Some(line)) = lines.next_line().await {
        let clean = strip_ansi(&line);
        let clean = clean.trim();

        if clean.is_empty() {
            continue;
        }

        if tail.len() == LOG_TAIL {
            tail.pop_front();
        }
        tail.push_back(clean.to_owned());

        if last_flush.is_none_or(|at| at.elapsed() >= LOG_FLUSH) {
            publish(tail.iter().cloned().collect());
            last_flush = Some(std::time::Instant::now());
        }
    }

    publish(tail.into_iter().collect());

    let status = child.wait().await?;

    if !status.success() {
        return Err(CommandError::Status(status));
    }

    Ok(())
}

/// The script the in-window update runs.
fn hyde_update_script(clone: &str, branch: &str) -> String {
    let quoted_clone = shell_quote(clone);
    let quoted_branch = shell_quote(branch);

    format!(
        concat!(
            "exec 2>&1\n",
            "cd {clone} || exit 1\n",
            "if [ -n \"$(git status --porcelain)\" ]; then\n",
            "  echo 'The clone carries uncommitted work; update it by hand.'\n",
            "  exit 1\n",
            "fi\n",
            "git fetch --update-shallow origin {branch} || exit 1\n",
            "if [ \"$(git rev-parse --abbrev-ref HEAD)\" != {branch} ]; then\n",
            "  git checkout -q {branch} 2>/dev/null",
            " || git checkout -q -B {branch} 'origin/'{branch} || exit 1\n",
            "fi\n",
            "git reset --hard 'origin/'{branch} \\\n",
            "  && ./Scripts/install.sh -r\n"
        ),
        clone = quoted_clone,
        branch = quoted_branch
    )
}

/// Drops the colour and cursor sequences the installer prints.
fn strip_ansi(line: &str) -> String {
    let mut cleaned = String::with_capacity(line.len());
    let mut chars = line.chars();

    while let Some(current) = chars.next() {
        if current == '\u{1b}' {
            if chars.next() == Some('[') {
                for escaped in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&escaped) {
                        break;
                    }
                }
            }

            continue;
        }

        if !current.is_control() {
            cleaned.push(current);
        }
    }

    cleaned
}

/// Wraps `value` so a shell reads it as one literal word.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Applies the configured update command, narrating into `publish`.
///
/// The command streams into the updates window like the `HyDE` update does;
/// a command that opens its own terminal still works, it just has nothing
/// to narrate.
pub(super) async fn apply_updates<F>(command: &str, publish: F) -> Result<(), CommandError>
where
    F: FnMut(Vec<String>)
{
    stream_shell(format!("exec 2>&1\n{command}\n"), publish).await
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

    /// `checkupdates; paru -Qua` ends in a query that fails when the AUR
    /// has nothing, and silence from both is the shape of
    /// "everything is current".
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

    #[test]
    fn hyde_commits_skip_blank_lines() {
        let commits = parse_hyde_commits("fix: one\n\n  feat: two  \n");

        assert_eq!(commits, vec!["fix: one".to_owned(), "feat: two".to_owned()]);
    }

    #[test]
    fn a_quoted_path_survives_an_apostrophe() {
        assert_eq!(
            shell_quote("/home/o'brien/HyDE"),
            "'/home/o'\\''brien/HyDE'"
        );
    }

    /// The documented update is a hard reset; the script must refuse to
    /// run it against a clone holding uncommitted work, while a clean
    /// clone on another branch is switched rather than refused.
    #[test]
    fn the_update_script_guards_local_work() {
        let script = hyde_update_script("/home/user/HyDE", "master");

        assert!(script.contains("git status --porcelain"));
        assert!(script.contains("git checkout -q 'master'"));
        assert!(script.contains("cd '/home/user/HyDE'"));
        assert!(script.contains("reset --hard 'origin/''master'"));
    }

    #[test]
    fn the_script_follows_the_chosen_branch() {
        let script = hyde_update_script("/home/user/HyDE", "dev");

        assert!(script.contains("fetch --update-shallow origin 'dev'"));
        assert!(script.contains("reset --hard 'origin/''dev'"));
    }

    #[test]
    fn the_password_dialog_lands_executable() {
        let dir = std::env::temp_dir().join(format!(
            "hydebar-askpass-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));

        let helper = write_askpass(&dir).expect("the helper is written");

        let mode = {
            use std::os::unix::fs::PermissionsExt;

            std::fs::metadata(&helper)
                .expect("the helper exists")
                .permissions()
                .mode()
        };
        let content = std::fs::read_to_string(&helper).expect("the helper reads back");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(mode & 0o111, 0o111);
        assert!(content.contains("rofi -dmenu -password"));
        assert!(content.contains("zenity --password"));
    }

    #[test]
    fn colour_sequences_are_stripped_from_the_log() {
        assert_eq!(
            strip_ansi("\u{1b}[1;32m[OK]\u{1b}[0m restored\t "),
            "[OK] restored "
        );
        assert_eq!(strip_ansi("plain"), "plain");
    }
}
