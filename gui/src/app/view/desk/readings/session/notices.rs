//! What the session wants the user to know about.

use super::super::{Panel, push};
use crate::app::state::App;

/// Longest a list of pending packages is written out.
///
/// A machine left alone for a month has hundreds waiting, and a column of
/// hundreds is a wall rather than a reading. The rest are counted instead.
const LISTED: usize = 12;

/// What is waiting to be installed, package by package.
pub fn updates(app: &App) -> Option<Panel> {
    let pending = app.updates.updates();
    let behind = app.updates.hyde_pending();
    let mut rows = Vec::new();

    push(&mut rows, "state", app.updates.tooltip());
    rows.push(("packages".to_owned(), pending.len().to_string()));

    if behind > 0 {
        rows.push(("desktop".to_owned(), format!("{behind} commits behind")));
    }

    for update in pending.iter().take(LISTED) {
        let (from, to) = update.versions();

        rows.push((update.package().to_owned(), format!("{from} → {to}")));
    }

    if pending.len() > LISTED {
        rows.push((
            String::new(),
            format!("and {} more", pending.len() - LISTED)
        ));
    }

    Panel::of("updates", rows)
}

/// The bells: what is unread and whether they are being held back.
pub fn notifications(app: &App) -> Option<Panel> {
    let service = app.notifications.service.as_ref()?;
    let mut rows = vec![
        ("unread".to_owned(), service.unread_count().to_string()),
        (
            "stored".to_owned(),
            service.get_notifications().len().to_string()
        ),
        (
            "do not disturb".to_owned(),
            if service.is_dnd() { "on" } else { "off" }.to_owned()
        ),
    ];

    push(
        &mut rows,
        "latest",
        service
            .get_notifications()
            .first()
            .map(|latest| latest.summary.clone())
    );

    Panel::of("notifications", rows)
}

/// What is watching or listening right now.
pub fn privacy(app: &App) -> Option<Panel> {
    let service = app.privacy.service.as_ref()?;

    let state = |in_use: bool| if in_use { "in use" } else { "idle" }.to_owned();

    Panel::of(
        "privacy",
        vec![
            ("camera".to_owned(), state(service.webcam_access())),
            ("microphone".to_owned(), state(service.microphone_access())),
            (
                "screen share".to_owned(),
                state(service.screenshare_access())
            ),
        ]
    )
}
