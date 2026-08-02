//! The serde defaults and range-checking deserializers backing the
//! appearance configuration.

use serde::{Deserialize, Deserializer, de::Error as _};

/// Automatic magnification is on unless the configuration turns it off.
pub(super) const fn default_auto_scale() -> bool {
    true
}

pub(super) const fn default_follow_hyde() -> bool {
    true
}

pub(super) fn scale_factor_deserializer<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>
{
    let value = f64::deserialize(deserializer)?;

    if value <= 0.0 {
        return Err(D::Error::custom("Scale factor must be greater than 0.0"));
    }

    if value > 2.0 {
        return Err(D::Error::custom("Scale factor cannot be greater than 2.0"));
    }

    Ok(value)
}

pub(super) const fn default_greeting() -> bool {
    true
}

pub(super) const fn default_bar_opacity() -> f32 {
    0.0
}

pub(super) const fn default_scale_factor() -> f64 {
    1.0
}

pub(super) fn opacity_deserializer<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>
{
    let value = f32::deserialize(deserializer)?;

    if value < 0.0 {
        return Err(D::Error::custom("Opacity cannot be negative"));
    }

    if value > 1.0 {
        return Err(D::Error::custom("Opacity cannot be greater than 1.0"));
    }

    Ok(value)
}

pub(super) const fn default_opacity() -> f32 {
    1.0
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]
    use serde::de::value::{Error as DeError, F32Deserializer, F64Deserializer};

    use super::*;

    #[test]
    fn scale_factor_deserializer_rejects_out_of_bounds_values() {
        let err_small: DeError = scale_factor_deserializer(F64Deserializer::<DeError>::new(0.0))
            .expect_err("scale factor <= 0 should error");
        assert!(err_small.to_string().contains("greater than 0.0"));

        let err_large: DeError = scale_factor_deserializer(F64Deserializer::<DeError>::new(2.1))
            .expect_err("scale factor > 2 should error");
        assert!(err_large.to_string().contains("greater than 2.0"));
    }

    #[test]
    fn opacity_deserializer_rejects_invalid_values() {
        let err_negative: DeError = opacity_deserializer(F32Deserializer::<DeError>::new(-0.1))
            .expect_err("negative opacity should error");
        assert!(err_negative.to_string().contains("cannot be negative"));

        let err_large: DeError = opacity_deserializer(F32Deserializer::<DeError>::new(1.1))
            .expect_err("opacity > 1 should error");
        assert!(err_large.to_string().contains("greater than 1.0"));
    }
}
