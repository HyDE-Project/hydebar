use std::collections::HashSet;

use super::{Config, ModuleDef, ModuleName};

/// Errors returned when validating a [`Config`].
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

/// Refuses a value outside `low..=high`, naming the field and the range.
fn in_range(
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

impl std::error::Error for ConfigValidationError {}

impl Config {
    /// Validates the configuration, ensuring module definitions are consistent.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigValidationError`] if duplicate custom modules are
    /// defined or if the module layout references undefined custom modules.
    ///
    /// # Examples
    ///
    /// ```
    /// use hydebar_proto::config::Config;
    ///
    /// let config = Config::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        self.validate_appearance()?;

        let mut seen_custom_modules = HashSet::new();

        for module in &self.custom_modules {
            if !seen_custom_modules.insert(module.name.clone()) {
                return Err(ConfigValidationError::DuplicateCustomModule {
                    name: module.name.clone()
                });
            }
        }

        let ensure_custom_module_exists = |name: &str| {
            if !seen_custom_modules.contains(name) {
                return Err(ConfigValidationError::MissingCustomModule {
                    name: name.to_owned()
                });
            }

            Ok(())
        };

        for module_def in self
            .modules
            .left
            .iter()
            .chain(self.modules.center.iter())
            .chain(self.modules.right.iter())
        {
            match module_def {
                ModuleDef::Single(ModuleName::Custom(name)) => {
                    ensure_custom_module_exists(name)?;
                }
                ModuleDef::Group(group) => {
                    for module in group {
                        if let ModuleName::Custom(name) = module {
                            ensure_custom_module_exists(name)?;
                        }
                    }
                }
                ModuleDef::Single(_) => {}
            }
        }

        Ok(())
    }

    /// Refuses appearance numbers the bar cannot draw.
    fn validate_appearance(&self) -> Result<(), ConfigValidationError> {
        let appearance = &self.appearance;

        if let Some(font_size) = appearance.font_size {
            in_range(
                "appearance.font_size",
                f64::from(font_size),
                4.0,
                128.0,
                "4 to 128"
            )?;
        }

        if let Some(height) = appearance.height {
            in_range(
                "appearance.height",
                f64::from(height),
                8.0,
                512.0,
                "8 to 512"
            )?;
        }

        in_range(
            "appearance.scale_factor",
            appearance.scale_factor,
            0.1,
            10.0,
            "0.1 to 10"
        )?;
        in_range(
            "appearance.opacity",
            f64::from(appearance.opacity),
            0.0,
            1.0,
            "0 to 1"
        )?;
        in_range(
            "appearance.menu.opacity",
            f64::from(appearance.menu.opacity),
            0.0,
            1.0,
            "0 to 1"
        )?;
        in_range(
            "appearance.menu.backdrop",
            f64::from(appearance.menu.backdrop),
            0.0,
            1.0,
            "0 to 1"
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::CustomModuleDef, *};
    use crate::config::Modules;

    fn custom_module(name: &str) -> CustomModuleDef {
        CustomModuleDef {
            name: name.to_owned(),
            command: String::from("true"),
            ..CustomModuleDef::default()
        }
    }

    #[test]
    fn validate_accepts_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_duplicate_custom_modules() {
        let config = Config {
            custom_modules: vec![custom_module("foo"), custom_module("foo")],
            ..Default::default()
        };

        let error = config
            .validate()
            .expect_err("expected duplicate module error");
        assert!(matches!(
            error,
            ConfigValidationError::DuplicateCustomModule { ref name } if name == "foo"
        ));
    }

    #[test]
    fn validate_rejects_missing_custom_module_reference() {
        let config = Config {
            custom_modules: vec![custom_module("foo")],
            modules: Modules {
                left: vec![ModuleDef::Single(ModuleName::Custom("bar".to_owned()))],
                ..Default::default()
            },
            ..Default::default()
        };

        let error = config
            .validate()
            .expect_err("expected missing module error");
        assert!(matches!(
            error,
            ConfigValidationError::MissingCustomModule { ref name } if name == "bar"
        ));
    }

    #[test]
    fn a_defaulted_appearance_passes_the_range_checks() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn a_twelvefold_opacity_is_named_and_refused() {
        let mut config = Config::default();
        config.appearance.opacity = 12.0;

        let error = config.validate().expect_err("opacity must be refused");
        assert!(matches!(
            error,
            ConfigValidationError::ValueOutOfRange {
                field: "appearance.opacity",
                ..
            }
        ));
    }

    #[test]
    fn a_zero_scale_factor_is_refused() {
        let mut config = Config::default();
        config.appearance.scale_factor = 0.0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn a_nan_menu_backdrop_is_refused() {
        let mut config = Config::default();
        config.appearance.menu.backdrop = f32::NAN;

        assert!(config.validate().is_err());
    }

    #[test]
    fn a_negative_font_size_is_refused() {
        let mut config = Config::default();
        config.appearance.font_size = Some(-3.0);

        assert!(config.validate().is_err());
    }
}
