//! What the compositor is holding: workspaces, windows and what is playing.

use hydebar_core::services::mpris::PlaybackStatus;

use super::super::{Frame, Miniature, Panel, push};
use crate::app::state::App;

/// Longest a window title is written out before it is cut.
///
/// The canvas has a column rather than a bar entry to write in, so a title is
/// given far more room than the strip gives it — but a browser tab can be a
/// paragraph, and a column of paragraphs is not a list of windows.
const TITLE_ROOM: usize = 48;

/// The workspaces of this screen, and what stands on each of them.
pub fn workspaces(app: &App) -> Option<Panel> {
    let items = app.workspaces.items();

    if items.is_empty() {
        return None;
    }

    let windows: u32 = items.iter().map(|space| u32::from(space.windows)).sum();
    let mut rows = vec![
        ("workspaces".to_owned(), items.len().to_string()),
        ("windows".to_owned(), windows.to_string()),
    ];

    push(
        &mut rows,
        "in view",
        items
            .iter()
            .find(|space| space.active)
            .map(|space| space.name.clone())
    );

    Some(Panel::drawn(
        "workspaces",
        rows,
        super::super::Figure::Overview(miniatures(app))
    ))
}

/// Every workspace of this screen as a miniature of what stands on it.
///
/// The windows are placed by the compositor's own geometry, taken as shares of
/// the screen: a window filling the left half is a shape filling the left half
/// of the miniature, and one dropped in a corner is a shape in that corner.
/// A screen the bar has not been told the size of yields empty rooms rather
/// than a drawing laid out against a guess.
fn miniatures(app: &App) -> Vec<Miniature> {
    let width = app.screen_width.unwrap_or_default();
    let height = app.screen_height.unwrap_or_default();

    app.workspaces
        .items()
        .iter()
        .map(|space| Miniature {
            name:    space.name.clone(),
            active:  space.active,
            urgent:  space.urgent,
            windows: if width > 0.0 && height > 0.0 {
                app.taskbar
                    .clients()
                    .iter()
                    .filter(|client| client.workspace_id == space.id)
                    .map(|client| frame_of(client, width, height))
                    .collect()
            } else {
                Vec::new()
            }
        })
        .collect()
}

/// One window as a share of the screen it stands on.
///
/// The compositor lays its screens out side by side on one plane, so a window
/// on the second monitor stands beyond the first one's width; the share is
/// taken within the screen it belongs to by folding the plane back onto one
/// screen. Anything the fold leaves outside is clamped to the edge rather than
/// drawn beyond it.
fn frame_of(
    client: &hydebar_proto::ports::hyprland::HyprlandClientInfo,
    width: f32,
    height: f32
) -> Frame {
    #[expect(
        clippy::cast_precision_loss,
        reason = "screen coordinates are far below any precision limit"
    )]
    let (left, top, across, down) = (
        client.at.0 as f32,
        client.at.1 as f32,
        client.size.0 as f32,
        client.size.1 as f32
    );

    let x = ((left % width) / width).clamp(0.0, 1.0);
    let y = (top / height).clamp(0.0, 1.0);

    Frame {
        x,
        y,
        width: (across / width).clamp(0.0, 1.0 - x),
        height: (down / height).clamp(0.0, 1.0 - y),
        focused: client.focused,
        floating: client.floating
    }
}

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
