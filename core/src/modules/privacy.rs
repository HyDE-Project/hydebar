//! The privacy indicator: small icons for whoever is watching or listening.
//!
//! Three rooms beside the shared state: [`publisher`] carries service events
//! onto the bus and keeps the listener loop alive, [`module`] starts and
//! stops that loop with the layout, and [`view`] draws the icons. The root
//! folds events into the service snapshot the view reads.

use log::{error, warn};
use tokio::task::JoinHandle;

use crate::{
    ModuleEventSender,
    services::{
        ReadOnlyService, ServiceEvent,
        privacy::{PrivacyService, error::PrivacyError}
    }
};

mod module;
mod publisher;
mod view;

/// Message emitted by the privacy module subscription.
#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    /// The privacy watcher said something.
    Event(ServiceEvent<PrivacyService>)
}

/// UI module exposing privacy information icons.
#[derive(Debug, Default)]
pub struct Privacy {
    /// The watcher, once it is up.
    pub service: Option<PrivacyService>,
    sender:      Option<ModuleEventSender<PrivacyMessage>>,
    tasks:       Vec<JoinHandle<()>>
}

impl Privacy {
    /// Update the module state based on new privacy events.
    pub fn update(&mut self, message: PrivacyMessage) {
        let PrivacyMessage::Event(event) = message;
        match event {
            ServiceEvent::Init(service) => {
                self.service = Some(service);
            }
            ServiceEvent::Update(data) => {
                if let Some(privacy) = self.service.as_mut() {
                    privacy.update(data);
                }
            }
            ServiceEvent::Error(error) => match error {
                PrivacyError::WebcamUnavailable => {
                    warn!("Webcam device unavailable; continuing with PipeWire-only privacy data");
                }
                _ => error!("Privacy service error: {error}")
            }
        }
    }
}
