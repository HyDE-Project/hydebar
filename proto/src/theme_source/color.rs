//! Color type and CSS color parsing.

/// Straight (non premultiplied) color with an alpha channel in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel, where `1.0` is fully opaque.
    pub a: f32
}

impl Rgba {
    /// Builds an opaque color from its channels.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            a: 1.0
        }
    }

    /// Builds a color from its channels and an explicit alpha.
    #[expect(
        clippy::self_named_constructors,
        reason = "named after the CSS rgba() notation it mirrors"
    )]
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r,
            g,
            b,
            a
        }
    }
}

/// Parses a CSS color in `rgb()`, `rgba()` or hexadecimal notation.
pub(super) fn parse_color(value: &str) -> Option<Rgba> {
    let value = value.trim();

    if let Some(rest) = value.strip_prefix('#') {
        return parse_hex(rest);
    }

    let lowered = value.to_ascii_lowercase();
    let rest = lowered
        .strip_prefix("rgba")
        .or_else(|| lowered.strip_prefix("rgb"))?
        .trim_start();
    let inner = rest.strip_prefix('(')?;
    let inner = inner[..inner.find(')')?].to_owned();

    let parts: Vec<&str> = inner
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();

    if parts.len() < 3 {
        return None;
    }

    let r = parse_channel(parts[0])?;
    let g = parse_channel(parts[1])?;
    let b = parse_channel(parts[2])?;
    let a = match parts.get(3) {
        Some(part) => parse_alpha(part)?,
        None => 1.0
    };

    Some(Rgba::rgba(r, g, b, a))
}

/// Parses one `rgb()` channel, accepting both absolute and percentage forms.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the channel is clamped to 0.0..=255.0 before the cast"
)]
fn parse_channel(part: &str) -> Option<u8> {
    let value: f32 = match part.strip_suffix('%') {
        Some(number) => number.trim().parse::<f32>().ok()? * 2.55,
        None => part.parse::<f32>().ok()?
    };

    Some(value.round().clamp(0.0, 255.0) as u8)
}

/// Parses an alpha component, accepting both fractions and percentages.
fn parse_alpha(part: &str) -> Option<f32> {
    let value: f32 = match part.strip_suffix('%') {
        Some(number) => number.trim().parse::<f32>().ok()? / 100.0,
        None => part.parse::<f32>().ok()?
    };

    Some(value.clamp(0.0, 1.0))
}

/// Parses a hexadecimal color body (the part after `#`).
fn parse_hex(body: &str) -> Option<Rgba> {
    let body = body.trim();

    if !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    let expand = |c: u8| -> Option<u8> {
        let digit = u8::try_from((c as char).to_digit(16)?).ok()?;
        Some(digit * 17)
    };
    let pair = |slice: &str| -> Option<u8> { u8::from_str_radix(slice, 16).ok() };

    match body.len() {
        3 | 4 => {
            let bytes = body.as_bytes();
            let alpha = match bytes.get(3) {
                Some(&c) => f32::from(expand(c)?) / 255.0,
                None => 1.0
            };
            Some(Rgba::rgba(
                expand(bytes[0])?,
                expand(bytes[1])?,
                expand(bytes[2])?,
                alpha
            ))
        }
        6 | 8 => {
            let alpha = if body.len() == 8 {
                f32::from(pair(&body[6..8])?) / 255.0
            } else {
                1.0
            };
            Some(Rgba::rgba(
                pair(&body[0..2])?,
                pair(&body[2..4])?,
                pair(&body[4..6])?,
                alpha
            ))
        }
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::{super::fixtures::assert_close, *};

    #[test]
    fn parses_rgba_with_alpha() {
        let color = parse_color("rgba(27,29,28,0.8)").expect("color");
        assert_eq!(color.r, 27);
        assert_eq!(color.g, 29);
        assert_eq!(color.b, 28);
        assert_close(color.a, 0.8);
    }

    #[test]
    fn parses_rgb_without_alpha() {
        assert_eq!(
            parse_color("rgb( 195 , 172 , 118 )"),
            Some(Rgba::rgb(195, 172, 118))
        );
    }

    #[test]
    fn parses_hex_colors() {
        assert_eq!(parse_color("#AAF0CD"), Some(Rgba::rgb(170, 240, 205)));
        assert_eq!(parse_color("#abc"), Some(Rgba::rgb(170, 187, 204)));

        let with_alpha = parse_color("#7d6c4b80").expect("color");
        assert_eq!((with_alpha.r, with_alpha.g, with_alpha.b), (125, 108, 75));
        assert_close(with_alpha.a, 128.0 / 255.0);
    }

    #[test]
    fn rejects_unparsable_colors() {
        assert_eq!(parse_color("not-a-color"), None);
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("rgba(1,2)"), None);
    }
}
