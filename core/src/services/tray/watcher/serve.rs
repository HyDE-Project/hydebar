//! Serving tray events from one live bus connection.

use std::future::Future;

use iced::futures::{StreamExt, stream::SelectAll};
use log::{debug, info};
use masterror::AppError;

use super::{
    super::{TrayEvent, TrayService, dbus::StatusNotifierWatcherProxy},
    TrayEventStream,
    error::TrayWatcherError,
    items::{build_item, enrol, initialize_data}
};
use crate::services::ServiceEvent;

/// Serves tray events from one bus connection until the bus lets go.
///
/// The registration signals are subscribed exactly once and per-item
/// streams join the running merge as items register — nothing is torn
/// down between registrations, so an item arriving while another is being
/// built is never missed.
pub(super) async fn serve<F, Fut>(conn: &zbus::Connection, publisher: &mut F) -> TrayWatcherError
where
    F: FnMut(ServiceEvent<TrayService>) -> Fut + Send,
    Fut: Future<Output = ()> + Send
{
    let watcher = match StatusNotifierWatcherProxy::new(conn).await {
        Ok(watcher) => watcher,
        Err(err) => {
            return TrayWatcherError::EventStream(AppError::internal(format!(
                "Failed to create StatusNotifierWatcherProxy: {err}"
            )));
        }
    };

    let mut registered = match watcher.receive_status_notifier_item_registered().await {
        Ok(stream) => stream,
        Err(err) => {
            return TrayWatcherError::EventStream(AppError::internal(format!(
                "Failed to receive status notifier item registered: {err}"
            )));
        }
    };

    let mut unregistered = match watcher.receive_status_notifier_item_unregistered().await {
        Ok(stream) => stream,
        Err(err) => {
            return TrayWatcherError::EventStream(AppError::internal(format!(
                "Failed to receive status notifier item unregistered: {err}"
            )));
        }
    };

    let data = match initialize_data(conn).await {
        Ok(data) => data,
        Err(err) => return err
    };

    let mut item_streams: SelectAll<TrayEventStream> = SelectAll::new();
    let mut leases: std::collections::HashMap<
        String,
        iced::futures::channel::oneshot::Sender<()>
    > = std::collections::HashMap::new();

    for item in data.iter() {
        enrol(&mut item_streams, &mut leases, item).await;
    }

    info!("Tray service initialized");

    publisher(ServiceEvent::Init(TrayService {
        data,
        _conn: conn.clone()
    }))
    .await;

    loop {
        tokio::select! {
            signal = registered.next() => {
                let Some(signal) = signal else {
                    return TrayWatcherError::EventStream(AppError::internal(
                        "the registration signal stream ended"
                    ));
                };

                debug!("registered {signal:?}");

                let Ok(args) = signal.args() else {
                    continue;
                };

                let Some(item) = build_item(conn, args.service.to_string()).await else {
                    continue;
                };

                enrol(&mut item_streams, &mut leases, &item).await;

                publisher(ServiceEvent::Update(TrayEvent::Registered(item))).await;
            }
            signal = unregistered.next() => {
                let Some(signal) = signal else {
                    return TrayWatcherError::EventStream(AppError::internal(
                        "the unregistration signal stream ended"
                    ));
                };

                debug!("unregistered {signal:?}");

                if let Ok(args) = signal.args() {
                    let service = args.service.to_string();

                    leases.remove(&service);

                    publisher(ServiceEvent::Update(TrayEvent::Unregistered(service))).await;
                }
            }
            Some(event) = item_streams.next(), if !item_streams.is_empty() => {
                debug!("tray data {event:?}");
                publisher(ServiceEvent::Update(event)).await;
            }
        }
    }
}
