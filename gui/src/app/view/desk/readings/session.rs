//! The session the bar is running in: its power, its bells, its theme.
//!
//! Every reading here comes off a module the bar already keeps; nothing is
//! sampled twice and nothing is invented. A module that has not answered yet
//! contributes no rows, and a block with no rows is not drawn.

use hydebar_core::modules::battery::{BatteryIcon, IndicatorState, PowerProfile};

use super::{Panel, push};
use crate::app::state::App;

/// The battery: its charge, its direction and what is steering it.
pub fn battery(app: &App) -> Option<Panel> {
    let data = app.battery.data()?;
    let mut rows = vec![
        ("charge".to_owned(), format!("{}%", data.capacity)),
        (
            "state".to_owned(),
            match data.icon {
                BatteryIcon::Full => "full".to_owned(),
                BatteryIcon::Charging(share) => format!("charging, {share}%"),
                BatteryIcon::Discharging(share) => format!("discharging, {share}%"),
                BatteryIcon::Unknown => "unknown".to_owned()
            }
        ),
        (
            "on mains".to_owned(),
            if data.charging { "yes" } else { "no" }.to_owned()
        ),
    ];

    push(
        &mut rows,
        "time left",
        data.time_remaining.map(|left| {
            format!(
                "{}h {:02}m",
                left.as_secs() / 3600,
                (left.as_secs() % 3600) / 60
            )
        })
    );
    push(&mut rows, "profile", profile_name(data.power_profile));
    push(
        &mut rows,
        "condition",
        match data.indicator_state {
            IndicatorState::Danger => Some("critical".to_owned()),
            IndicatorState::Warning => Some("low".to_owned()),
            IndicatorState::Success | IndicatorState::Normal => None
        }
    );

    Panel::of("battery", rows)
}

/// The name of a power profile, absent when the session reports none.
fn profile_name(profile: PowerProfile) -> Option<String> {
    match profile {
        PowerProfile::Balanced => Some("balanced".to_owned()),
        PowerProfile::Performance => Some("performance".to_owned()),
        PowerProfile::PowerSaver => Some("power saver".to_owned()),
        PowerProfile::Unknown => None
    }
}

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

/// Whether the session is being held awake.
pub fn session_idle(app: &App) -> Option<Panel> {
    Panel::of(
        "session",
        vec![(
            "idle".to_owned(),
            if app.control_center.is_idle_inhibited() {
                "held awake"
            } else {
                "free to sleep"
            }
            .to_owned()
        )]
    )
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

    Panel::of("theme", rows)
}
