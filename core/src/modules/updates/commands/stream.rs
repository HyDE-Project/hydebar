//! Running a script without a terminal, narrating its output as a tail.

use std::process::Stdio;

use tokio::process;

use super::{elevate::askpass_helper, error::CommandError};

/// Lines of output the update keeps for the window.
const LOG_TAIL: usize = 6;

/// Shortest pause between two log publications.
///
/// The installer prints in bursts, and forwarding every line as its own
/// message would redraw the window hundreds of times for one update.
const LOG_FLUSH: std::time::Duration = std::time::Duration::from_millis(150);

/// Runs `script` without a terminal, narrating its output into `publish`
/// as the tail of the last few lines.
///
/// Anything in the script that calls for elevation asks through the
/// desktop's polkit agent, and a closed stdin answers anything that would
/// have been a prompt.
pub(super) async fn stream_shell<F>(script: String, mut publish: F) -> Result<(), CommandError>
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

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CommandError::Io(std::io::Error::other("the update has no output pipe")))?;

    let mut lines = BufReader::new(stdout).lines();
    let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let mut last_flush: Option<std::time::Instant> = None;

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(err) => {
                log::error!("reading the update output failed: {err}");
                if tail.len() == LOG_TAIL {
                    tail.pop_front();
                }
                tail.push_back(format!("[the output stream broke: {err}]"));
                break;
            }
        };

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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn colour_sequences_are_stripped_from_the_log() {
        assert_eq!(
            strip_ansi("\u{1b}[1;32m[OK]\u{1b}[0m restored\t "),
            "[OK] restored "
        );
        assert_eq!(strip_ansi("plain"), "plain");
    }
}
