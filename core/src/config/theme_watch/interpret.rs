//! Translation of theme file events into configuration reloads.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc
};

use hydebar_proto::{compositor_look::CompositorLook, theme_source};
use iced::futures::{
    SinkExt,
    channel::mpsc::{SendError, Sender}
};
use log::{debug, info, warn};

use super::sources::ThemeRoots;
use crate::config::{
    ConfigApplied, ConfigEvent, ConfigManager, ConfigUpdateError,
    watch::{
        Event, WatchedEvent,
        interpret::classify_mask,
        load::{load_candidate_with, send_degradation}
    }
};

/// Reports whether an event touches any of the files the theme is read from.
///
/// A theme switch rewrites several files in an order nothing guarantees, so the
/// watcher cannot key on a single name the way the configuration watcher does;
/// matching against the whole set is what makes a switch observable whichever
/// file lands first.
pub(super) fn interpret_theme_event<E: WatchedEvent>(
    event: &E,
    watched_names: &[OsString]
) -> Option<Event> {
    let name = event.file_name()?;

    if !watched_names
        .iter()
        .any(|watched| watched.as_os_str() == name)
    {
        return None;
    }

    classify_mask(event.mask())
}

/// Re-reads the configuration with the theme currently on disk and publishes
/// it.
///
/// Both a rewritten and a removed source lead here: a theme that lost a file
/// simply contributes fewer values, and the bar should show that state rather
/// than keep painting with the values of a theme that is gone.
///
/// The compositor is asked again on every switch because a `HyDE` theme carries
/// the window rounding the bar matches its islands to, and that rounding lands
/// in the compositor rather than in any file the watcher sees.
pub(super) async fn handle_theme_event(
    output: &mut Sender<ConfigEvent>,
    config_path: &Path,
    roots: &ThemeRoots,
    event: Event,
    manager: Arc<ConfigManager>
) -> Result<(), SendError> {
    debug!("HyDE theme sources reported {event:?}");
    info!("Reloading the configuration to follow the HyDE theme");

    match reload(
        config_path.to_path_buf(),
        roots.clone(),
        Arc::clone(&manager)
    )
    .await
    {
        Ok(applied) => output.send(ConfigEvent::Applied(applied)).await,
        Err(reason) => {
            warn!("Following the HyDE theme failed: {reason}");
            send_degradation(output, manager, reason).await
        }
    }
}

/// Re-reads the configuration and the theme away from the runtime's own
/// threads.
///
/// Every source is read with blocking calls, and asking the compositor for its
/// rounding starts a process per option; done inline, a theme switch would park
/// a runtime worker for as long as those take, in the middle of the burst of
/// events the switch itself produces. Handing the whole read to the blocking
/// pool keeps the bar answering while the desktop is being rebuilt around it.
async fn reload(
    config_path: PathBuf,
    roots: ThemeRoots,
    manager: Arc<ConfigManager>
) -> Result<ConfigApplied, ConfigUpdateError> {
    let read = tokio::task::spawn_blocking(move || {
        let dirs = roots.dirs.clone();
        let layout_dirs = dirs.clone();

        load_candidate_with(
            &config_path,
            &manager,
            || theme_source::load_from(&dirs, &CompositorLook::read()),
            |custom_names| hydebar_proto::bar_layout::load_from(&layout_dirs, custom_names)
        )
    })
    .await;

    match read {
        Ok(outcome) => outcome,
        Err(error) => Err(ConfigUpdateError::state(format!(
            "the theme could not be read: {error}"
        )))
    }
}
