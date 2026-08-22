//! Who is at the machine, and what they are sitting in front of.
//!
//! None of this moves while the bar runs: a session does not change its user,
//! its host or the desktop it was started under. It is read once and answered
//! from memory ever after.

use std::{fs, sync::LazyLock};

/// The session, as the machine describes it to the programs in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Who {
    /// Who the session belongs to.
    pub user:    Option<String>,
    /// What the machine calls itself.
    pub host:    Option<String>,
    /// The desktop the session was started under.
    pub desktop: Option<String>,
    /// Whether the session is on Wayland, X or a plain console.
    pub seat:    Option<String>,
    /// The shell the user's commands are run through.
    pub shell:   Option<String>
}

static WHO: LazyLock<Who> = LazyLock::new(read);

/// The session, read once and answered from memory ever after.
#[must_use]
pub fn who() -> &'static Who {
    &WHO
}

/// Reads the session out of the environment and the kernel.
fn read() -> Who {
    Who {
        user:    named("USER").or_else(|| named("LOGNAME")),
        host:    fs::read_to_string("/proc/sys/kernel/hostname")
            .ok()
            .map(|host| host.trim().to_owned())
            .filter(|host| !host.is_empty()),
        desktop: named("XDG_CURRENT_DESKTOP").map(|desktop| {
            desktop
                .split(':')
                .next()
                .unwrap_or(desktop.as_str())
                .to_owned()
        }),
        seat:    named("XDG_SESSION_TYPE"),
        shell:   named("SHELL").map(|shell| {
            shell
                .rsplit('/')
                .next()
                .unwrap_or(shell.as_str())
                .to_owned()
        })
    }
}

/// One environment variable, absent when it is unset or empty.
fn named(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_machine_answers_what_it_calls_itself() {
        assert!(who().host.is_some(), "the kernel names the host");
    }

    #[test]
    fn an_unset_name_reads_as_nothing() {
        assert!(named("HYDEBAR_A_NAME_NOTHING_SETS").is_none());
    }

    #[test]
    fn the_reading_is_the_same_one_every_time() {
        assert!(std::ptr::eq(who(), who()));
    }
}
