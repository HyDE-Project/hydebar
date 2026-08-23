//! Asking the compositor a question and reading its answer back.

use hydebar_proto::{compositor_ipc, ports::hyprland::HyprlandError};
use serde::de::DeserializeOwned;

use super::records::{Client, Devices, Monitor, Option_, Workspace};

/// Asks `command` and reads the answer as `T`.
///
/// # Errors
///
/// Returns [`HyprlandError::Message`] when there is no compositor to ask, or
/// when the answer does not read as `T`. The two are told apart by the text,
/// because a caller can do nothing about either but say so.
fn ask<T>(operation: &'static str, command: &str) -> Result<T, HyprlandError>
where
    T: DeserializeOwned
{
    let answer = compositor_ipc::request(command).ok_or(HyprlandError::Message {
        operation,
        message: format!("the compositor did not answer `{command}`")
    })?;

    serde_json::from_str(&answer).map_err(|err| HyprlandError::Message {
        operation,
        message: format!("the answer to `{command}` does not read: {err}")
    })
}

/// Every screen the compositor is driving.
///
/// # Errors
///
/// Returns an error when the compositor cannot be reached or its answer does
/// not read.
pub fn monitors(operation: &'static str) -> Result<Vec<Monitor>, HyprlandError> {
    ask(operation, "j/monitors")
}

/// Every workspace the compositor holds.
///
/// # Errors
///
/// Returns an error when the compositor cannot be reached or its answer does
/// not read.
pub fn workspaces(operation: &'static str) -> Result<Vec<Workspace>, HyprlandError> {
    ask(operation, "j/workspaces")
}

/// The workspace holding focus.
///
/// # Errors
///
/// Returns an error when the compositor cannot be reached or its answer does
/// not read.
pub fn active_workspace(operation: &'static str) -> Result<Workspace, HyprlandError> {
    ask(operation, "j/activeworkspace")
}

/// Every window the compositor is drawing.
///
/// # Errors
///
/// Returns an error when the compositor cannot be reached or its answer does
/// not read.
pub fn clients(operation: &'static str) -> Result<Vec<Client>, HyprlandError> {
    ask(operation, "j/clients")
}

/// The window holding focus, if any does.
///
/// The compositor answers an empty object when nothing is focused, which is
/// what the absent case reads as.
///
/// # Errors
///
/// Returns an error when the compositor cannot be reached or its answer does
/// not read.
pub fn active_window(operation: &'static str) -> Result<Option<Client>, HyprlandError> {
    ask::<serde_json::Value>(operation, "j/activewindow")
        .map(|value| serde_json::from_value(value).ok())
}

/// Every input device the compositor has.
///
/// # Errors
///
/// Returns an error when the compositor cannot be reached or its answer does
/// not read.
pub fn devices(operation: &'static str) -> Result<Devices, HyprlandError> {
    ask(operation, "j/devices")
}

/// The value of one configuration option, as text.
///
/// # Errors
///
/// Returns an error when the compositor cannot be reached or its answer does
/// not read.
pub fn option_text(operation: &'static str, name: &str) -> Result<String, HyprlandError> {
    ask::<Option_>(operation, &format!("j/getoption {name}")).map(|option| option.text)
}
