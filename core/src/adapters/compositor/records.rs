//! The compositor's own answers, read as only what the bar asks about.
//!
//! The compositor describes a monitor with sixteen fields and a window with
//! twenty; the bar draws eight of them altogether. Only those are modelled
//! here, and serde ignores the rest — so a release that adds a field, renames
//! one the bar never reads, or changes a type it never parses goes unnoticed
//! instead of failing the whole answer.

use serde::Deserialize;

/// The workspace an answer names in passing, as a monitor or a window does.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRef {
    /// Identifier of the workspace, or zero where there is none.
    pub id: i32
}

/// One screen the compositor is driving.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    /// Identifier the compositor addresses the screen by.
    pub id:                i64,
    /// Name the screen answers to, as `DP-1`.
    pub name:              String,
    /// The workspace the screen is showing.
    #[serde(rename = "activeWorkspace")]
    pub active_workspace:  WorkspaceRef,
    /// The special workspace drawn over it, if one is open.
    #[serde(rename = "specialWorkspace")]
    pub special_workspace: WorkspaceRef
}

/// One workspace the compositor holds.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// Identifier the compositor addresses the workspace by.
    pub id:         i32,
    /// Name the workspace carries, which is its number unless it was named.
    pub name:       String,
    /// Name of the screen the workspace sits on.
    pub monitor:    String,
    /// Identifier of that screen, absent while the workspace has no screen.
    #[serde(rename = "monitorID")]
    pub monitor_id: Option<i64>,
    /// How many windows the workspace holds.
    pub windows:    u16
}

/// One window the compositor is drawing.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Client {
    /// Address the compositor addresses the window by.
    pub address:          String,
    /// Class the application registers under.
    pub class:            String,
    /// Title the window carries.
    pub title:            String,
    /// The workspace the window sits on.
    pub workspace:        WorkspaceRef,
    /// Whether the window floats rather than tiles.
    pub floating:         bool,
    /// Whether the window has a surface on screen at all.
    pub mapped:           bool,
    /// Where the top left corner of the window is, in screen pixels.
    pub at:               (i16, i16),
    /// How wide and how tall the window is, in screen pixels.
    pub size:             (i16, i16),
    /// Place in the focus order; zero is the window holding focus.
    #[serde(rename = "focusHistoryID")]
    pub focus_history_id: i8
}

/// One keyboard the compositor has.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Keyboard {
    /// Whether this is the keyboard the session follows.
    pub main:          bool,
    /// The layout the keyboard is currently in.
    pub active_keymap: String
}

/// Every input device the compositor has, of which the bar reads keyboards.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Devices {
    /// The keyboards attached to the session.
    pub keyboards: Vec<Keyboard>
}

/// One configuration option, as the compositor answers it.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Option_ {
    /// The value as written, which is what a list of layouts arrives as.
    #[serde(rename = "str", default)]
    pub text: String
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{Client, Devices, Monitor, Option_, Workspace};

    #[test]
    fn a_monitor_is_read_from_the_fields_the_bar_draws() {
        let monitor: Monitor = serde_json::from_str(
            r#"{"id":0,"name":"DP-1","description":"a screen","width":2560,
                "activeWorkspace":{"id":3,"name":"3"},
                "specialWorkspace":{"id":0,"name":""}}"#
        )
        .expect("the answer reads");

        assert_eq!(monitor.name, "DP-1");
        assert_eq!(monitor.active_workspace.id, 3);
        assert_eq!(monitor.special_workspace.id, 0);
    }

    #[test]
    fn a_field_the_bar_never_reads_does_not_fail_the_answer() {
        let monitor: Monitor = serde_json::from_str(
            r#"{"id":0,"name":"DP-1","somethingNew":{"nested":[1,2,3]},
                "activeWorkspace":{"id":1,"name":"1"},
                "specialWorkspace":{"id":0,"name":""}}"#
        )
        .expect("an unknown field is passed over");

        assert_eq!(monitor.id, 0);
    }

    #[test]
    fn a_workspace_without_a_screen_reads_as_one() {
        let workspace: Workspace = serde_json::from_str(
            r#"{"id":9,"name":"nine","monitor":"","monitorID":null,"windows":0}"#
        )
        .expect("the answer reads");

        assert_eq!(workspace.monitor_id, None);
        assert_eq!(workspace.windows, 0);
    }

    #[test]
    fn a_window_is_read_with_its_place_and_its_size() {
        let client: Client = serde_json::from_str(
            r#"{"address":"0x1","class":"kitty","title":"a shell","at":[10,20],
                "size":[800,600],"workspace":{"id":2,"name":"2"},"floating":false,
                "mapped":true,"focusHistoryID":0}"#
        )
        .expect("the answer reads");

        assert_eq!(client.at, (10, 20));
        assert_eq!(client.size, (800, 600));
        assert_eq!(client.focus_history_id, 0);
    }

    #[test]
    fn the_keyboards_are_read_out_of_the_devices() {
        let devices: Devices = serde_json::from_str(
            r#"{"mice":[{"address":"0x2"}],
                "keyboards":[{"main":true,"active_keymap":"English (US)"}]}"#
        )
        .expect("the answer reads");

        assert_eq!(devices.keyboards.len(), 1);
        assert!(devices.keyboards[0].main);
    }

    #[test]
    fn an_option_is_read_by_the_text_it_was_written_as() {
        let option: Option_ =
            serde_json::from_str(r#"{"option":"input:kb_layout","str":"us,ru","set":true}"#)
                .expect("the answer reads");

        assert_eq!(option.text, "us,ru");
    }
}
