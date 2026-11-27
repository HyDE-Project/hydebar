//! Reactive service exposing notifications to the bar: the state handle the
//! bar holds, wired to the session that serves the bus.

use iced::{Subscription, futures::SinkExt, stream};
use log::error;

use self::{end::SessionEnd, session::serve};
use super::{NotificationEvent, NotificationsError};
use crate::services::{ReadOnlyService, ServiceEvent, reconnect_delay};

mod end;
mod handle;
mod session;

pub use handle::NotificationsService;

impl ReadOnlyService for NotificationsService {
    type UpdateEvent = NotificationEvent;
    type Error = NotificationsError;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            NotificationEvent::Received(_) | NotificationEvent::ActionInvoked(_, _) => {}
            NotificationEvent::Closed(id) => {
                self.storage().remove(id);
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = std::any::TypeId::of::<Self>();
        Subscription::run_with(id, |&_id| {
            stream::channel(
                100,
                |mut output: iced::futures::channel::mpsc::Sender<ServiceEvent<Self>>| async move {
                    let service = Self::new();
                    let storage = service.shared_storage();

                    if output
                        .send(ServiceEvent::Init(service.clone()))
                        .await
                        .is_err()
                    {
                        error!("Failed to send notifications service init event");
                        return;
                    }

                    let mut failures: u32 = 0;
                    loop {
                        match serve(&storage, &mut output).await {
                            SessionEnd::UiClosed => return,
                            SessionEnd::Failed => {
                                failures = failures.saturating_add(1);
                                tokio::time::sleep(reconnect_delay(failures)).await;
                            }
                        }
                    }
                }
            )
        })
    }
}
