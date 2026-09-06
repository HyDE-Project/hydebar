//! Deriving every spelling of a module name from the one the developer types.
//!
//! The developer names the module once, in `snake_case`, and every other
//! form — the `PascalCase` type, the human label, the file paths — is
//! derived, so the scaffold cannot spell the same module two ways.

use std::fmt;

/// Every spelling of a new module name.
pub struct ModuleNames {
    /// Directory and file stem: `demo`.
    pub snake:  String,
    /// Type and variant: `Demo`.
    pub pascal: String,
    /// Human label: `Demo` unless `--label` says otherwise.
    pub label:  String,
    /// Variant documentation: `--doc` or a marker to replace.
    pub doc:    String
}

/// Why a name cannot become a module.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidName {
    /// What was typed.
    pub input:  String,
    /// What is wrong with it.
    pub reason: &'static str
}

impl fmt::Display for InvalidName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid module name '{}': {}", self.input, self.reason)
    }
}

/// Derives every spelling from a `snake_case` name and an optional label.
pub fn derive(
    input: &str,
    label: Option<&str>,
    doc: Option<&str>
) -> Result<ModuleNames, InvalidName> {
    if input.is_empty() {
        return Err(InvalidName {
            input:  input.to_owned(),
            reason: "a name is required"
        });
    }

    if !input
        .chars()
        .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '_')
    {
        return Err(InvalidName {
            input:  input.to_owned(),
            reason: "use snake_case: lowercase letters, digits and underscores"
        });
    }

    if input.starts_with(|char: char| char.is_ascii_digit()) || input.starts_with('_') {
        return Err(InvalidName {
            input:  input.to_owned(),
            reason: "start with a lowercase letter"
        });
    }

    let pascal: String = input
        .split('_')
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect();

    Ok(ModuleNames {
        snake: input.to_owned(),
        label: label.unwrap_or(&pascal).to_owned(),
        doc: doc
            .unwrap_or("TODO: describe what the module does.")
            .to_owned(),
        pascal
    })
}

/// Capitalizes one `snake_case` segment: `submap` becomes `Submap`.
fn capitalize(segment: &str) -> String {
    let mut chars = segment.chars();

    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_name_derives_every_spelling() {
        let names = derive("demo", None, None).expect("a plain name derives");

        assert_eq!(names.snake, "demo");
        assert_eq!(names.pascal, "Demo");
        assert_eq!(names.label, "Demo");
    }

    #[test]
    fn segments_capitalize_into_one_type_name() {
        let names = derive("night_light", None, None).expect("segments derive");

        assert_eq!(names.pascal, "NightLight");
    }

    #[test]
    fn an_explicit_label_and_doc_win() {
        let names = derive("demo", Some("Demo thing"), Some("Does things."))
            .expect("explicit spellings derive");

        assert_eq!(names.label, "Demo thing");
        assert_eq!(names.doc, "Does things.");
    }

    #[test]
    fn names_outside_snake_case_are_refused() {
        for input in ["", "Demo", "demo-mode", "demo mode", "_demo", "9demo"] {
            assert!(
                derive(input, None, None).is_err(),
                "{input} must not derive"
            );
        }
    }
}
