//! Dialect the compositor accepts its dispatch commands in.
//!
//! Hyprland moved its configuration and its dispatchers to a scripted syntax:
//! a compositor built after the move rejects `workspace 1` and expects
//! `hl.dsp.focus{workspace=1}`, while an older one rejects the scripted form.
//! Rather than pin a version number that will age badly, the first dispatch
//! tries the scripted form and remembers which one the compositor answered
//! `ok` to.

use std::sync::atomic::{AtomicU8, Ordering};

use hydebar_proto::ports::hyprland::{HyprlandMonitorSelector, HyprlandWorkspaceSelector};
use hyprland::{
    dispatch::{Dispatch, DispatchType},
    error::HyprError
};

/// Not resolved yet.
const UNKNOWN: u8 = 0;
/// The compositor speaks the scripted syntax.
const SCRIPTED: u8 = 1;
/// The compositor speaks the legacy syntax.
const LEGACY: u8 = 2;

/// Which syntax the compositor answered `ok` to, resolved once per process.
static DIALECT: AtomicU8 = AtomicU8::new(UNKNOWN);

/// Syntax a dispatch command is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// `hl.dsp.focus{workspace=1}`, understood by a scripted compositor.
    Scripted,
    /// `workspace 1`, understood by a compositor from before the move.
    Legacy
}

impl Dialect {
    /// Dialect resolved so far, if any dispatch has succeeded yet.
    fn cached() -> Option<Self> {
        match DIALECT.load(Ordering::Relaxed) {
            SCRIPTED => Some(Self::Scripted),
            LEGACY => Some(Self::Legacy),
            _ => None
        }
    }

    /// Remembers this dialect as the one the compositor accepts.
    fn remember(self) {
        DIALECT.store(
            match self {
                Self::Scripted => SCRIPTED,
                Self::Legacy => LEGACY
            },
            Ordering::Relaxed
        );
    }

    /// Order the dialects are attempted in while none is known yet.
    fn attempts() -> [Self; 2] {
        match Self::cached() {
            Some(Self::Legacy) => [Self::Legacy, Self::Scripted],
            _ => [Self::Scripted, Self::Legacy]
        }
    }
}

/// Renders a workspace selector as a scripted argument.
fn scripted_workspace(workspace: &HyprlandWorkspaceSelector) -> String {
    match workspace {
        HyprlandWorkspaceSelector::Id(id) => id.to_string(),
        HyprlandWorkspaceSelector::Name(name) => format!("\"{name}\"")
    }
}

/// Renders a monitor selector as a scripted argument.
fn scripted_monitor(monitor: &HyprlandMonitorSelector) -> String {
    match monitor {
        HyprlandMonitorSelector::Id(id) => id.to_string(),
        HyprlandMonitorSelector::Name(name) => format!("\"{name}\"")
    }
}

/// Command switching the session to `workspace`, in `dialect`.
pub fn focus_workspace(dialect: Dialect, workspace: &HyprlandWorkspaceSelector) -> String {
    match dialect {
        Dialect::Scripted => format!(
            "hl.dsp.focus{{workspace={}}}",
            scripted_workspace(workspace)
        ),
        Dialect::Legacy => match workspace {
            HyprlandWorkspaceSelector::Id(id) => format!("workspace {id}"),
            HyprlandWorkspaceSelector::Name(name) => format!("workspace name:{name}")
        }
    }
}

/// Command focusing `monitor`, in `dialect`.
pub fn focus_monitor(dialect: Dialect, monitor: &HyprlandMonitorSelector) -> String {
    match dialect {
        Dialect::Scripted => format!("hl.dsp.focus{{monitor={}}}", scripted_monitor(monitor)),
        Dialect::Legacy => match monitor {
            HyprlandMonitorSelector::Id(id) => format!("focusmonitor {id}"),
            HyprlandMonitorSelector::Name(name) => format!("focusmonitor {name}")
        }
    }
}

/// Command toggling the special workspace `name`, in `dialect`.
pub fn toggle_special_workspace(dialect: Dialect, name: &str) -> String {
    match dialect {
        Dialect::Scripted => format!("hl.dsp.workspace.toggle_special{{name=\"{name}\"}}"),
        Dialect::Legacy => format!("togglespecialworkspace {name}")
    }
}

/// Command focusing the window at `address`, in `dialect`.
pub fn focus_window(dialect: Dialect, address: &str) -> String {
    match dialect {
        Dialect::Scripted => format!("hl.dsp.focus{{window=\"address:{address}\"}}"),
        Dialect::Legacy => format!("focuswindow address:{address}")
    }
}

