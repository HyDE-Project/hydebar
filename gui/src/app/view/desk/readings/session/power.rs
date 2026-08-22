//! What the machine runs on, and whether it is allowed to sleep.

use hydebar_core::modules::battery::{BatteryIcon, IndicatorState, PowerProfile};

use super::super::{Panel, push};
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
        "health",
        data.health.map(|share| format!("{share}% of new"))
    );
    push(
        &mut rows,
        "cycles",
        data.cycles.map(|cycles| cycles.to_string())
    );
    push(
        &mut rows,
        "draw",
        data.watts.map(|watts| format!("{watts:.1} W"))
    );
    push(
        &mut rows,
        "charge held",
        data.watt_hours
            .map(|(now, full)| format!("{now:.1} of {full:.1} Wh"))
    );
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
