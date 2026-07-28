//! Scheduled and signal driven execution of a custom module command.
//!
//! Where the listener task keeps a single process alive and reads its output,
//! the poller re-runs a short lived command: once at startup, then on every
//! interval tick and whenever the configured real time signal arrives. This is
//! the Waybar `interval` plus `signal` contract, so scripts written for
//! `pkill -RTMIN+N waybar` work unchanged.

use std::{future::pending, sync::Arc, time::Duration};

use log::error;
use tokio::{
    process::Command,
    signal::unix::{Signal, SignalKind, signal},
    time::{Instant, Interval, MissedTickBehavior, interval_at}
};

use super::{
    Message,
    data::CustomListenData,
    error::{CustomCommandError, CustomListenerError, truncate_snippet},
    listener::send_event
};
use crate::{ModuleEventSender, services::ServiceEvent};

/// Resolves the real time signal number an offset refers to.
///
/// The offset is relative to `SIGRTMIN`, the same base `pkill -RTMIN+N` uses,
/// and offsets past `SIGRTMAX` are rejected instead of aliasing another signal.
pub(super) fn real_time_signal(offset: u8) -> Option<i32> {
    let raw = libc::SIGRTMIN().checked_add(i32::from(offset))?;

    (raw <= libc::SIGRTMAX()).then_some(raw)
}

/// Registers the refresh signal, if the module asked for one.
fn open_refresh_signal(offset: Option<u8>) -> Result<Option<Signal>, CustomListenerError> {
    let Some(offset) = offset else {
        return Ok(None);
    };

    let raw = real_time_signal(offset).ok_or(CustomListenerError::Command(
        CustomCommandError::UnsupportedSignal(offset)
    ))?;

    signal(SignalKind::from_raw(raw)).map(Some).map_err(|err| {
        CustomListenerError::Command(CustomCommandError::Signal(offset, Arc::new(err)))
    })
}

/// Builds the ticker firing after the first period, the initial run happening
/// before the loop is entered.
fn open_ticker(period: Option<Duration>) -> Option<Interval> {
    let period = period?;
    let mut ticker = interval_at(Instant::now() + period, period);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    Some(ticker)
}

/// Waits for the next scheduled tick, or forever when no interval is set.
async fn next_tick(ticker: Option<&mut Interval>) {
    match ticker {
        Some(ticker) => {
            ticker.tick().await;
        }
        None => pending::<()>().await
    }
}

/// Waits for the next refresh signal, or forever when none is registered.
async fn next_refresh(refresh: Option<&mut Signal>) {
    match refresh {
        Some(refresh) => {
            refresh.recv().await;
        }
        None => pending::<()>().await
    }
}

/// Runs the command once and publishes whatever it printed.
///
/// The whole standard output is parsed as a single JSON object, matching the
/// non-continuous Waybar `exec` contract. A failing run reports an error event
/// so the module can render an alert without tearing the poller down.
async fn run_once(
    module_name: &str,
    command: &str,
    sender: &ModuleEventSender<Message>
) -> Result<(), CustomListenerError> {
    let output = Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()
        .await
        .map_err(|err| CustomListenerError::Command(CustomCommandError::Spawn(Arc::new(err))))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload = stdout.trim();

    if payload.is_empty() {
        if !output.status.success() {
            let failure = CustomCommandError::NonZeroExit {
                status: output.status.code()
            };
            error!("Custom module '{module_name}' command failed: {failure:?}");

            return send_event(sender, ServiceEvent::Error(failure))
                .map_err(CustomListenerError::Module);
        }

        return Ok(());
    }

    match serde_json::from_str::<CustomListenData>(payload) {
        Ok(data) => {
            send_event(sender, ServiceEvent::Update(data)).map_err(CustomListenerError::Module)
        }
        Err(err) => {
            let parse_error = CustomCommandError::Parse(truncate_snippet(payload), Arc::new(err));
            error!("Custom module '{module_name}' failed to parse JSON output: {parse_error:?}");

            send_event(sender, ServiceEvent::Error(parse_error))
                .map_err(CustomListenerError::Module)
        }
    }
}

/// Drives a custom module by re-running its command.
///
/// The command runs immediately, then on every `period` tick and on every
/// delivery of the real time signal `signal_offset` refers to. Without either
/// trigger the command runs exactly once and the task completes.
pub(super) async fn run_custom_poller(
    module_name: Arc<str>,
    command: Arc<str>,
    period: Option<Duration>,
    signal_offset: Option<u8>,
    sender: ModuleEventSender<Message>
) -> Result<(), CustomListenerError> {
    let mut refresh = open_refresh_signal(signal_offset)?;
    let mut ticker = open_ticker(period);

    run_once(module_name.as_ref(), command.as_ref(), &sender).await?;

    if ticker.is_none() && refresh.is_none() {
        return Ok(());
    }

    loop {
        tokio::select! {
            () = next_tick(ticker.as_mut()) => {}
            () = next_refresh(refresh.as_mut()) => {}
        }

        run_once(module_name.as_ref(), command.as_ref(), &sender).await?;
    }
}
