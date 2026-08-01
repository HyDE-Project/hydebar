//! Listener and commander threads driving the `PulseAudio` mainloop.

use std::{
    cell::RefCell,
    rc::Rc,
    thread::{self, JoinHandle}
};

use iced::futures::executor::block_on;
use libpulse_binding::{context::subscribe::InterestMaskSet, mainloop::standard::IterateResult};
use log::error;
use masterror::{AppError, AppResult};
use tokio::sync::mpsc::{Receiver, Sender};

use super::{BackendCommand, BackendEvent, PulseAudioServer};

impl PulseAudioServer {
    pub(super) async fn start_listener(
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
                        server.wait_for_response(server.introspector.get_server_info({
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
                        server.wait_for_response(server.introspector.get_sink_info_list({
                            let tx = from_server_tx.clone();
                            let sinks = sinks.clone();
                            move |info| {
                                Self::populate_and_send_sinks(info, &tx, &mut sinks.borrow_mut());
                            }
                        }))
                    {
                        error!("Failed to get sink info: {err}");
                        let _ = from_server_tx.try_send(BackendEvent::Error(err.to_string()));
                    }

                    let sources = Rc::new(RefCell::new(Vec::new()));
                    if let Err(err) =
                        server.wait_for_response(server.introspector.get_source_info_list({
                            let tx = from_server_tx.clone();
                            let sources = sources.clone();
                            move |info| {
                                Self::populate_and_send_sources(
                                    info,
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
                                        info,
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
                                        info,
                                        &tx,
                                        &mut sources.borrow_mut()
                                    );
                                }
                            });
                        }
                    )));

                    loop {
                        let data = server.mainloop.iterate(true);
                        if let IterateResult::Quit(_) | IterateResult::Err(_) = data {
                            error!("PulseAudio mainloop error");
                            let _ = from_server_tx.try_send(BackendEvent::Error("PulseAudio mainloop error".into()));
                            break;
                        }
                    }
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

    pub(super) async fn start_commander(
        from_server_tx: Sender<BackendEvent>,
        mut to_server_rx: Receiver<BackendCommand>
    ) -> AppResult<JoinHandle<()>> {
        let (ready_tx, mut ready_rx) = tokio::sync::mpsc::channel(4);

        let handle = thread::spawn(move || {
            block_on(async move {
                match Self::new() {
                    Ok(mut server) => {
                        let _ = ready_tx.try_send(true);
                        while let Some(command) = to_server_rx.recv().await {
                            if let Err(err) = match command {
                                BackendCommand::SinkMute(name, mute) => {
                                    server.set_sink_mute(&name, mute)
                                }
                                BackendCommand::SourceMute(name, mute) => {
                                    server.set_source_mute(&name, mute)
                                }
                                BackendCommand::SinkVolume(name, volume) => {
                                    server.set_sink_volume(&name, &volume)
                                }
                                BackendCommand::SourceVolume(name, volume) => {
                                    server.set_source_volume(&name, &volume)
                                }
                                BackendCommand::DefaultSink(name, port) => {
                                    server.set_default_sink(&name, &port)
                                }
                                BackendCommand::DefaultSource(name, port) => {
                                    server.set_default_source(&name, &port)
                                }
                            } {
                                error!("PulseAudio command failed: {err}");
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed to start PulseAudio commander: {err}");
                        let _ = from_server_tx.try_send(BackendEvent::Error(err.to_string()));
                    }
                }
            });
        });

        match ready_rx.recv().await {
            Some(true) => Ok(handle),
            _ => Err(AppError::internal(
                "Failed to start PulseAudio commander thread"
            ))
        }
    }
}
