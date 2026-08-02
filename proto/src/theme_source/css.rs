//! Minimal CSS tokenizer: comment stripping, block splitting and declarations.

/// Removes `/* ... */` comments from a stylesheet.
///
/// An unterminated comment swallows the rest of the input, which mirrors the
/// behaviour of a CSS parser.
pub(super) fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        out.push(' ');

        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out
        }
    }

    out.push_str(rest);
    out
}

/// Splits a stylesheet into `(selector, body)` pairs.
///
/// Nested braces are tracked so that at-rules such as `@media` do not truncate
/// the block they wrap.
pub(super) fn blocks(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut selector_start = 0usize;
    let mut body_start = 0usize;
    let mut depth = 0usize;

    for (index, byte) in source.bytes().enumerate() {
        match byte {
            b'{' => {
                if depth == 0 {
                    body_start = index + 1;
                }
                depth += 1;
            }
            b'}' => {
                if depth == 0 {
                    selector_start = index + 1;
                    continue;
                }

                depth -= 1;

                if depth == 0 {
                    let selector = source[selector_start..body_start.saturating_sub(1)].trim();
                    let body = &source[body_start..index];
                    out.push((selector.to_owned(), body.to_owned()));
                    selector_start = index + 1;
                }
            }
            _ => {}
        }
    }

    out
}

/// Splits a rule body into `(property, value)` pairs, lowercasing properties.
pub(super) fn declarations(body: &str) -> Vec<(String, String)> {
    body.split(';')
        .filter_map(|entry| {
            let entry = entry.trim();
            let colon = entry.find(':')?;
            let property = entry[..colon].trim().to_ascii_lowercase();
            let value = entry[colon + 1..].trim();

            if property.is_empty() || value.is_empty() {
                return None;
            }

            Some((property, value.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments() {
        assert_eq!(strip_comments("a/* x */b"), "a b");
        assert_eq!(strip_comments("a/* unterminated"), "a ");
    }
}
