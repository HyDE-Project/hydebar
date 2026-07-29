//! Taking the notification bus from the daemon that already holds it.
//!
//! The bus name has a single owner and the polite mechanisms do not work here:
//! a request with the replacement flags only succeeds when the incumbent asked
//! to be replaceable, and the daemons a session starts do not. The request is
//! then queued forever while the old daemon keeps painting, which is the exact
//! opposite of what the user asked for by choosing the bar's own popups.
//!
//! So the bar ends the incumbent — but only when it can prove what it is
//! ending. The proof is [`owns_unit`]: the unit named by the holder's control
//! group must have that very process as its main one. A daemon started as its
//! own service passes; a daemon that happens to share a session application
//! unit with, say, a terminal does not, and is left alone. Stopping such a unit
//! would take everything else in it down as well, which is not a hypothetical:
//! it took a terminal down once.

use std::{fs, process::Command};

use log::{debug, warn};
use zbus::{Connection, fdo::DBusProxy, names::BusName};

/// Name a notification server answers to.
const NOTIFICATIONS: &str = "org.freedesktop.Notifications";

/// Service manager of the session the daemon belongs to.
const SERVICE_MANAGER: &str = "systemctl";

/// Unit the bar may stop to take the notification bus, if there is one.
///
/// Answers `None` whenever the holder cannot be pinned to a service of its own,
/// which is the case the bar must never act on.
pub async fn replaceable_unit(connection: &Connection) -> Option<String> {
    let proxy = DBusProxy::new(connection).await.ok()?;
    let name = BusName::try_from(NOTIFICATIONS).ok()?;
    let holder = proxy.get_connection_unix_process_id(name).await.ok()?;
    let unit = unit_of(holder)?;

    owns_unit(&unit, holder).then_some(unit)
}

/// Service named by the control group of `pid`.
///
/// The control group path is what the service manager itself uses to say which
/// unit a process belongs to, and reading it needs no process of its own.
fn unit_of(pid: u32) -> Option<String> {
    let text = fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;

    named_unit(&text)
}

/// Last service component of a control group path.
fn named_unit(cgroup: &str) -> Option<String> {
    cgroup
        .lines()
        .flat_map(|line| line.rsplit('/'))
        .find(|part| part.ends_with(".service"))
        .map(ToOwned::to_owned)
}

/// Whether `unit` exists to run `pid` and nothing else.
///
/// A service whose main process is the holder is that daemon's own service, and
/// ending it ends the daemon. A unit that merely contains the holder among
/// other processes — a session application unit holding a whole terminal, for
/// instance — reports a different main process, and is refused.
fn owns_unit(unit: &str, pid: u32) -> bool {
    let Ok(output) = Command::new(SERVICE_MANAGER)
        .args(["--user", "show", "-p", "MainPID", "--value", unit])
        .output()
    else {
        return false;
    };

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        == Ok(pid)
}

/// Stops `unit`, reporting whether the service manager took the instruction.
///
/// Stopping rather than killing is deliberate: a killed service is started
/// again by the manager that owns it, and the bar would be fighting a
/// supervisor rather than replacing a daemon.
pub fn stop(unit: &str) -> bool {
    match Command::new(SERVICE_MANAGER)
        .args(["--user", "stop", unit])
        .status()
    {
        Ok(status) if status.success() => {
            debug!("stopped {unit} so the bar can serve notifications");
            true
        }
        Ok(status) => {
            warn!("the session refused to stop {unit}: {status}");
            false
        }
        Err(err) => {
            warn!("the session service manager could not be reached: {err}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_control_group_path_names_its_service() {
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/dunst.service\n";

        assert_eq!(named_unit(cgroup), Some("dunst.service".to_owned()));
    }

    #[test]
    fn the_innermost_service_wins() {
        // the path names the session manager as well, and stopping that would
        // end the whole session
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/dunst.service\n";

        assert_ne!(named_unit(cgroup), Some("user@1000.service".to_owned()));
    }

    #[test]
    fn a_process_outside_any_service_names_none() {
        assert_eq!(named_unit("0::/\n"), None);
        assert_eq!(named_unit(""), None);
    }

    #[test]
    fn a_scope_is_not_a_service() {
        // a scope holds processes the manager did not start, so there is
        // nothing there the bar may end
        let cgroup = "0::/user.slice/user-1000.slice/session-3.scope\n";

        assert_eq!(named_unit(cgroup), None);
    }

    #[test]
    fn the_bar_addresses_the_session_manager_and_not_the_system_one() {
        assert_eq!(SERVICE_MANAGER, "systemctl");
    }
}
