//! Subscription publishing privacy state changes.

use std::{any::TypeId, fs, path::Path};

use iced::{Subscription, futures::StreamExt, stream::channel};
use log::{debug, warn};

use super::{PrivacyEvent, PrivacyService, State, error::PrivacyError};
use crate::services::{ReadOnlyService, ServiceEvent};

impl ReadOnlyService for PrivacyService {
    type UpdateEvent = PrivacyEvent;
    type Error = PrivacyError;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            PrivacyEvent::AddNode(node) => {
                self.data.nodes.push(node);
            }
            PrivacyEvent::RemoveNode(id) => {
                self.data.nodes.retain(|node| node.id != id);
            }
            PrivacyEvent::WebcamOpen => {
                self.data.webcam_access += 1;
                debug!("Webcam opened {}", self.data.webcam_access);
            }
            PrivacyEvent::WebcamClose => {
                self.data.webcam_access = i32::max(self.data.webcam_access - 1, 0);
                debug!("Webcam closed {}", self.data.webcam_access);
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with(id, |&_id| {
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    match PrivacyService::start_listening(state, &mut output).await {
                        Ok(next_state) => {
                            state = next_state;
                        }
                        Err(error) => {
                            if let Err(send_error) = PrivacyService::emit_event(
                                &mut output,
                                ServiceEvent::Error(error.clone())
                            )
                            .await
                            {
                                warn!("Failed to emit privacy service error event: {send_error}");
                                break;
                            }

                            state = State::Init;
                        }
                    }
                }
            })
        })
    }
}

pub(super) fn is_device_in_use(target: &str) -> i32 {
    let mut used_by = 0;
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let pid_path = entry.path();

            if !pid_path.join("fd").exists() {
                continue;
            }

            if let Ok(fd_entries) = fs::read_dir(pid_path.join("fd")) {
                for fd_entry in fd_entries.flatten() {
                    if let Ok(link_path) = fs::read_link(fd_entry.path())
                        && link_path == Path::new(target)
                    {
                        used_by += 1;
                    }
                }
            }
        }
    }

    used_by
}
