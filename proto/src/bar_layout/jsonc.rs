//! Stripping the relaxed syntax the layout files carry down to plain JSON.

/// Strips the relaxed syntax the layout files carry down to plain JSON.
///
/// Layout files carry line and block comments and the odd trailing comma;
/// both are removed outside of strings so the strict parser can take the
/// rest.
#[must_use]
pub fn plain_json(source: &str) -> String {
    let without_comments = strip_comments(source);

    strip_trailing_commas(&without_comments)
}

/// Removes line and block comments, leaving string contents alone.
fn strip_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if in_string {
            output.push(c);
            match c {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => in_string = false,
                _ => escaped = false
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                output.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        output.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = ' ';
                for skipped in chars.by_ref() {
                    if last == '*' && skipped == '/' {
                        break;
                    }
                    last = skipped;
                }
            }
            _ => output.push(c)
        }
    }

    output
}

/// Removes commas left dangling before a closing brace or bracket.
fn strip_trailing_commas(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut in_string = false;
    let mut escaped = false;

    for c in source.chars() {
        if in_string {
            output.push(c);
            match c {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => in_string = false,
                _ => escaped = false
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                output.push(c);
            }
            '}' | ']' => {
                while output.trim_end().ends_with(',') {
                    let end = output.trim_end().len();
                    output.truncate(end - 1);
                }
                output.push(c);
            }
            _ => output.push(c)
        }
    }

    output
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use crate::{
        bar_layout::parse,
        config::{ModuleDef, ModuleName}
    };

    #[test]
    fn comments_and_trailing_commas_are_tolerated() {
        let source = "{\n /* block */ \"modules-left\": [\"clock\",], // line\n}";

        let modules = parse(source, &[]).expect("layout").modules;

        assert_eq!(modules.left, vec![ModuleDef::Single(ModuleName::Clock)]);
    }

    #[test]
    fn comment_markers_inside_strings_are_left_alone() {
        let source = r#"{ "modules-left": ["clock"], "note": "https://example.com/a" }"#;

        assert!(parse(source, &[]).is_some());
    }
}
