//! Listener thread bootstrapping the `PulseAudio` subscription mainloop.

use std::{
    cell::RefCell,
    rc::Rc,
    thread::{self, JoinHandle}
};

use libpulse_binding::{
    context::subscribe::InterestMaskSet,
    mainloop::standard::{IterateResult, Mainloop},
    time::{MicroSeconds, MonotonicTs}
};
use log::error;
use masterror::{AppError, AppResult};
use tokio::sync::mpsc::Sender;

use super::super::{BackendEvent, PulseAudioServer};

/// How often the parked mainloop wakes to notice an abandoned bridge.
///
/// The listener blocks inside the `PulseAudio` poll with nothing to say on a
/// quiet system; this recurring timer is its only guaranteed wakeup, letting
/// the thread see a closed event channel and leave instead of outliving its
/// dropped handle.
const SHUTDOWN_POLL: MicroSeconds = MicroSeconds(300_000);

impl PulseAudioServer {
    #[expect(
        clippy::too_many_lines,
        reason = "the listener thread body is one sequential PulseAudio bootstrap; splitting it would detach each step from its error reporting"
    )]
    pub(in super::super) async fn start_listener(
        from_server_tx: Sender<BackendEvent>
    ) -> AppResult<JoinHandle<()>> {
        let (ready_tx, mut ready_rx) = tokio::sync::mpsc::channel(4);

        let handle = thread::spawn({
            let from_server_tx = from_server_tx.clone();
            move || match Self::new() {
                Ok(mut server) => {
                    let _ = ready_tx.try_send(true);

                    server.context.subscribe(
                        InterestMaskSet::SERVER
                            .union(InterestMaskSet::SINK)
                            .union(InterestMaskSet::SOURCE),
                        |result| {
                            if !result {
                                error!("Audio subscription failed");
                            }
                        }
                    );

                    if let Err(err) =
                        server.wait_for_response(&server.introspector.get_server_info({
                            let tx = from_server_tx.clone();
                            move |info| {
                                Self::send_server_info(info, &tx);
                            }
                        }))
                    {
                        error!("Failed to get server info: {err}");
                        let _ = from_server_tx.try_send(BackendEvent::Error(err.to_string()));
                    }

                    let sinks = Rc::new(RefCell::new(Vec::new()));
                    if let Err(err) =
                        server.wait_for_response(&server.introspector.get_sink_info_list({
                            let tx = from_server_tx.clone();
                            let sinks = sinks.clone();
                            move |info| {
                                Self::populate_and_send_sinks(&info, &tx, &mut sinks.borrow_mut());
                            }
                        }))
                    {
                        error!("Failed to get sink info: {err}");
                        let _ = from_server_tx.try_send(BackendEvent::Error(err.to_string()));
                    }

                    let sources = Rc::new(RefCell::new(Vec::new()));
                    if let Err(err) =
                        server.wait_for_response(&server.introspector.get_source_info_list({
                            let tx = from_server_tx.clone();
                            let sources = sources.clone();
                            move |info| {
                                Self::populate_and_send_sources(
                                    &info,
                                    &tx,
                                    &mut sources.borrow_mut()
                                );
                            }
                        }))
                    {
                        error!("Failed to get source info: {err}");
                        let _ = from_server_tx.try_send(BackendEvent::Error(err.to_string()));
                    }

                    let introspector = server.context.introspect();
                    let server_introspector = server.context.introspect();
                    let from_server_tx_clone = from_server_tx.clone();
                    server.context.set_subscribe_callback(Some(Box::new(
                        move |_facility, _operation, _idx| {
                            server_introspector.get_server_info({
                                let tx = from_server_tx_clone.clone();

                                move |info| {
                                    Self::send_server_info(info, &tx);
                                }
                            });
                            introspector.get_sink_info_list({
                                let tx = from_server_tx_clone.clone();
                                let sinks = sinks.clone();

                                move |info| {
                                    Self::populate_and_send_sinks(
                                        &info,
                                        &tx,
                                        &mut sinks.borrow_mut()
                                    );
                                }
                            });
                            introspector.get_source_info_list({
                                let tx = from_server_tx_clone.clone();
                                let sources = sources.clone();

                                move |info| {
                                    Self::populate_and_send_sources(
                                        &info,
                                        &tx,
                                        &mut sources.borrow_mut()
                                    );
                                }
                            });
                        }
                    )));

                    let heartbeat = server.context.rttime_new::<Mainloop, _>(
                        &server.mainloop,
                        MonotonicTs::now() + SHUTDOWN_POLL,
                        |mut event| {
                            event.restart_rt(MonotonicTs::now() + SHUTDOWN_POLL);
                        }
                    );

                    loop {
                        let data = server.mainloop.iterate(true);
                        if let IterateResult::Quit(_) | IterateResult::Err(_) = data {
                            error!("PulseAudio mainloop error");
                            let _ = from_server_tx
                                .try_send(BackendEvent::Error("PulseAudio mainloop error".into()));
                            break;
                        }

                        if from_server_tx.is_closed() {
                            break;
                        }
                    }

                    drop(heartbeat);
                }
                Err(err) => {
                    error!("Failed to start PulseAudio listener thread: {err}");
                    let _ = ready_tx.try_send(false);
                }
            }
        });

        match ready_rx.recv().await {
            Some(true) => Ok(handle),
            _ => Err(AppError::internal(
                "Failed to start PulseAudio listener thread"
            ))
        }
    }
}
