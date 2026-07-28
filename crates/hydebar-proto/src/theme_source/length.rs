//! CSS length parsing, normalised to pixels.

/// Conversion factor between CSS points and CSS pixels.
const PX_PER_PT: f32 = 96.0 / 72.0;

/// Fallback font size used to resolve `em` lengths when the stylesheet does not
/// declare one.
const DEFAULT_EM_PX: f32 = 16.0;

/// Parses a CSS length into pixels.
///
/// `px`, `pt`, `em`, `rem` and unitless numbers are understood; anything else
/// yields [`None`]. `em` and `rem` fall back to a 16px root when the stylesheet
/// has no explicit font size.
pub(super) fn parse_length(value: &str, font_size_px: Option<f32>) -> Option<f32> {
    let value = value.trim();
    let split = value
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
        .unwrap_or(value.len());

    let number: f32 = value[..split].parse().ok()?;
    let unit = value[split..].trim().to_ascii_lowercase();

    match unit.as_str() {
        "" | "px" => Some(number),
        "pt" => Some(number * PX_PER_PT),
        "em" | "rem" => Some(number * font_size_px.unwrap_or(DEFAULT_EM_PX)),
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::{super::fixtures::assert_close, *};

    #[test]
    fn converts_points_to_pixels() {
        assert_close(parse_length("3pt", None).expect("length"), 4.0);
        assert_close(parse_length("9pt", None).expect("length"), 12.0);
        assert_close(parse_length("10px", None).expect("length"), 10.0);
        assert_close(parse_length("0em", Some(10.0)).expect("length"), 0.0);
        assert_close(parse_length("2em", Some(10.0)).expect("length"), 20.0);
        assert_eq!(parse_length("3vh", None), None);
    }
}
