//! Deriving every spelling of a module name from the one the developer types.
//!
//! The developer names the module once, in `snake_case`, and every other
//! form — the `PascalCase` type, the human label, the file paths — is
//! derived, so the scaffold cannot spell the same module two ways.

use std::fmt;

/// Every spelling of a new module name.
pub struct ModuleNames {
    /// Directory and file stem: `demo`.
    pub snake: String,
    /// Type and variant: `Demo`.
    pub pascal: String,
    /// Human label: `Demo` unless `--label` says otherwise.
    pub label: String,
    /// Variant documentation: `--doc` or a marker to replace.
    pub doc: String,
}

/// Why a name cannot become a module.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidName {
    /// What was typed.
    pub input: String,
    /// What is wrong with it.
    pub reason: &'static str,
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
    doc: Option<&str>,
) -> Result<ModuleNames, InvalidName> {
    if input.is_empty() {
        return Err(InvalidName {
            input: input.to_owned(),
            reason: "a name is required",
        });
    }

    if !input
        .chars()
        .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '_')
    {
        return Err(InvalidName {
            input: input.to_owned(),
            reason: "use snake_case: lowercase letters, digits and underscores",
        });
    }

    if input.starts_with(|char: char| char.is_ascii_digit()) || input.starts_with('_') {
        return Err(InvalidName {
            input: input.to_owned(),
            reason: "start with a lowercase letter",
        });
    }

    if is_rust_keyword(input) {
        return Err(InvalidName {
            input: input.to_owned(),
            reason: "a reserved Rust keyword cannot name a module",
        });
    }

    let pascal: String = input
        .split('_')
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect();

    let doc = doc.unwrap_or("TODO: describe what the module does.");

    if doc
        .chars()
        .any(|char| matches!(char, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    {
        return Err(InvalidName {
            input: doc.to_owned(),
            reason: "the doc is one line; line terminators are not allowed",
        });
    }

    Ok(ModuleNames {
        snake: input.to_owned(),
        label: label.unwrap_or(&pascal).to_owned(),
        doc: doc.to_owned(),
        pascal,
    })
}

/// Whether `word` is a reserved Rust keyword and cannot name a module.
///
/// A keyword cannot serve as a `snake_case` file stem or a generated
/// `PascalCase` type: `mod`, `use`, `struct`, `self`, `super`, … would not
/// parse. The list is the compiler's own reserved set; `Self` cannot slip in
/// because names are lowercase. Raw identifiers (`r#type`) are not offered —
/// a scaffold names a clean module.
fn is_rust_keyword(word: &str) -> bool {
    const RUST_KEYWORDS: &[&str] = &[
        "as",
        "async",
        "await",
        "box",
        "break",
        "const",
        "continue",
        "crate",
        "dyn",
        "else",
        "enum",
        "extern",
        "false",
        "fn",
        "for",
        "if",
        "impl",
        "in",
        "let",
        "loop",
        "macro",
        "macro_rules",
        "match",
        "mod",
        "move",
        "mut",
        "pub",
        "ref",
        "return",
        "self",
        "Self",
        "static",
        "struct",
        "super",
        "trait",
        "true",
        "try",
        "type",
        "union",
        "unsafe",
        "use",
        "where",
        "while",
        "yield",
    ];

    RUST_KEYWORDS.contains(&word)
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

    #[test]
    fn reserved_rust_keywords_are_refused() {
        for input in [
            "self", "super", "crate", "mod", "fn", "match", "impl", "struct", "enum", "trait",
            "use", "pub", "let", "const", "static", "mut", "ref", "move", "as", "in", "for",
            "loop", "while", "if", "else", "return", "break", "continue", "where", "type", "dyn",
            "extern", "unsafe", "async", "await",
        ] {
            assert!(
                derive(input, None, None).is_err(),
                "keyword '{input}' must not derive"
            );
        }
    }

    #[test]
    fn a_keyword_like_stem_that_is_not_reserved_is_allowed() {
        assert!(derive("my_type", None, None).is_ok());
        assert!(derive("asyncly", None, None).is_ok());
    }

    #[test]
    fn a_multi_line_doc_is_refused() {
        for doc in [
            "first\nsecond",
            "first\r\nsecond",
            "first\rsecond",
            "first\u{2028}second",
        ] {
            assert!(
                derive("demo", None, Some(doc)).is_err(),
                "doc with a line terminator must not derive"
            );
        }
    }
}
