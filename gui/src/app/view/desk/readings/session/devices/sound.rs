//! What the session plays out and what it listens in through.

use hydebar_core::services::audio::AudioData;

use super::super::super::Panel;
use crate::app::state::App;

/// The sound server: what is playing out, what is listening in.
pub fn sound(app: &App) -> Option<Panel> {
    let audio = app.control_center.audio_readings()?;
    let mut rows = vec![
        ("output".to_owned(), named(audio, true)),
        (
            "volume".to_owned(),
            level(audio.cur_sink_volume, muted(audio, true))
        ),
        ("input".to_owned(), named(audio, false)),
        (
            "input level".to_owned(),
            level(audio.cur_source_volume, muted(audio, false))
        ),
    ];

    rows.push(("outputs".to_owned(), audio.sinks.len().to_string()));
    rows.push(("inputs".to_owned(), audio.sources.len().to_string()));

    Panel::of("sound", rows)
}

/// The description of the default sink or source, by its own name.
fn named(audio: &AudioData, out: bool) -> String {
    let (devices, wanted) = if out {
        (&audio.sinks, &audio.server_info.default_sink)
    } else {
        (&audio.sources, &audio.server_info.default_source)
    };

    devices
        .iter()
        .find(|device| device.name == *wanted)
        .map_or_else(|| wanted.clone(), |device| device.description.clone())
}

/// Whether the default device of that direction is silenced.
fn muted(audio: &AudioData, out: bool) -> bool {
    let (devices, wanted) = if out {
        (&audio.sinks, &audio.server_info.default_sink)
    } else {
        (&audio.sources, &audio.server_info.default_source)
    };

    devices
        .iter()
        .find(|device| device.name == *wanted)
        .is_some_and(|device| device.is_mute)
}

/// A volume, said as a share and as a state.
fn level(volume: i32, muted: bool) -> String {
    if muted {
        return format!("{volume}%, muted");
    }

    format!("{volume}%")
}
