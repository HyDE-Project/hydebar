//! Inventing custom modules for the inline text entries a layout defines.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::config::CustomModuleDef;

/// Invents a custom module for an inline text entry, when it is one.
///
/// Only a `custom/` entry the layout file defines beside the arrays
/// qualifies, and only when that definition states a literal `format` — a
/// definition with an `exec` is a live module the bar cannot restate from a
/// label. The `#variant` stays in the name so two text entries stay two
/// modules. A name the configuration already defines is never invented over.
pub(super) fn synthesize_text_entry(
    root: &Value,
    name: &str,
    custom: &BTreeSet<&str>
) -> Option<CustomModuleDef> {
    let tail = name.strip_prefix("custom/")?;

    if custom.contains(tail) {
        return None;
    }

    let definition = root.get(name)?;

    if definition.get("exec").is_some() {
        return None;
    }

    let label = definition.get("format")?.as_str()?.trim();

    if label.is_empty() {
        return None;
    }

    let command = definition
        .get("on-click")
        .and_then(Value::as_str)
        .unwrap_or_default();

    Some(CustomModuleDef {
        name: tail.to_owned(),
        command: command.to_owned(),
        icon: Some(label.to_owned()),
        ..CustomModuleDef::default()
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        bar_layout::parse,
        config::{ModuleDef, ModuleName}
    };

    /// An inline text entry becomes a custom module carrying its label and
    /// its click command; an entry running a live command stays skipped.
    #[test]
    fn an_inline_text_entry_is_invented_as_a_custom_module() {
        let source = r#"{
            "modules-left": ["custom/text#two", "custom/help#macos", "image#wallpaper"],
            "custom/text#two": { "format": "File", "on-click": "dolphin" },
            "custom/help#macos": { "format": "Help", "exec": "some-live-feed" }
        }"#;

        let restated = parse(source, &[]).expect("layout");

        assert_eq!(
            restated.modules.left,
            vec![
                ModuleDef::Single(ModuleName::Custom("text#two".into())),
                ModuleDef::Single(ModuleName::Wallpaper),
            ]
        );
        assert_eq!(restated.synthesized.len(), 1);
        assert_eq!(restated.synthesized[0].name, "text#two");
        assert_eq!(restated.synthesized[0].icon.as_deref(), Some("File"));
        assert_eq!(restated.synthesized[0].command, "dolphin");
    }
}
