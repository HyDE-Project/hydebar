//! Test doubles available to internal tests and, via the `test-utils`
//! feature, to other crates' tests.
#![cfg(any(test, feature = "test-utils"))]
#![cfg_attr(coverage_nightly, coverage(off))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test double fails loudly instead of masking a broken fixture"
)]

use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering}
};

use hydebar_proto::ports::hyprland::{
    HyprlandClientInfo, HyprlandError, HyprlandEventStream, HyprlandKeyboardEvent,
    HyprlandKeyboardState, HyprlandMonitorInfo, HyprlandMonitorSelector, HyprlandPort,
    HyprlandWindowEvent, HyprlandWindowInfo, HyprlandWorkspaceEvent, HyprlandWorkspaceInfo,
    HyprlandWorkspaceSelector, HyprlandWorkspaceSnapshot
};
use tokio_stream;

/// A compositor that answers whatever a test told it to, and counts what it was
/// asked.
#[derive(Debug)]
pub struct MockHyprlandPort {
    /// What it answers when asked for the focused window.
    pub active_window:          Mutex<Option<HyprlandWindowInfo>>,
    /// What it answers when asked for the workspaces.
    pub workspace_snapshot:     Mutex<HyprlandWorkspaceSnapshot>,
    /// What it answers when asked about the keyboard.
    pub keyboard_state:         Mutex<HyprlandKeyboardState>,
    /// What it answers when asked for the windows.
    pub clients_snapshot:       Mutex<Vec<HyprlandClientInfo>>,
    /// How often it was asked to switch workspace.
    pub change_workspace_calls: AtomicUsize,
    /// How often it was asked to open a special workspace.
    pub toggle_special_calls:   AtomicUsize,
    /// How often it was asked to step the keyboard layout.
    pub switch_layout_calls:    AtomicUsize,
    /// How often it was asked to focus a window.
    pub focus_window_calls:     AtomicUsize
}

impl Default for MockHyprlandPort {
    fn default() -> Self {
        Self {
            active_window:          Mutex::new(Some(HyprlandWindowInfo {
                title: "Mock Window".into(),
                class: "MockClass".into()
            })),
            workspace_snapshot:     Mutex::new(HyprlandWorkspaceSnapshot {
                monitors:            vec![HyprlandMonitorInfo {
                    id:                   0,
                    name:                 "MockMonitor".into(),
                    active_workspace_id:  Some(1),
                    special_workspace_id: None
                }],
                workspaces:          vec![HyprlandWorkspaceInfo {
                    id:           1,
                    name:         "1".into(),
                    monitor_id:   Some(0),
                    monitor_name: "MockMonitor".into(),
                    window_count: 0
                }],
                active_workspace_id: Some(1)
            }),
            keyboard_state:         Mutex::new(HyprlandKeyboardState {
                active_layout:        "us".into(),
                has_multiple_layouts: true,
                active_submap:        Some("resize".into())
            }),
            clients_snapshot:       Mutex::new(Vec::new()),
            change_workspace_calls: AtomicUsize::new(0),
            toggle_special_calls:   AtomicUsize::new(0),
            switch_layout_calls:    AtomicUsize::new(0),
            focus_window_calls:     AtomicUsize::new(0)
        }
    }
}

impl MockHyprlandPort {
    /// # Panics
    ///
    /// Panics when the active window lock was poisoned by a panicking thread.
    #[must_use]
    pub fn with_active_window(title: &str, class: &str) -> Self {
        let port = Self::default();
        *port
            .active_window
            .lock()
            .expect("poisoned active window lock") = Some(HyprlandWindowInfo {
            title: title.into(),
            class: class.into()
        });
        port
    }

    /// How often it was asked to switch workspace.
    pub fn workspace_calls(&self) -> usize {
        self.change_workspace_calls.load(Ordering::SeqCst)
    }

    /// How often it was asked to open a special workspace.
    pub fn toggle_special_calls(&self) -> usize {
        self.toggle_special_calls.load(Ordering::SeqCst)
    }

    /// How often it was asked to step the keyboard layout.
    pub fn switch_layout_calls(&self) -> usize {
        self.switch_layout_calls.load(Ordering::SeqCst)
    }
}

impl HyprlandPort for MockHyprlandPort {
    fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
        Ok(Box::pin(tokio_stream::pending()))
    }

    fn workspace_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
        Ok(Box::pin(tokio_stream::pending()))
    }

    fn keyboard_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
        Ok(Box::pin(tokio_stream::pending()))
    }

    fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError> {
        Ok(self
            .active_window
            .lock()
            .expect("poisoned active window lock")
            .clone())
    }

    fn workspace_snapshot(&self) -> Result<HyprlandWorkspaceSnapshot, HyprlandError> {
        Ok(self
            .workspace_snapshot
            .lock()
            .expect("poisoned workspace snapshot lock")
            .clone())
    }

    fn change_workspace(&self, _: HyprlandWorkspaceSelector) -> Result<(), HyprlandError> {
        self.change_workspace_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn focus_and_toggle_special_workspace(
        &self,
        _: HyprlandMonitorSelector,
        _: &str
    ) -> Result<(), HyprlandError> {
        self.toggle_special_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn keyboard_state(&self) -> Result<HyprlandKeyboardState, HyprlandError> {
        Ok(self
            .keyboard_state
            .lock()
            .expect("poisoned keyboard state lock")
            .clone())
    }

    fn switch_keyboard_layout(&self) -> Result<(), HyprlandError> {
        self.switch_layout_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn clients_snapshot(&self) -> Result<Vec<HyprlandClientInfo>, HyprlandError> {
        Ok(self
            .clients_snapshot
            .lock()
            .expect("poisoned clients snapshot lock")
            .clone())
    }

    fn focus_window(&self, _: &str) -> Result<(), HyprlandError> {
        self.focus_window_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
