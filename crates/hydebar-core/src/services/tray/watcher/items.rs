//! Building registered items and enrolling their event streams.

use std::{future::Future, time::Duration};

use iced::futures::{FutureExt, StreamExt, stream::SelectAll};
use log::{debug, warn};
use masterror::AppError;

use super::{
    TrayEventStream,
    error::TrayWatcherError,
    super::{StatusNotifierItem, TrayData, TrayEvent, app_identity, dbus::StatusNotifierWatcherProxy, icon}
};

/// Longest a tray application may take to answer the item handshake.
///
/// One frozen application must cost the tray a skipped icon, not the whole
/// listener parked on an unanswered call forever.
const ITEM_BUILD_TIMEOUT: Duration = Duration::from_secs(5);

/// Builds a registered item, giving up on one that will not answer.
pub(super) async fn build_item(
    conn: &zbus::Connection,
    name: String
) -> Option<StatusNotifierItem> {
    match tokio::time::timeout(
        ITEM_BUILD_TIMEOUT,
        StatusNotifierItem::new(conn, name.clone())
    )
    .await
    {
        Ok(Ok(item)) => Some(item),
        Ok(Err(err)) => {
            warn!("skipping tray item '{name}': {err}");
            None
        }
        Err(_) => {
            warn!("skipping tray item '{name}': no answer within {ITEM_BUILD_TIMEOUT:?}");
            None
        }
    }
}

/// Reads the items currently registered, skipping the unresponsive ones.
///
/// # Errors
///
/// Returns an error when the watcher proxy cannot be created or the item
/// listing cannot be read; a single item failing its handshake is skipped
/// rather than failing the whole tray.
pub async fn initialize_data(conn: &zbus::Connection) -> Result<TrayData, TrayWatcherError> {
    debug!("initializing tray data");
    let proxy = StatusNotifierWatcherProxy::new(conn).await.map_err(|err| {
        TrayWatcherError::Initialization(AppError::internal(format!(
            "Failed to create StatusNotifierWatcherProxy: {err}"
        )))
    })?;

    let items = proxy
        .registered_status_notifier_items()
        .await
        .map_err(|err| {
            TrayWatcherError::Initialization(AppError::internal(format!(
                "Failed to get registered status notifier items: {err}"
            )))
        })?;

    let mut status_items: Vec<StatusNotifierItem> = Vec::with_capacity(items.len());
    for item in items {
        if let Some(item) = build_item(conn, item).await {
            status_items.retain(|kept| app_identity(&kept.name) != app_identity(&item.name));
            status_items.push(item);
        }
    }

    debug!("created items: {status_items:?}");

    Ok(TrayData(status_items))
}

/// The property and menu streams of one registered item.
fn item_event_streams(
    item: &StatusNotifierItem
) -> impl Future<Output = Vec<TrayEventStream>> + Send + 'static {
    let name = item.name.clone();
    let item_proxy = item.item_proxy.clone();
    let menu_proxy = item.menu_proxy.clone();

    async move {
        let mut streams: Vec<TrayEventStream> = Vec::with_capacity(3);

        let stream = item_proxy.receive_icon_pixmap_changed().await;
        streams.push(
            stream
                .filter_map({
                    let name = name.clone();
                    move |icon| {
                        let name = name.clone();
                        async move {
                            let pixmaps = icon.get().await.ok()?;
                            let icon = tokio::task::spawn_blocking(move || {
                                icon::icon_from_pixmaps(pixmaps)
                            })
                            .await
                            .ok()
                            .flatten()?;

                            Some(TrayEvent::IconChanged(name.clone(), icon))
                        }
                    }
                })
                .boxed()
        );

        let stream = item_proxy.receive_icon_name_changed().await;
        streams.push(
            stream
                .filter_map({
                    let name = name.clone();
                    move |icon_name| {
                        let name = name.clone();
                        async move {
                            let icon_name = icon_name.get().await.ok()?;
                            let icon = tokio::task::spawn_blocking(move || {
                                icon::icon_from_name(&icon_name)
                            })
                            .await
                            .ok()
                            .flatten()?;

                            Some(TrayEvent::IconChanged(name.clone(), icon))
                        }
                    }
                })
                .boxed()
        );

        if let Ok(layout_updated) = menu_proxy.receive_layout_updated().await {
            streams.push(
                layout_updated
                    .filter_map({
                        let name = name.clone();
                        let menu_proxy = menu_proxy.clone();
                        move |_| {
                            debug!("layout update event name {name}");
                            let name = name.clone();
                            let menu_proxy = menu_proxy.clone();
                            async move {
                                menu_proxy
                                    .get_layout(0, -1, &[])
                                    .await
                                    .ok()
                                    .map(|(_, layout)| {
                                        TrayEvent::MenuLayoutChanged(name.clone(), layout)
                                    })
                            }
                        }
                    })
                    .boxed()
            );
        }

        streams
    }
}

/// Seats one item's streams under a lease its unregistration revokes.
///
/// The property and menu streams of a vanished application never end on
/// their own — they pend on the live connection forever. Ending them on
/// the lease is what keeps the merged set from hoarding the dead: an app
/// restarting every hour must not grow the bar by three streams an hour.
pub(super) async fn enrol(
    item_streams: &mut SelectAll<TrayEventStream>,
    leases: &mut std::collections::HashMap<String, iced::futures::channel::oneshot::Sender<()>>,
    item: &StatusNotifierItem
) {
    let (lease, revoked) = iced::futures::channel::oneshot::channel::<()>();
    let revoked = revoked.shared();

    leases.insert(item.name.clone(), lease);

    for stream in item_event_streams(item).await {
        item_streams.push(stream.take_until(revoked.clone()).boxed());
    }
}
