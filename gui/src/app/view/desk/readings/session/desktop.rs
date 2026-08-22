//! The state of the desk itself: its keyboard, its sky, its tray, its look.

use super::super::{Panel, push};
use crate::app::state::App;

/// The keyboard layout in force.
pub fn keyboard(app: &App) -> Option<Panel> {
    let active = app.keyboard_layout.active_layout();

    if active.is_empty() {
        return None;
    }

    let named = app
        .config
        .keyboard_layout
        .labels
        .get(active)
        .cloned()
        .unwrap_or_else(|| active.to_owned());

    Panel::of("keyboard", vec![("layout".to_owned(), named)])
}

/// The sky over the configured place.
pub fn weather(app: &App) -> Option<Panel> {
    let sky = app.weather.data();

    if !sky.has_reading() {
        return None;
    }

    Panel::of(
        "weather",
        vec![
            ("place".to_owned(), sky.location.clone()),
            ("temperature".to_owned(), sky.temperature.clone()),
            ("sky".to_owned(), sky.description.clone()),
            ("humidity".to_owned(), sky.humidity.clone()),
            ("wind".to_owned(), sky.wind_speed.clone()),
        ]
    )
}

/// The applications keeping an icon in the tray.
pub fn tray(app: &App) -> Option<Panel> {
    let service = app.tray.service.as_ref()?;

    if service.data.is_empty() {
        return None;
    }

    let mut rows = vec![("items".to_owned(), service.data.len().to_string())];

    for item in service.data.iter() {
        let name = item
            .name
            .rsplit('/')
            .next()
            .unwrap_or(item.name.as_str())
            .to_owned();

        rows.push((String::new(), name));
    }

    Panel::of("tray", rows)
}

/// The desktop theme in force.
pub fn theme(app: &App) -> Option<Panel> {
    let hyde = app.themes.hyde();
    let mut rows = Vec::new();

    push(&mut rows, "in force", hyde.theme.clone());
    push(
        &mut rows,
        "switching to",
        app.themes.switching().map(ToOwned::to_owned)
    );
    rows.push((
        "colours".to_owned(),
        if hyde.wallpaper_colors {
            "from the wallpaper"
        } else {
            "from the theme"
        }
        .to_owned()
    ));
    push(
        &mut rows,
        "wallpaper",
        app.wallpaper_preview.as_ref().and_then(|(path, _)| {
            path.file_stem()
                .map(|name| name.to_string_lossy().into_owned())
        })
    );
    push(&mut rows, "shader", hyde.shader.clone());

    match app.wallpaper_preview.as_ref() {
        Some((_, picture)) => Some(Panel::drawn(
            "theme",
            rows,
            super::super::Figure::Picture(picture.clone())
        )),
        None => Panel::of("theme", rows)
    }
}

/// Who is at the machine and what they are sitting in front of.
///
/// The header of any screen worth the name: a machine is not a list of
/// readings, it is somebody's machine, and the first thing an overview says
/// is whose and where.
pub fn seat(app: &App) -> Option<Panel> {
    let who = hydebar_core::modules::system_info::who::who();
    let mut rows = Vec::new();

    push(
        &mut rows,
        "session",
        who.user.as_ref().map(|user| {
            who.host
                .as_ref()
                .map_or_else(|| user.clone(), |host| format!("{user}@{host}"))
        })
    );
    push(&mut rows, "desktop", who.desktop.clone());
    push(&mut rows, "display", who.seat.clone());
    push(&mut rows, "shell", who.shell.clone());
    push(
        &mut rows,
        "screen",
        app.screen_width
            .zip(app.screen_height)
            .map(|(width, height)| format!("{width:.0} × {height:.0}"))
    );

    Panel::of("seat", rows)
}
