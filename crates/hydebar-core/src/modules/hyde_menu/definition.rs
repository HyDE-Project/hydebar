//! Reading of the waybar module definition the desktop ships for its menu.

use std::collections::HashMap;

use hydebar_proto::bar_layout::plain_json;
use log::warn;

/// Everything the waybar module definition states about the menu.
pub(super) struct Definition {
    pub(super) glyph:     Option<String>,
    pub(super) menu_file: Option<String>,
    pub(super) actions:   HashMap<String, String>
}

/// Directory the desktop keeps its waybar assets in.
fn data_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/share")))
}

/// Reads the module definition the desktop ships for its menu.
pub(super) fn read_definition() -> Option<Definition> {
    let path = data_dir()?.join("waybar/modules/custom-hyde-menu.jsonc");
    let source = std::fs::read_to_string(&path)
        .inspect_err(|err| warn!("cannot read {}: {err}", path.display()))
        .ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&plain_json(&source))
        .inspect_err(|err| warn!("cannot parse {}: {err}", path.display()))
        .ok()?;
    let module = parsed.get("custom/hyde-menu")?;

    let actions = module
        .get("menu-actions")
        .and_then(|value| value.as_object())
        .map(|table| {
            table
                .iter()
                .filter_map(|(id, command)| Some((id.clone(), command.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default();

    Some(Definition {
        glyph: module
            .get("format")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
        menu_file: module
            .get("menu-file")
            .and_then(|value| value.as_str())
            .map(expand_path),
        actions
    })
}

/// Expands the `${VAR:-fallback}` and `$VAR` forms the desktop uses.
///
/// Only what the shipped files actually contain: one substitution at the
/// front of the path is the whole grammar.
fn expand_path(raw: &str) -> String {
    let expand_var = |name: &str, fallback: Option<&str>| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| fallback.map(str::to_owned))
            .unwrap_or_default()
    };

    if let Some(start) = raw.find("${")
        && let Some(end) = raw[start..].find('}')
    {
        let inner = &raw[start + 2..start + end];
        let (name, fallback) = match inner.split_once(":-") {
            Some((name, fallback)) => (name, Some(fallback)),
            None => (inner, None)
        };
        let fallback = fallback.map(expand_plain);

        return format!(
            "{}{}{}",
            &raw[..start],
            expand_var(name, fallback.as_deref()),
            &raw[start + end + 1..]
        );
    }

    expand_plain(raw)
}

/// Expands a leading `$VAR` with no braces.
fn expand_plain(raw: &str) -> String {
    let Some(name) = raw.strip_prefix('$') else {
        return raw.to_owned();
    };

    let (name, rest) = name.split_at(name.find('/').unwrap_or(name.len()));

    match std::env::var(name) {
        Ok(value) if !value.is_empty() => format!("{value}{rest}"),
        _ => data_dir().map_or_else(|| raw.to_owned(), |dir| format!("{}{rest}", dir.display()))
    }
}
