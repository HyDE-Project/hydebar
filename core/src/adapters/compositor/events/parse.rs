//! Reading one announcement line into the event it stands for.

use super::CompositorEvent;

/// What separates an event's name from what it carries.
const SEPARATOR: &str = ">>";

/// Reads one line the compositor wrote.
///
/// Returns nothing for a line that carries no separator, and for every event
/// the bar does not draw — which is most of them: the compositor announces
/// layers opening, groups locking and windows being pinned, and the bar has no
/// readout that changes for any of it.
pub(super) fn line(line: &str) -> Option<CompositorEvent> {
    let (name, data) = line.split_once(SEPARATOR)?;

    Some(match name {
        "workspacev2" => CompositorEvent::WorkspaceChanged,
        "createworkspacev2" => CompositorEvent::WorkspaceAdded,
        "destroyworkspacev2" => CompositorEvent::WorkspaceRemoved,
        "moveworkspacev2" => CompositorEvent::WorkspaceMoved,
        "focusedmon" => CompositorEvent::ActiveMonitorChanged,
        "monitorremoved" => CompositorEvent::MonitorRemoved,
        "openwindow" => CompositorEvent::WindowOpened,
        "closewindow" => CompositorEvent::WindowClosed,
        "movewindowv2" => CompositorEvent::WindowMoved,
        "activewindowv2" => CompositorEvent::ActiveWindowChanged,
        "windowtitlev2" => CompositorEvent::WindowTitleChanged,
        "activelayout" => CompositorEvent::LayoutChanged,
        "configreloaded" => CompositorEvent::ConfigReloaded,
        "submap" => CompositorEvent::SubmapChanged(non_empty(data)),
        "urgent" => CompositorEvent::Urgent {
            address: data.trim().to_owned()
        },
        "activespecial" => special(data),
        _ => return None
    })
}

/// Tells a special workspace being opened from the last one being closed.
///
/// The compositor announces both the same way — `workspace,monitor` — and
/// leaves the workspace empty when what happened is that the monitor no longer
/// has one.
fn special(data: &str) -> CompositorEvent {
    let workspace = data.split(',').next().unwrap_or_default();

    if workspace.trim().is_empty() {
        CompositorEvent::SpecialRemoved
    } else {
        CompositorEvent::SpecialChanged
    }
}

/// The text, unless it is blank, which is how the compositor says "none".
fn non_empty(data: &str) -> Option<String> {
    let data = data.trim();

    (!data.is_empty()).then(|| data.to_owned())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{CompositorEvent, line};

    #[test]
    fn a_workspace_switch_is_read() {
        assert_eq!(
            line("workspacev2>>3,three"),
            Some(CompositorEvent::WorkspaceChanged)
        );
    }

    #[test]
    fn a_line_carrying_no_separator_is_read_as_nothing() {
        assert_eq!(line("workspacev2"), None);
        assert_eq!(line(""), None);
    }

    #[test]
    fn an_event_the_bar_draws_nothing_for_is_read_as_nothing() {
        assert_eq!(line("openlayer>>notifications"), None);
        assert_eq!(line("lockgroups>>1"), None);
    }

    #[test]
    fn an_urgent_window_carries_the_address_it_names() {
        assert_eq!(
            line("urgent>>0x55a1"),
            Some(CompositorEvent::Urgent {
                address: String::from("0x55a1")
            })
        );
    }

    #[test]
    fn a_submap_being_entered_carries_its_name() {
        assert_eq!(
            line("submap>>resize"),
            Some(CompositorEvent::SubmapChanged(Some(String::from("resize"))))
        );
    }

    #[test]
    fn a_submap_being_left_carries_nothing() {
        assert_eq!(line("submap>>"), Some(CompositorEvent::SubmapChanged(None)));
    }

    #[test]
    fn a_special_workspace_opening_is_told_from_one_closing() {
        assert_eq!(
            line("activespecial>>special:magic,DP-1"),
            Some(CompositorEvent::SpecialChanged)
        );
        assert_eq!(
            line("activespecial>>,DP-1"),
            Some(CompositorEvent::SpecialRemoved)
        );
    }

    #[test]
    fn a_reload_carries_nothing_and_is_still_read() {
        assert_eq!(
            line("configreloaded>>"),
            Some(CompositorEvent::ConfigReloaded)
        );
    }

    #[test]
    fn a_title_holding_the_separator_does_not_confuse_the_reader() {
        assert_eq!(
            line("windowtitlev2>>0x1,a >> b"),
            Some(CompositorEvent::WindowTitleChanged)
        );
    }
}
