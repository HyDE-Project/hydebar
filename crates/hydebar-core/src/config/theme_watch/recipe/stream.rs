//! The watch loop: batches from the kernel, settled, into reloads.

use std::{any::TypeId, hash::Hash, sync::Arc};

use iced::futures::{StreamExt, channel::mpsc::Sender, pin_mut};
use iced_futures::subscription::{self, Recipe};
use inotify::Inotify;
use log::{error, info};

use super::{
    BATCH_SIZE, SETTLE, ThemeWatcher,
    watches::add_watches
};
use crate::config::{
    ConfigEvent,
    theme_watch::interpret::{handle_theme_event, interpret_theme_event},
    theme_watch::sources::watched_names,
    watch::{WatchLoopOutcome, interpret::process_event_batches}
};

impl Recipe for ThemeWatcher {
    type Output = ConfigEvent;

    fn hash(&self, state: &mut subscription::Hasher) {
        TypeId::of::<Self>().hash(state);
        self.config_path.hash(state);
        self.roots.hash(state);
        Arc::as_ptr(&self.manager).hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream
    ) -> iced::futures::stream::BoxStream<'static, Self::Output> {
        let Self {
            config_path,
            roots,
            targets,
            manager
        } = *self;

        Box::pin(iced::stream::channel(100, move |output: Sender<ConfigEvent>| {
            let manager = Arc::clone(&manager);
            let names = watched_names();

            async move {
                let mut restarts: u32 = 0;
                loop {
                    let inotify = match Inotify::init() {
                        Ok(inotify) => inotify,
                        Err(e) => {
                            error!("Failed to initialize inotify for the HyDE theme: {e}");
                            break;
                        }
                    };

                    if !add_watches(&inotify, &targets) {
                        error!("No HyDE theme directory could be watched; giving up");
                        break;
                    }

                    let buffer = [0; 1024];
                    let stream = match inotify.into_event_stream(buffer) {
                        Ok(stream) => stream,
                        Err(e) => {
                            error!("Failed to create the HyDE theme event stream: {e}");
                            break;
                        }
                    };

                    let event_stream = stream.ready_chunks(BATCH_SIZE);
                    pin_mut!(event_stream);

                    let sender_template = output.clone();
                    let config_path_clone = config_path.clone();
                    let roots_clone = roots.clone();
                    let manager_clone = Arc::clone(&manager);
                    let names_ref = &names;

                    let outcome = process_event_batches(
                        event_stream.as_mut(),
                        |event| interpret_theme_event(event, names_ref),
                        move |event| {
                            let mut sender = sender_template.clone();
                            let config_path = config_path_clone.clone();
                            let roots = roots_clone.clone();
                            let manager = Arc::clone(&manager_clone);

                            async move {
                                tokio::time::sleep(SETTLE).await;

                                handle_theme_event(
                                    &mut sender,
                                    &config_path,
                                    &roots,
                                    event,
                                    manager
                                )
                                .await
                            }
                        }
                    )
                    .await;

                    match outcome {
                        WatchLoopOutcome::StreamEnded => {
                            info!(
                                "HyDE theme watch stream closed; attempting to restart the inotify watcher"
                            );

                            restarts = restarts.saturating_add(1);
                            tokio::time::sleep(crate::services::reconnect_delay(restarts)).await;
                        }
                        WatchLoopOutcome::HandlerClosed => {
                            info!("HyDE theme watch handler closed; stopping watcher loop");
                            break;
                        }
                    }
                }

                info!("HyDE theme watcher terminated");
            }
        }))
    }
}