/// Sends the command `build` renders, trying each dialect until one is
/// accepted.
///
/// The dialect that answers `ok` is remembered, so every later dispatch costs a
/// single round trip.
///
/// # Errors
/// Returns the error of the last attempt when no dialect is accepted.
pub fn dispatch_in_any_dialect<F>(build: F) -> Result<(), HyprError>
where
    F: Fn(Dialect) -> String
{
    let mut last = None;

    for dialect in Dialect::attempts() {
        let command = build(dialect);

        match Dispatch::call(DispatchType::Custom(&command, "")) {
            Ok(()) => {
                dialect.remember();
                return Ok(());
            }
            Err(err) => last = Some(err)
        }
    }

    Err(last.unwrap_or_else(|| HyprError::NotOkDispatch("no dialect was attempted".to_owned())))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    /// Serialises the tests that read or write the remembered dialect.
    ///
    /// The dialect is process wide, so two of these running at once would each
    /// see the other's writes and fail for reasons that have nothing to do with
    /// what they assert.
    static REMEMBERED: Mutex<()> = Mutex::new(());

    /// Takes the lock and forgets whatever an earlier test remembered.
    fn with_a_forgotten_dialect() -> MutexGuard<'static, ()> {
        let guard = REMEMBERED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        DIALECT.store(UNKNOWN, Ordering::Relaxed);

        guard
    }

    #[test]
    fn a_scripted_workspace_is_addressed_by_id() {
        assert_eq!(
            (focus_workspace(Dialect::Scripted, &HyprlandWorkspaceSelector::Id(3))),
            "hl.dsp.focus{workspace=3}"
        );
    }

    #[test]
    fn a_scripted_workspace_name_is_quoted() {
        assert_eq!(
            (focus_workspace(
                Dialect::Scripted,
                &HyprlandWorkspaceSelector::Name("code".to_owned())
            )),
            "hl.dsp.focus{workspace=\"code\"}"
        );
    }

    #[test]
    fn the_legacy_form_keeps_the_old_spelling() {
        assert_eq!(
            (focus_workspace(Dialect::Legacy, &HyprlandWorkspaceSelector::Id(3))),
            "workspace 3"
        );
        assert_eq!(
            (focus_workspace(
                Dialect::Legacy,
                &HyprlandWorkspaceSelector::Name("code".to_owned())
            )),
            "workspace name:code"
        );
    }

    #[test]
    fn a_monitor_is_rendered_in_both_dialects() {
        assert_eq!(
            (focus_monitor(
                Dialect::Scripted,
                &HyprlandMonitorSelector::Name("DP-1".to_owned())
            )),
            "hl.dsp.focus{monitor=\"DP-1\"}"
        );
        assert_eq!(
            (focus_monitor(Dialect::Legacy, &HyprlandMonitorSelector::Id(1))),
            "focusmonitor 1"
        );
    }

    #[test]
    fn a_window_address_is_rendered_in_both_dialects() {
        assert_eq!(
            (focus_window(Dialect::Scripted, "0xabc123")),
            "hl.dsp.focus{window=\"address:0xabc123\"}"
        );
        assert_eq!(
            (focus_window(Dialect::Legacy, "0xabc123")),
            "focuswindow address:0xabc123"
        );
    }

    #[test]
    fn a_special_workspace_is_rendered_in_both_dialects() {
        assert_eq!(
            (toggle_special_workspace(Dialect::Scripted, "magic")),
            "hl.dsp.workspace.toggle_special{name=\"magic\"}"
        );
        assert_eq!(
            (toggle_special_workspace(Dialect::Legacy, "magic")),
            "togglespecialworkspace magic"
        );
    }

    #[test]
    fn the_scripted_dialect_is_tried_first_while_none_is_known() {
        let _guard = with_a_forgotten_dialect();

        assert_eq!(Dialect::attempts(), [Dialect::Scripted, Dialect::Legacy]);
    }

    #[test]
    fn a_remembered_legacy_dialect_is_tried_first() {
        let _guard = with_a_forgotten_dialect();
        Dialect::Legacy.remember();

        assert_eq!(Dialect::attempts(), [Dialect::Legacy, Dialect::Scripted]);
        assert_eq!(Dialect::cached(), Some(Dialect::Legacy));

        DIALECT.store(UNKNOWN, Ordering::Relaxed);
    }
}
