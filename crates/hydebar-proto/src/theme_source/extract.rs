//! Per-file extraction of the values the bar reads from the HyDE stylesheets.

use super::{
    color::parse_color,
    css::{blocks, declarations},
    length::parse_length,
    theme::HydeTheme
};

/// Fills the color fields from a `theme.css` source.
pub(super) fn apply_colors(theme: &mut HydeTheme, source: &str) {
    for (name, value) in define_colors(source) {
        let Some(color) = parse_color(&value) else {
            continue;
        };

        match name.as_str() {
            "bar-bg" => theme.bar_background = Some(color),
            "main-bg" => theme.module_background = Some(color),
            "main-fg" => theme.text = Some(color),
            "wb-act-bg" => theme.active_background = Some(color),
            "wb-act-fg" => theme.active_text = Some(color),
            "wb-hvr-bg" => theme.hover_background = Some(color),
            "wb-hvr-fg" => theme.hover_text = Some(color),
            _ => {}
        }
    }
}

/// Extracts every `@define-color <name> <value>;` pair from a stylesheet.
fn define_colors(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = source;

    while let Some(start) = rest.find("@define-color") {
        let tail = &rest[start + "@define-color".len()..];
        let (declaration, remainder) = match tail.find(';') {
            Some(end) => (&tail[..end], &tail[end + 1..]),
            None => (tail, "")
        };
        rest = remainder;

        let declaration = declaration.trim();
        let Some(split) = declaration.find(char::is_whitespace) else {
            continue;
        };

        let name = declaration[..split].trim();
        let value = declaration[split..].trim();

        if !name.is_empty() && !value.is_empty() {
            out.push((name.to_owned(), value.to_owned()));
        }
    }

    out
}

/// Fills the font fields from a `global.css` source.
pub(super) fn apply_global(theme: &mut HydeTheme, source: &str) {
    let body = universal_block(source).unwrap_or_else(|| source.to_owned());

    for (property, value) in declarations(&body) {
        match property.as_str() {
            "font-family" => {
                if theme.font_family.is_none() {
                    theme.font_family = parse_font_family(&value);
                }
            }
            "font-size" if theme.font_size_px.is_none() => {
                theme.font_size_px = parse_length(&value, None);
            }
            _ => {}
        }
    }
}

/// Returns the body of the `*` rule, if the stylesheet declares one.
fn universal_block(source: &str) -> Option<String> {
    blocks(source)
        .into_iter()
        .find(|(selector, _)| selector.trim() == "*")
        .map(|(_, body)| body)
}

/// Returns the first family of a `font-family` value, without quotes.
fn parse_font_family(value: &str) -> Option<String> {
    let first = value.split(',').next()?.trim();
    let unquoted = first
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            first
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .unwrap_or(first)
        .trim();

    if unquoted.is_empty() {
        None
    } else {
        Some(unquoted.to_owned())
    }
}

/// Returns the radius, in pixels, of the rule that styles a full `#pill`.
///
/// Half pills (`#pill-up`, `#pill-in`, ...) and leaves are ignored, as is any
/// rule that does not target a pill at all.
pub(super) fn parse_pill_radius(source: &str, font_size_px: Option<f32>) -> Option<f32> {
    blocks(source)
        .into_iter()
        .filter(|(selector, _)| targets_full_pill(selector))
        .find_map(|(_, body)| {
            declarations(&body)
                .into_iter()
                .find(|(property, _)| property == "border-radius")
                .and_then(|(_, value)| {
                    let first = value.split_whitespace().next()?;
                    parse_length(first, font_size_px)
                })
        })
}

/// Reports whether a selector group targets the bare `#pill` identifier.
fn targets_full_pill(selector: &str) -> bool {
    let bytes = selector.as_bytes();
    let mut offset = 0usize;

    while let Some(found) = selector[offset..].find("#pill") {
        let start = offset + found;
        let end = start + "#pill".len();

        let boundary = match bytes.get(end) {
            None => true,
            Some(&next) => !(next.is_ascii_alphanumeric() || next == b'-' || next == b'_')
        };

        if boundary {
            return true;
        }

        offset = end;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            css::strip_comments,
            fixtures::{BORDER_RADIUS_CSS, assert_close}
        },
        *
    };

    #[test]
    fn picks_the_full_pill_radius() {
        let radius = parse_pill_radius(&strip_comments(BORDER_RADIUS_CSS), Some(10.0))
            .expect("pill radius");
        assert_close(radius, 4.0);
    }

    #[test]
    fn full_pill_selector_excludes_variants_and_leaves() {
        assert!(targets_full_pill("window#waybar.top #pill button"));
        assert!(targets_full_pill("window#waybar.top #pill,\n#pill menu"));
        assert!(!targets_full_pill("window#waybar.top #pill-up button"));
        assert!(!targets_full_pill("window#waybar.top #pill-in tooltip"));
        assert!(!targets_full_pill("window#waybar.top #pill-down"));
        assert!(!targets_full_pill("window#waybar.top #pill-left"));
        assert!(!targets_full_pill("window#waybar.top #pill-right"));
        assert!(!targets_full_pill("window#waybar.top #pill-out"));
        assert!(!targets_full_pill("window#waybar.top #leaf-inverse"));
        assert!(!targets_full_pill("tooltip"));
    }
}
