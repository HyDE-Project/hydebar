//! Telling the compositor to do something, and checking that it did.

use hydebar_proto::{compositor_ipc, ports::hyprland::HyprlandError};

/// The only answer the compositor gives to a command it carried out.
const ACCEPTED: &str = "ok";

/// Sends `command` and reports whether the compositor accepted it.
///
/// # Errors
///
/// Returns [`HyprlandError::Message`] when there is no compositor to tell, or
/// when it answered anything other than its acceptance — which is where a
/// command written in a syntax the compositor does not speak lands.
pub fn send(operation: &'static str, command: &str) -> Result<(), HyprlandError> {
    let answer = compositor_ipc::request(command).ok_or(HyprlandError::Message {
        operation,
        message: format!("the compositor did not answer `{command}`")
    })?;

    if answer.trim() == ACCEPTED {
        return Ok(());
    }

    Err(HyprlandError::Message {
        operation,
        message: format!("the compositor refused `{command}`: {}", answer.trim())
    })
}

/// Sends a dispatcher `command`, the form the compositor takes actions in.
///
/// # Errors
///
/// Returns the refusal, as [`send`] does.
pub fn dispatch(operation: &'static str, command: &str) -> Result<(), HyprlandError> {
    send(operation, &format!("dispatch {command}"))
}
