//! A size that can be stated as absolute pixels or as a percentage of the
//! screen height.
//!
//! The configuration accepts either form for every layout dimension: a plain
//! number is pixels, a string ending in `%` is a fraction of the screen the
//! bar lands on.  Percentages are resolved once at startup, after the
//! compositor reports the focused screen, so the bar adapts without the user
//! having to retune every value when moving between monitors.

use serde::{Deserialize, Deserializer, de::Visitor};

/// A layout dimension that may be given in pixels or as a screen-height
/// percentage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeValue {
    /// An absolute size in logical pixels.
    Pixels(f32),
    /// A fraction of the screen height, e.g. `1.5` means 1.5 %.
    Percent(f32)
}

impl SizeValue {
    /// Resolves the value against a known screen height, in logical pixels.
    ///
    /// [`SizeValue::Pixels`] is returned unchanged; [`SizeValue::Percent`]
    /// is converted to `screen_height * pct / 100`.
    #[must_use]
    pub fn resolve(self, screen_height: f32) -> f32 {
        match self {
            Self::Pixels(px) => px,
            Self::Percent(pct) => screen_height * pct / 100.0
        }
    }
}

struct SizeValueVisitor;

impl Visitor<'_> for SizeValueVisitor {
    type Value = SizeValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a number (pixels) or a percentage string (e.g. \"1.5%\")")
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E> {
        Ok(SizeValue::Pixels(v))
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "TOML values fit f32 for layout dimensions"
    )]
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
        Ok(SizeValue::Pixels(v as f32))
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "TOML values fit f32 for layout dimensions"
    )]
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
        Ok(SizeValue::Pixels(v as f32))
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "TOML values fit f32 for layout dimensions"
    )]
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
        Ok(SizeValue::Pixels(v as f32))
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error
    {
        let trimmed = s.trim();

        if let Some(pct_str) = trimmed.strip_suffix('%') {
            let value: f32 = pct_str
                .trim()
                .parse()
                .map_err(|_| E::custom("invalid percentage value"))?;

            if value <= 0.0 {
                return Err(E::custom("percentage must be greater than 0"));
            }

            if value > 100.0 {
                return Err(E::custom("percentage cannot exceed 100"));
            }

            return Ok(SizeValue::Percent(value));
        }

        let value: f32 = trimmed
            .parse()
            .map_err(|_| E::custom("invalid size value"))?;

        Ok(SizeValue::Pixels(value))
    }
}

impl<'de> Deserialize<'de> for SizeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        deserializer.deserialize_any(SizeValueVisitor)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    #[derive(Deserialize, Debug)]
    struct Wrapper {
        size: SizeValue
    }

    #[test]
    fn a_plain_number_is_pixels() {
        let w: Wrapper = toml::from_str("size = 38.0").expect("plain number should deserialize");
        assert_eq!(w.size, SizeValue::Pixels(38.0));
    }

    #[test]
    fn an_integer_is_pixels() {
        let w: Wrapper = toml::from_str("size = 38").expect("integer should deserialize");
        assert_eq!(w.size, SizeValue::Pixels(38.0));
    }

    #[test]
    fn a_percentage_string_is_percent() {
        let w: Wrapper =
            toml::from_str("size = \"1.5%\"").expect("percentage string should deserialize");
        assert_eq!(w.size, SizeValue::Percent(1.5));
    }

    #[test]
    fn percentage_with_spaces() {
        let w: Wrapper =
            toml::from_str("size = \" 2.0 % \"").expect("spaced percentage should deserialize");
        assert_eq!(w.size, SizeValue::Percent(2.0));
    }

    #[test]
    fn a_plain_string_number_is_pixels() {
        let w: Wrapper =
            toml::from_str("size = \"10\"").expect("string number should deserialize");
        assert_eq!(w.size, SizeValue::Pixels(10.0));
    }

    #[test]
    fn zero_percent_is_rejected() {
        let err = toml::from_str::<Wrapper>("size = \"0%\"").expect_err("0% should error");
        assert!(err.to_string().contains("greater than 0"));
    }

    #[test]
    fn over_hundred_percent_is_rejected() {
        let err = toml::from_str::<Wrapper>("size = \"101%\"").expect_err("101% should error");
        assert!(err.to_string().contains("cannot exceed 100"));
    }

    #[test]
    fn invalid_number_is_rejected() {
        let err =
            toml::from_str::<Wrapper>("size = \"abc\"").expect_err("non-numeric should error");
        assert!(err.to_string().contains("invalid size value"));
    }

    #[test]
    fn invalid_percentage_is_rejected() {
        let err = toml::from_str::<Wrapper>("size = \"abc%\"")
            .expect_err("non-numeric percentage should error");
        assert!(err.to_string().contains("invalid percentage value"));
    }

    #[test]
    fn resolve_pixels_returns_unchanged() {
        assert_eq!(SizeValue::Pixels(38.0).resolve(2160.0), 38.0);
    }

    #[test]
    fn resolve_percent_computes_screen_fraction() {
        let value = SizeValue::Percent(1.5).resolve(2160.0);
        assert_eq!(value, 32.4);
    }

    #[test]
    fn resolve_percent_on_1080p() {
        let value = SizeValue::Percent(3.5).resolve(1080.0);
        assert_eq!(value, 37.8);
    }
}
