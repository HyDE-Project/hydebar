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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{io::Write, sync::Mutex};

    use super::*;

    /// The environment is process-wide, so the tests that set it take turns.
    static ENVIRONMENT: Mutex<()> = Mutex::new(());

    /// States a variable for the test currently holding [`ENVIRONMENT`].
    fn set_env(key: &str, value: impl AsRef<std::ffi::OsStr>) {
        #[expect(
            unsafe_code,
            reason = "the ENVIRONMENT lock keeps every other test off the process environment"
        )]
        unsafe {
            std::env::set_var(key, value);
        }
    }

    /// Clears a variable for the test currently holding [`ENVIRONMENT`].
    fn unset_env(key: &str) {
        #[expect(
            unsafe_code,
            reason = "the ENVIRONMENT lock keeps every other test off the process environment"
        )]
        unsafe {
            std::env::remove_var(key);
        }
    }

    /// Runs `body` with `XDG_DATA_HOME` pointing at a scratch directory that
    /// holds `definition` as the desktop's menu module, if given.
    fn with_data_home<T>(definition: Option<&str>, body: impl FnOnce() -> T) -> T {
        let _guard = ENVIRONMENT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = tempfile::tempdir().expect("a scratch data home");

        if let Some(definition) = definition {
            let modules = home.path().join("waybar/modules");
            std::fs::create_dir_all(&modules).expect("the module directory is made");

            let mut file = std::fs::File::create(modules.join("custom-hyde-menu.jsonc"))
                .expect("the module file is made");
            file.write_all(definition.as_bytes())
                .expect("the module is written");
        }

        let previous = std::env::var_os("XDG_DATA_HOME");
        set_env("XDG_DATA_HOME", home.path());

        let outcome = body();

        match previous {
            Some(value) => set_env("XDG_DATA_HOME", value),
            None => unset_env("XDG_DATA_HOME")
        }

        outcome
    }

    #[test]
    fn a_desktop_that_ships_no_module_states_nothing() {
        assert!(with_data_home(None, read_definition).is_none());
    }

    #[test]
    fn a_module_file_that_is_not_json_states_nothing() {
        assert!(with_data_home(Some("{ not json"), read_definition).is_none());
    }

    #[test]
    fn a_json_file_without_the_menu_module_states_nothing() {
        assert!(with_data_home(Some(r#"{"custom/other": {}}"#), read_definition).is_none());
    }

    #[test]
    fn the_module_states_its_glyph_its_menu_file_and_its_actions() {
        let definition = with_data_home(
            Some(
                r#"{
                    // the desktop comments its modules
                    "custom/hyde-menu": {
                        "format": "",
                        "menu-file": "/etc/hyde/menu.ui",
                        "menu-actions": { "lock": "hyprlock", "quit": "uwsm stop" }
                    }
                }"#
            ),
            read_definition
        )
        .expect("the module is read");

        assert_eq!(definition.glyph.as_deref(), Some("\u{f303}"));
        assert_eq!(definition.menu_file.as_deref(), Some("/etc/hyde/menu.ui"));
        assert_eq!(
            definition.actions.get("lock").map(String::as_str),
            Some("hyprlock")
        );
        assert_eq!(
            definition.actions.get("quit").map(String::as_str),
            Some("uwsm stop")
        );
    }

    #[test]
    fn a_module_that_states_nothing_about_itself_is_still_read() {
        let definition = with_data_home(Some(r#"{"custom/hyde-menu": {}}"#), read_definition)
            .expect("the module is read");

        assert!(definition.glyph.is_none());
        assert!(definition.menu_file.is_none());
        assert!(definition.actions.is_empty());
    }

    #[test]
    fn an_action_that_is_not_a_command_is_dropped() {
        let definition = with_data_home(
            Some(r#"{"custom/hyde-menu": {"menu-actions": {"lock": 3, "quit": "exit"}}}"#),
            read_definition
        )
        .expect("the module is read");

        assert!(!definition.actions.contains_key("lock"));
        assert_eq!(
            definition.actions.get("quit").map(String::as_str),
            Some("exit")
        );
    }

    #[test]
    fn a_path_naming_no_variable_is_left_as_it_stands() {
        assert_eq!(expand_path("/etc/hyde/menu.ui"), "/etc/hyde/menu.ui");
    }

    #[test]
    fn a_braced_variable_is_replaced_by_what_it_holds() {
        let _guard = ENVIRONMENT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        set_env("HYDE_MENU_ROOT", "/opt/hyde");

        assert_eq!(
            expand_path("${HYDE_MENU_ROOT}/menu.ui"),
            "/opt/hyde/menu.ui"
        );

        unset_env("HYDE_MENU_ROOT");
    }

    #[test]
    fn a_braced_variable_nobody_set_falls_back_to_what_the_file_names() {
        let _guard = ENVIRONMENT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unset_env("HYDE_MENU_ROOT");

        assert_eq!(
            expand_path("${HYDE_MENU_ROOT:-/etc/hyde}/menu.ui"),
            "/etc/hyde/menu.ui"
        );
    }

    #[test]
    fn a_variable_holding_nothing_is_treated_as_unset() {
        let _guard = ENVIRONMENT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        set_env("HYDE_MENU_ROOT", "");

        assert_eq!(
            expand_path("${HYDE_MENU_ROOT:-/etc/hyde}/menu.ui"),
            "/etc/hyde/menu.ui"
        );

        unset_env("HYDE_MENU_ROOT");
    }

    #[test]
    fn a_braced_variable_with_no_fallback_and_no_value_leaves_a_bare_path() {
        let _guard = ENVIRONMENT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unset_env("HYDE_MENU_ROOT");

        assert_eq!(expand_path("${HYDE_MENU_ROOT}/menu.ui"), "/menu.ui");
    }

    /// An unclosed brace is not a form the desktop's files use; the reader
    /// falls through to the bare-variable rule rather than guessing, so the
    /// path lands under the data directory instead of being handed on with a
    /// `$` in it that nothing could open.
    #[test]
    fn an_unclosed_brace_falls_through_to_the_bare_variable_rule() {
        with_data_home(None, || {
            let expanded = expand_path("${HYDE/menu.ui");

            assert!(expanded.ends_with("/menu.ui"));
            assert!(!expanded.contains('$'));
        });
    }

    #[test]
    fn a_bare_variable_is_replaced_by_what_it_holds() {
        let _guard = ENVIRONMENT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        set_env("HYDE_MENU_ROOT", "/opt/hyde");

        assert_eq!(expand_plain("$HYDE_MENU_ROOT/menu.ui"), "/opt/hyde/menu.ui");

        unset_env("HYDE_MENU_ROOT");
    }

    #[test]
    fn a_bare_variable_nobody_set_falls_back_to_the_data_directory() {
        with_data_home(None, || {
            let expanded = expand_plain("$HYDE_MENU_ROOT/menu.ui");

            assert!(expanded.ends_with("/menu.ui"));
            assert!(!expanded.starts_with('$'));
        });
    }

    #[test]
    fn a_path_with_no_leading_variable_is_left_as_it_stands() {
        assert_eq!(expand_plain("/etc/hyde/menu.ui"), "/etc/hyde/menu.ui");
    }
}
