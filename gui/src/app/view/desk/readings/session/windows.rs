//! What the compositor is holding: workspaces, windows and what is playing.
//!
//! Two rooms: [`rooms`] is the workspaces and the miniature each of them is
//! drawn as, and here are the windows themselves, what is playing through
//! them and whichever submap the keyboard is in.

mod rooms;

use hydebar_core::services::mpris::PlaybackStatus;
pub use rooms::workspaces;

use super::super::{Panel, push};
use crate::app::state::App;

/// Longest a window title is written out before it is cut.
///
/// The canvas has a column rather than a bar entry to write in, so a title is
/// given far more room than the strip gives it — but a browser tab can be a
/// paragraph, and a column of paragraphs is not a list of windows.
const TITLE_ROOM: usize = 48;

/// The windows the compositor is holding, whichever screen they are on.
pub fn windows(app: &App) -> Option<Panel> {
    let clients = app.taskbar.clients();

    if clients.is_empty() {
        return None;
    }

    let floating = clients.iter().filter(|client| client.floating).count();
    let mut rows = vec![("open".to_owned(), clients.len().to_string())];

    if floating > 0 {
        rows.push(("floating".to_owned(), floating.to_string()));
    }

    push(
        &mut rows,
        "in focus",
        app.window_title.full().map(shortened)
    );

    for client in clients {
        rows.push((client.class.clone(), shortened(client.title.as_str())));
    }

    Panel::of("windows", rows)
}

/// A title cut to the room a column line has, on a word where it can be.
fn shortened(title: &str) -> String {
    if title.chars().count() <= TITLE_ROOM {
        return title.to_owned();
    }

    let kept: String = title.chars().take(TITLE_ROOM).collect();

    match kept.rsplit_once(' ') {
        Some((head, _)) if head.chars().count() > TITLE_ROOM / 2 => format!("{head}…"),
        _ => format!("{kept}…")
    }
}

/// What the session is playing, on a player that has anything loaded.
pub fn playing(app: &App) -> Option<Panel> {
    let players = app.media_player.players();
    let leading = players
        .iter()
        .find(|player| matches!(player.state, PlaybackStatus::Playing))
        .or_else(|| players.first())?;

    let metadata = leading.metadata.as_ref()?;
    let mut rows = Vec::new();

    push(&mut rows, "track", metadata.title.clone());
    push(
        &mut rows,
        "by",
        metadata
            .artists
            .as_ref()
            .filter(|artists| !artists.is_empty())
            .map(|artists| artists.join(", "))
    );
    rows.push((
        "state".to_owned(),
        match leading.state {
            PlaybackStatus::Playing => "playing",
            PlaybackStatus::Paused => "paused",
            PlaybackStatus::Stopped => "stopped"
        }
        .to_owned()
    ));
    rows.push(("player".to_owned(), player_name(&leading.service)));
    push(
        &mut rows,
        "volume",
        leading.volume.map(|volume| format!("{}%", volume.round()))
    );

    if players.len() > 1 {
        rows.push(("players".to_owned(), players.len().to_string()));
    }

    Panel::of("playing", rows)
}

/// The last part of a bus name, which is what the player calls itself.
fn player_name(service: &str) -> String {
    service.rsplit('.').next().unwrap_or(service).to_owned()
}

/// The keyboard submap the compositor is in, while it is in one.
pub fn submap(app: &App) -> Option<Panel> {
    let submap = app.keyboard_submap.active();

    if submap.is_empty() {
        return None;
    }

    Panel::of("submap", vec![("in force".to_owned(), submap.to_owned())])
}
