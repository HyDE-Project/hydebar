use std::{future::Future, pin::Pin, time::Duration};

use iced::futures::{Stream, StreamExt, stream::SelectAll};
use log::{debug, error, info, warn};
use masterror::AppError;

use super::{
    StatusNotifierItem, TrayData, TrayEvent, TrayService,
    dbus::{StatusNotifierWatcher, StatusNotifierWatcherProxy},
    icon
};
use crate::services::ServiceEvent;

pub type TrayEventStream = Pin<Box<dyn Stream<Item = TrayEvent> + Send + 'static>>;

/// Longest a tray application may take to answer the item handshake.
///
/// One frozen application must cost the tray a skipped icon, not the whole
/// listener parked on an unanswered call forever.
const ITEM_BUILD_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum TrayWatcherError {
    Connection(AppError),
    Initialization(AppError),
    EventStream(AppError)
}

impl std::fmt::Display for TrayWatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(err) => write!(f, "failed to connect to system bus: {err}"),
            Self::Initialization(err) => write!(f, "failed to initialise tray service: {err}"),
            Self::EventStream(err) => write!(f, "failed to listen for tray events: {err}")
        }
    }
}

impl std::error::Error for TrayWatcherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connection(err) | Self::Initialization(err) | Self::EventStream(err) => {
                err.source()
            }
        }
    }
}

/// The watcher's bus presence, owned for the life of the listener.
///
/// Dropping it aborts the bus-name watching task along with the listener,
/// so a torn-down tray module leaves neither a task nor a claimed name
/// behind — restarts replace the server instead of stacking one per retry.
struct WatcherServer {
    conn:  zbus::Connection,
    watch: tokio::task::JoinHandle<()>
}

impl Drop for WatcherServer {
    fn drop(&mut self) {
        self.watch.abort();
    }
}

/// Builds a registered item, giving up on one that will not answer.
async fn build_item(conn: &zbus::Connection, name: String) -> Option<StatusNotifierItem> {
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
            status_items
                .retain(|kept| super::app_identity(&kept.name) != super::app_identity(&item.name));
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

/// Serves tray events from one bus connection until the bus lets go.
///
/// The registration signals are subscribed exactly once and per-item
/// streams join the running merge as items register — nothing is torn
/// down between registrations, so an item arriving while another is being
/// built is never missed.
async fn serve<F, Fut>(conn: &zbus::Connection, publisher: &mut F) -> TrayWatcherError
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
    for item in data.iter() {
        for stream in item_event_streams(item).await {
            item_streams.push(stream);
        }
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

                for stream in item_event_streams(&item).await {
                    item_streams.push(stream);
                }

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
                    publisher(ServiceEvent::Update(TrayEvent::Unregistered(
                        args.service.to_string()
                    )))
                    .await;
                }
            }
            Some(event) = item_streams.next(), if !item_streams.is_empty() => {
                debug!("tray data {event:?}");
                publisher(ServiceEvent::Update(event)).await;
            }
        }
    }
}

pub async fn start_listening<F, Fut>(mut publisher: F)
where
    F: FnMut(ServiceEvent<TrayService>) -> Fut + Send,
    Fut: Future<Output = ()> + Send
{
    let mut failures: u32 = 0;
    let server = loop {
        match StatusNotifierWatcher::start_server().await {
            Ok((conn, watch)) => {
                break WatcherServer {
                    conn,
                    watch
                };
            }
            Err(err) => {
                error!("{}", TrayWatcherError::Connection(err));
                failures = failures.saturating_add(1);
                tokio::time::sleep(crate::services::reconnect_delay(failures)).await;
            }
        }
    };

    let mut failures: u32 = 0;
    loop {
        let err = serve(&server.conn, &mut publisher).await;
        error!("{err}");

        failures = failures.saturating_add(1);
        tokio::time::sleep(crate::services::reconnect_delay(failures)).await;
    }
}

#[cfg(test)]
mod tests {
    use masterror::AppError;

    use super::TrayWatcherError;

    #[test]
    fn error_variants_have_context() {
        let error = TrayWatcherError::EventStream(AppError::internal("failure"));
        let message = format!("{error}");
        assert!(message.contains("failed to listen"));
    }

    #[test]
    fn connection_errors_name_the_bus() {
        let error = TrayWatcherError::Connection(AppError::internal("boom"));
        let message = format!("{error}");
        assert!(message.contains("failed to connect"));
    }
}
