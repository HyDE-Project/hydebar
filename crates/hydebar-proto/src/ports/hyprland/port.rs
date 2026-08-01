//! The Hyprland port contract itself, together with the stream type its
//! subscriptions hand back.

use std::pin::Pin;

use tokio_stream::Stream;

use super::{
    HyprlandClientInfo, HyprlandError, HyprlandKeyboardEvent, HyprlandKeyboardState,
    HyprlandMonitorSelector, HyprlandWindowEvent, HyprlandWindowInfo, HyprlandWorkspaceEvent,
    HyprlandWorkspaceSelector, HyprlandWorkspaceSnapshot
};

/// Stream type alias used for Hyprland event subscriptions.
pub type HyprlandEventStream<E> =
    Pin<Box<dyn Stream<Item = Result<E, HyprlandError>> + Send + 'static>>;

/// Abstraction over Hyprland-specific functionality required by Hydebar
/// modules.
///
/// Backends are expected to provide retry/timeout behaviour and surface errors
/// using [`HyprlandError`]. All methods must be thread-safe.
///
/// # Examples
/// ```ignore
/// use std::sync::Arc;
/// use hydebar_proto::ports::hyprland::{
///     HyprlandEventStream, HyprlandKeyboardEvent, HyprlandKeyboardState, HyprlandMonitorSelector,
///     HyprlandPort, HyprlandWorkspaceEvent, HyprlandWorkspaceSelector, HyprlandWindowEvent,
/// };
///
/// struct DummyPort;
///
/// impl HyprlandPort for DummyPort {
///     fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
///         Err(HyprlandError::unsupported("window_events"))
///     }
///
///     fn workspace_events(
///         &self,
///     ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
///         Err(HyprlandError::unsupported("workspace_events"))
///     }
///
///     fn keyboard_events(
///         &self,
///     ) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
///         Err(HyprlandError::unsupported("keyboard_events"))
///     }
///
///     fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError> {
///         Err(HyprlandError::unsupported("active_window"))
///     }
///
///     fn workspace_snapshot(&self) -> Result<HyprlandWorkspaceSnapshot, HyprlandError> {
///         Err(HyprlandError::unsupported("workspace_snapshot"))
///     }
///
///     fn change_workspace(
///         &self,
///         _: HyprlandWorkspaceSelector,
///     ) -> Result<(), HyprlandError> {
///         Err(HyprlandError::unsupported("change_workspace"))
///     }
///
///     fn focus_and_toggle_special_workspace(
///         &self,
///         _: HyprlandMonitorSelector,
///         _: &str,
///     ) -> Result<(), HyprlandError> {
///         Err(HyprlandError::unsupported("focus_and_toggle_special_workspace"))
///     }
///
///     fn keyboard_state(&self) -> Result<HyprlandKeyboardState, HyprlandError> {
///         Err(HyprlandError::unsupported("keyboard_state"))
///     }
///
///     fn switch_keyboard_layout(&self) -> Result<(), HyprlandError> {
///         Err(HyprlandError::unsupported("switch_keyboard_layout"))
///     }
///
///     fn clients_snapshot(
///         &self,
///     ) -> Result<Vec<hydebar_proto::ports::hyprland::HyprlandClientInfo>, HyprlandError> {
///         Err(HyprlandError::unsupported("clients_snapshot"))
///     }
///
///     fn focus_window(&self, _: &str) -> Result<(), HyprlandError> {
///         Err(HyprlandError::unsupported("focus_window"))
///     }
/// }
///
/// let port: Arc<dyn HyprlandPort> = Arc::new(DummyPort);
/// assert!(port.active_window().is_err());
/// ```
pub trait HyprlandPort: Send + Sync {
    /// Subscribe to window related events.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the backend cannot open the event
    /// stream or does not support window events.
    fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError>;

    /// Subscribe to workspace related events.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the backend cannot open the event
    /// stream or does not support workspace events.
    fn workspace_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError>;

    /// Subscribe to keyboard related events.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the backend cannot open the event
    /// stream or does not support keyboard events.
    fn keyboard_events(&self)
    -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError>;

    /// Retrieve the currently active window, if any.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the compositor cannot be queried or
    /// the backend does not support the operation.
    fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError>;

    /// Obtain the latest snapshot of monitors and workspaces.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the compositor cannot be queried or
    /// the backend does not support the operation.
    fn workspace_snapshot(&self) -> Result<HyprlandWorkspaceSnapshot, HyprlandError>;

    /// Request Hyprland to focus the provided workspace.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the compositor rejects the request,
    /// the dispatch times out, or the backend does not support it.
    fn change_workspace(&self, workspace: HyprlandWorkspaceSelector) -> Result<(), HyprlandError>;

    /// Focus the provided monitor and toggle a special workspace by name.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the compositor rejects the request,
    /// the dispatch times out, or the backend does not support it.
    fn focus_and_toggle_special_workspace(
        &self,
        monitor: HyprlandMonitorSelector,
        workspace_name: &str
    ) -> Result<(), HyprlandError>;

    /// Retrieve the current keyboard state, including layout metadata.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the compositor cannot be queried or
    /// the backend does not support the operation.
    fn keyboard_state(&self) -> Result<HyprlandKeyboardState, HyprlandError>;

    /// Request Hyprland to switch to the next keyboard layout.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the compositor rejects the request or
    /// the backend does not support it.
    fn switch_keyboard_layout(&self) -> Result<(), HyprlandError>;

    /// Obtain the latest snapshot of the compositor's mapped clients.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the compositor cannot be queried or
    /// the backend does not support the operation.
    fn clients_snapshot(&self) -> Result<Vec<HyprlandClientInfo>, HyprlandError>;

    /// Request Hyprland to focus the window at `address`.
    ///
    /// # Errors
    ///
    /// Returns a [`HyprlandError`] when the compositor rejects the request,
    /// the dispatch times out, or the backend does not support it.
    fn focus_window(&self, address: &str) -> Result<(), HyprlandError>;
}
