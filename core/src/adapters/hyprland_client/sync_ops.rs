//! Blocking Hyprland requests: the retry policy every synchronous reader
//! shares, and the state queries built on it.

use std::{
    sync::{Arc, mpsc::channel},
    thread,
    time::Duration
};

use hydebar_proto::ports::hyprland::{HyprlandError, HyprlandKeyboardState};
use log::warn;

use super::{config::HyprlandClientConfig, util::calculate_retry_delay};
use crate::adapters::compositor::query;

/// The name a keyboard state query reports under.
const KEYBOARD_STATE_OP: &str = "keyboard_state";

/// Queries which keyboard layout is active and how many layouts are configured.
///
/// Goes through the retry policy of `config` like every other blocking request,
/// so any reader — the event wiring included — asks the same question the same
/// careful way.
pub fn keyboard_state(
    config: &HyprlandClientConfig
) -> Result<HyprlandKeyboardState, HyprlandError> {
    execute_with_retry(config, KEYBOARD_STATE_OP, || {
        let layouts = query::option_text(KEYBOARD_STATE_OP, "input:kb_layout")?;
        let has_multiple_layouts = layouts
            .split(',')
            .filter(|value| !value.trim().is_empty())
            .count()
            > 1;

        let devices = query::devices(KEYBOARD_STATE_OP)?;
        let active_layout = devices
            .keyboards
            .iter()
            .find(|keyboard| keyboard.main)
            .map_or_else(
                || "unknown".to_string(),
                |keyboard| keyboard.active_keymap.clone()
            );

        Ok(HyprlandKeyboardState {
            active_layout,
            has_multiple_layouts,
            active_submap: None
        })
    })
}

/// Execute a blocking Hyprland request in a worker thread and wait for it to
/// complete within the provided timeout.
pub fn execute_once<R, F>(
    operation: &'static str,
    timeout_dur: Duration,
    func: Arc<F>
) -> Result<R, HyprlandError>
where
    R: Send + 'static,
    F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static
{
    let (tx, rx) = channel();
    thread::spawn(move || {
        let result = func();
        if tx.send(result).is_err() {
            warn!(
                target: "hydebar::hyprland",
                "result receiver dropped before completion (operation={operation})"
            );
        }
    });

    match rx.recv_timeout(timeout_dur) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(HyprlandError::Timeout {
            operation,
            timeout: timeout_dur
        }),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(HyprlandError::message(
            operation,
            "worker thread terminated before sending result"
        ))
    }
}

/// Execute a blocking Hyprland request with retry and backoff semantics derived
/// from [`HyprlandClientConfig`].
pub fn execute_with_retry<R, F>(
    config: &HyprlandClientConfig,
    operation: &'static str,
    func: F
) -> Result<R, HyprlandError>
where
    R: Send + 'static,
    F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static
{
    let func = Arc::new(func);
    let mut last_error = None;

    for attempt in 1..=config.retry_attempts {
        let func_clone = Arc::clone(&func);
        match execute_once(operation, config.request_timeout, func_clone) {
            Ok(result) => return Ok(result),
            Err(err) => {
                warn!(
                    target: "hydebar::hyprland",
                    "Hyprland operation failed (operation={operation}, attempt={attempt}, error={err})"
                );
                last_error = Some(err);
                if attempt < config.retry_attempts {
                    let delay = calculate_retry_delay(config.retry_backoff, attempt);
                    if !delay.is_zero() {
                        thread::sleep(delay);
                    }
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        HyprlandError::message(operation, "Hyprland operation failed without error detail")
    }))
}
