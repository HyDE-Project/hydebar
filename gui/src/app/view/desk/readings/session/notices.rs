//! What the session wants the user to know about.

use super::super::{Panel, push};
use crate::app::state::App;

/// What is waiting to be installed.
pub fn updates(app: &App) -> Option<Panel> {
    let mut rows = Vec::new();

    push(&mut rows, "pending", app.updates.tooltip());

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
