//! Subscription recipe driving the configuration watcher.

use std::{
    any::TypeId,
    ffi::OsStr,
    hash::Hash,
    path::{Path, PathBuf},
    sync::Arc
};

use iced::{
    Subscription,
    advanced::subscription::{self, Recipe, from_recipe},
    futures::{StreamExt, channel::mpsc::Sender, pin_mut},
    stream::channel
};
use inotify::{Inotify, WatchMask};
use log::{debug, error, info};

use super::{
    ConfigEvent, WatchLoopOutcome,
    interpret::{handle_watch_event, interpret_event, process_event_batches}
};
use crate::config::ConfigManager;

struct ConfigWatcher {
    path:    PathBuf,
    manager: Arc<ConfigManager>
}

impl Recipe for ConfigWatcher {
    type Output = ConfigEvent;

    fn hash(&self, state: &mut subscription::Hasher) {
        TypeId::of::<Self>().hash(state);
        self.path.hash(state);
        Arc::as_ptr(&self.manager).hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream
    ) -> iced::futures::stream::BoxStream<'static, Self::Output> {
        let path = self.path;
        let manager = self.manager;

        Box::pin(channel(100, move |output: Sender<ConfigEvent>| {
            let manager = Arc::clone(&manager);

            async move {
                let Some(folder) = path.parent().map(Path::to_path_buf) else {
                    error!(
                        "Config file path does not have a parent directory, cannot watch for changes"
                    );
                    return;
                };

                let Some(file_name) = path.file_name().map(OsStr::to_os_string) else {
                    error!("Config file path does not have a file name, cannot watch for changes");
                    return;
                };

                loop {
                    let inotify = match Inotify::init() {
                        Ok(inotify) => inotify,
                        Err(e) => {
                            error!("Failed to initialize inotify: {e}");
                            break;
                        }
                    };

                    debug!("Watching config file at {path:?}");

                    let watch_result = inotify.watches().add(
                        &folder,
                        WatchMask::CREATE
                            | WatchMask::DELETE
                            | WatchMask::MOVE
                            | WatchMask::MODIFY
                    );

                    if let Err(e) = watch_result {
                        error!("Failed to add watch for {folder:?}: {e}");
                        break;
                    }

                    let buffer = [0; 1024];
                    let stream = match inotify.into_event_stream(buffer) {
                        Ok(stream) => stream,
                        Err(e) => {
                            error!("Failed to create inotify event stream: {e}");
                            break;
                        }
                    };

                    let event_stream = stream.ready_chunks(10);
                    pin_mut!(event_stream);

                    let sender_template = output.clone();
                    let path_clone = path.clone();
                    let manager_clone = Arc::clone(&manager);

                    match process_event_batches(
                        event_stream.as_mut(),
                        |event| interpret_event(event, file_name.as_os_str()),
                        move |event| {
                            let mut sender = sender_template.clone();
                            let path = path_clone.clone();
                            let manager = Arc::clone(&manager_clone);

                            async move { handle_watch_event(&mut sender, &path, event, manager).await }
                        },
                    )
                    .await
                    {
                        WatchLoopOutcome::StreamEnded => {
                            info!(
                                "Config watch stream closed; attempting to restart the inotify watcher"
                            );
                            continue;
                        }
                        WatchLoopOutcome::HandlerClosed => {
                            info!("Config watch handler closed; stopping watcher loop");
                            break;
                        }
                    }
                }

                info!("Config watcher terminated");
            }
        }))
    }
}

/// Watches the bar configuration file and republishes it on every change.
///
/// The watch sits on the parent directory rather than on the file itself
/// because editors and generators replace a file instead of writing into it,
/// which would silently detach an inode level watch.
pub fn subscription(path: &Path, manager: Arc<ConfigManager>) -> Subscription<ConfigEvent> {
    from_recipe(ConfigWatcher {
        path: path.to_path_buf(),
        manager
    })
}
