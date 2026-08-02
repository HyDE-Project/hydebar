//! The named refusals configuration validation can answer with.

/// Errors returned when validating a [`Config`].
///
/// [`Config`]: crate::config::Config
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValidationError {
    /// Duplicate custom module definitions were found.
    DuplicateCustomModule { name: String },

    /// A module references a custom module definition that does not exist.
    MissingCustomModule { name: String },

    /// A numeric setting stands outside the range the bar can draw.
    ///
    /// Caught here rather than clamped silently: a zero scale or a
    /// twelvefold opacity is a typo, and a typo deserves a named refusal
    /// with the allowed range, not a bar that quietly looks wrong.
    ValueOutOfRange {
        /// The setting as the file spells it.
        field:   &'static str,
        /// The range the bar accepts, spelled for the message.
        allowed: &'static str,
        /// What the file said.
        got:     f64
    }
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateCustomModule {
                name
            } => {
                write!(f, "duplicate custom module definition for '{name}'")
            }
            Self::MissingCustomModule {
                name
            } => {
                write!(
                    f,
                    "custom module '{name}' referenced in layout but not defined"
                )
            }
            Self::ValueOutOfRange {
                field,
                allowed,
                got
            } => {
                write!(f, "'{field}' is {got}, allowed range is {allowed}")
            }
        }
    }
}

impl std::error::Error for ConfigValidationError {}

/// Refuses a value outside `low..=high`, naming the field and the range.
pub(super) fn in_range(
    field: &'static str,
    value: f64,
    low: f64,
    high: f64,
    allowed: &'static str
) -> Result<(), ConfigValidationError> {
    if value.is_finite() && (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(ConfigValidationError::ValueOutOfRange {
            field,
            allowed,
            got: value
        })
    }
}
