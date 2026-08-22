//! Every workspace of a screen, and the miniature it is drawn as.

use super::super::super::{Figure, Frame, Miniature, Panel, push};
use crate::app::state::App;

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
        Figure::Overview {
            rooms:  miniatures(app),
            ground: app
                .wallpaper_preview
                .as_ref()
                .map(|(_, picture)| picture.clone())
        }
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
