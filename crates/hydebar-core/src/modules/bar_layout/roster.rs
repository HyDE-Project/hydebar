//! Reading the desktop's layout roster and the record of the one in force.

use super::LayoutEntry;

/// Reads the roster and the record of the layout in force.
pub(super) fn list_layouts() -> Vec<LayoutEntry> {
    let listing = std::process::Command::new("timeout")
        .args(["10", "hyde-shell", "waybar", "--json"])
        .output();

    let Ok(output) = listing else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let listed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();

    let active = active_layout_name();

    let names: Vec<String> = listed["layouts"]
        .as_array()
        .map(|layouts| {
            layouts
                .iter()
                .filter_map(|entry| entry["name"].as_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let chosen = active
        .as_deref()
        .and_then(|active| active_index(&names, active));

    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| LayoutEntry {
            active: Some(index) == chosen,
            name
        })
        .collect()
}

/// The one roster position the recorded name speaks of, if it names one.
///
/// The roster lists `hyprdots/02` while the state may record the bare
/// `02`: an exact match wins outright, and the tail spelling is honoured
/// only while it is unambiguous — `02` beside two differently housed
/// `02`s marks nobody rather than everybody.
fn active_index(names: &[String], recorded: &str) -> Option<usize> {
    if let Some(index) = names.iter().position(|name| name == recorded) {
        return Some(index);
    }

    let mut tails = names
        .iter()
        .enumerate()
        .filter(|(_, name)| name.rsplit('/').next().is_some_and(|tail| tail == recorded));

    let first = tails.next().map(|(index, _)| index);

    match tails.next() {
        Some(_) => None,
        None => first
    }
}

/// The layout name the desktop's state records as in force.
///
/// Read through the same shell-variable reader the layout loader uses,
/// so the picker's active mark and the arrangement on the bar can never
/// disagree about what the record says.
fn active_layout_name() -> Option<String> {
    let staterc = hydebar_proto::hyde_dirs::HydeDirs::from_env()?.staterc();
    let source = std::fs::read_to_string(staterc).ok()?;

    hydebar_proto::shell_vars::value_of(&source, "WAYBAR_LAYOUT_NAME")
}
