//! Reading a Waybar configuration into migration inputs.

use std::collections::BTreeMap;

use serde_json::Value;

use super::import::CustomEntry;
/// Pill groups declared beside the module arrays, keyed by their full tail.
///
/// `pill#1` and `pill#2` are distinct groups that happen to share a family;
/// the family name is only a fallback when nothing answers to the exact
/// tail, so collapsing them here would silently rewire the strip.
pub fn read_groups(input: &Value) -> BTreeMap<String, Vec<String>> {
    let mut groups = BTreeMap::new();

    if let Some(object) = input.as_object() {
        for (key, value) in object {
            if let Some(group) = key.strip_prefix("group/") {
                let members: Vec<String> = value
                    .get("modules")
                    .and_then(Value::as_array)
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                groups.insert(group.to_owned(), members);
            }
        }
    }

    groups
}

/// Scripted customs carrying their definitions.
pub fn read_customs(input: &Value) -> Vec<CustomEntry> {
    let mut customs = Vec::new();

    if let Some(object) = input.as_object() {
        for (key, value) in object {
            if let Some(tail) = key.strip_prefix("custom/") {
                let exec = value.get("exec").and_then(Value::as_str).map(str::to_owned);
                if exec.is_none() {
                    continue;
                }
                customs.push(CustomEntry {
                    name: tail.to_owned(),
                    exec,
                    interval: value.get("interval").and_then(Value::as_u64),
                    signal: value
                        .get("signal")
                        .and_then(Value::as_u64)
                        .and_then(|signal| u8::try_from(signal).ok()),
                    command: value
                        .get("on-click")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                });
            }
        }
    }

    customs
}

/// Bar edge the Waybar position names, if this bar knows it.
pub fn read_edge(value: Option<&Value>) -> Option<String> {
    match value.and_then(Value::as_str) {
        Some("top") => Some("Top".to_owned()),
        Some("bottom") => Some("Bottom".to_owned()),
        _ => None
    }
}

/// Compositor layer the Waybar layer names, if this bar knows it.
pub fn read_layer(value: Option<&Value>) -> Option<String> {
    match value.and_then(Value::as_str) {
        Some("background") => Some("Background".to_owned()),
        Some("bottom") => Some("Bottom".to_owned()),
        Some("top") => Some("Top".to_owned()),
        Some("overlay") => Some("Overlay".to_owned()),
        _ => None
    }
}
