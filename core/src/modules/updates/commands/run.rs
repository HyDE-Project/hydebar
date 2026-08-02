//! The two updates the menu can start, each narrating into the window.

use super::{error::CommandError, stream::stream_shell};

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
pub async fn update_hyde<F>(clone: &str, branch: &str, publish: F) -> Result<(), CommandError>
where
    F: FnMut(Vec<String>)
{
    stream_shell(hyde_update_script(clone, branch), publish).await
}

/// Applies the configured update command, narrating into `publish`.
///
/// The command streams into the updates window like the `HyDE` update does;
/// a command that opens its own terminal still works, it just has nothing
/// to narrate.
pub async fn apply_updates<F>(command: &str, publish: F) -> Result<(), CommandError>
where
    F: FnMut(Vec<String>)
{
    stream_shell(format!("exec 2>&1\n{command}\n"), publish).await
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

/// Wraps `value` so a shell reads it as one literal word.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
