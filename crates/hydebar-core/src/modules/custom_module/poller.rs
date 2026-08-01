//! Scheduled and signal driven execution of a custom module command.
//!
//! Where the listener task keeps a single process alive and reads its
//! output, the poller re-runs a short lived command: once at startup,
//! then on every interval tick and whenever the configured real time
//! signal arrives. This is the Waybar `interval` plus `signal`
//! contract, so scripts written for `pkill -RTMIN+N waybar` work
//! unchanged.

use std::{future::pending, process::Stdio, sync::Arc, time::Duration};

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
/// The offset is relative to `SIGRTMIN`, the same base `pkill -RTMIN+N`
/// uses, and offsets past `SIGRTMAX` are rejected instead of aliasing
/// another signal.
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

/// Builds the ticker firing after the first period, the initial run
/// happening before the loop is entered.
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
/// The whole standard output is parsed as a single JSON object, matching
/// the non-continuous Waybar `exec` contract. A failing run reports an
/// error event so the module can render an alert without tearing the
/// poller down.
///
/// `published` carries the payload the bar is already showing. A run that
/// reprints it publishes nothing, since the repaint every event triggers
/// would produce an identical frame.
///
/// The run happens in a process group of its own so that a reload landing
/// while the command is still working ends it instead of orphaning it:
/// a script that blocks on the network, run every few seconds, would
/// otherwise pile up one stranded copy per reload.
async fn run_once(
    module_name: &str,
    command: &str,
    sender: &ModuleEventSender<Message>,
    published: &mut Option<CustomListenData>
) -> Result<(), CustomListenerError> {
    let mut spawner = Command::new("bash");
    spawner
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = crate::utils::process_group::guarded_output(&mut spawner)
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
            *published = None;

            send_event(sender, ServiceEvent::Error(failure));

            return Ok(());
        }

        return Ok(());
    }

    match serde_json::from_str::<CustomListenData>(payload) {
        Ok(data) => {
            if published.as_ref() == Some(&data) {
                return Ok(());
            }

            *published = Some(data.clone());

            send_event(sender, ServiceEvent::Update(data));

            Ok(())
        }
        Err(err) => {
            let parse_error = CustomCommandError::Parse(truncate_snippet(payload), Arc::new(err));
            error!("Custom module '{module_name}' failed to parse JSON output: {parse_error:?}");
            *published = None;

            send_event(sender, ServiceEvent::Error(parse_error));

            Ok(())
        }
    }
}

/// Drives a custom module by re-running its command.
///
/// The command runs immediately, then on every `period` tick and on every
/// delivery of the real time signal `signal_offset` refers to. Without
/// either trigger the command runs exactly once and the task completes.
pub(super) async fn run_custom_poller(
    module_name: Arc<str>,
    command: Arc<str>,
    period: Option<Duration>,
    signal_offset: Option<u8>,
    sender: ModuleEventSender<Message>
) -> Result<(), CustomListenerError> {
    let mut refresh = open_refresh_signal(signal_offset)?;
    let mut ticker = open_ticker(period);
    let mut published = None;

    run_once(
        module_name.as_ref(),
        command.as_ref(),
        &sender,
        &mut published
    )
    .await?;

    if ticker.is_none() && refresh.is_none() {
        return Ok(());
    }

    loop {
        tokio::select! {
            () = next_tick(ticker.as_mut()) => {}
            () = next_refresh(refresh.as_mut()) => {}
        }

        run_once(
            module_name.as_ref(),
            command.as_ref(),
            &sender,
            &mut published
        )
        .await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Distance from `SIGRTMIN` to `SIGRTMAX` on the running kernel.
    fn real_time_span() -> u8 {
        u8::try_from(libc::SIGRTMAX() - libc::SIGRTMIN()).expect("span fits a byte")
    }

    #[test]
    fn a_zero_offset_names_sigrtmin_itself() {
        assert_eq!(real_time_signal(0), Some(libc::SIGRTMIN()));
    }

    #[test]
    fn an_offset_counts_up_from_sigrtmin() {
        assert_eq!(real_time_signal(2), Some(libc::SIGRTMIN() + 2));
    }

    #[test]
    fn the_last_real_time_signal_is_still_accepted() {
        assert_eq!(real_time_signal(real_time_span()), Some(libc::SIGRTMAX()));
    }

    #[test]
    fn an_offset_past_sigrtmax_is_rejected() {
        assert_eq!(real_time_signal(real_time_span() + 1), None);
    }

    #[test]
    fn the_largest_offset_is_rejected() {
        assert_eq!(real_time_signal(u8::MAX), None);
    }

    #[test]
    fn no_interval_means_no_ticker() {
        assert!(open_ticker(None).is_none());
    }

    #[tokio::test]
    async fn a_ticker_keeps_the_configured_period_and_delays_missed_ticks() {
        let ticker = open_ticker(Some(Duration::from_secs(5))).expect("ticker");

        assert_eq!(ticker.period(), Duration::from_secs(5));
        assert_eq!(ticker.missed_tick_behavior(), MissedTickBehavior::Delay);
    }

    #[tokio::test(start_paused = true)]
    async fn the_first_tick_waits_a_full_period() {
        let period = Duration::from_secs(5);
        let start = Instant::now();
        let mut ticker = open_ticker(Some(period)).expect("ticker");

        ticker.tick().await;

        assert_eq!(Instant::now() - start, period);
    }
}
