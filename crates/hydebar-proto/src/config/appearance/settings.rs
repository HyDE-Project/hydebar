//! Top-level appearance configuration and the serde defaults backing it.

use serde::{Deserialize, Deserializer, de::Error as _};

use super::{
    animation::AnimationConfig,
    color::{
        AppearanceColor, default_background_color, default_danger_color, default_primary_color,
        default_secondary_color, default_success_color, default_text_color, default_warning_color,
        default_workspace_colors
    },
    menu::MenuAppearance,
    style::AppearanceStyle
};

/// Top-level appearance configuration.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Appearance {
    #[serde(default)]
    pub font_name:                Option<String>,
    /// Default text size, in pixels, applied to every widget that does not set
    /// its own.
    ///
    /// Left unset the renderer default is used; set it to match another bar,
    /// for example `10.0` for waybar's default font size.
    #[serde(default)]
    pub font_size:                Option<f32>,
    #[serde(
        deserialize_with = "scale_factor_deserializer",
        default = "default_scale_factor"
    )]
    pub scale_factor:             f64,
    /// Corner radius of the island pills, in pixels.
    ///
    /// Left unset the bar falls back to [`DEFAULT_RADIUS`], the radius of the
    /// reference waybar theme.
    ///
    /// [`DEFAULT_RADIUS`]: super::DEFAULT_RADIUS
    #[serde(default)]
    pub radius:                   Option<f32>,
    /// Height of the bar, in pixels.
    ///
    /// Left unset the bar keeps its built-in height. Set it to match another
    /// bar, for example `38.0` for the height the HyDE waybar theme reserves.
    #[serde(default)]
    pub height:                   Option<f32>,
    /// Padding kept between the screen edge and the outermost island, in
    /// pixels.
    ///
    /// Left unset the bar takes the gap the compositor keeps outside its
    /// windows, so the leftmost module starts on the same column a window does
    /// and the rightmost one ends on the same column. Set it to pin the padding
    /// to a value of its own, whatever the compositor is configured with.
    #[serde(default)]
    pub side_padding:             Option<f32>,
    /// Whether the appearance follows the theme published by the HyDE Project.
    ///
    /// Enabled by default: every field the user did not set explicitly is taken
    /// from the HyDE theme, so the bar changes along with the rest of the
    /// desktop. Fields present in the configuration always win.
    #[serde(default = "default_follow_hyde")]
    pub follow_hyde:              bool,
    /// Whether the bar magnifies itself to stay readable on the screen it
    /// lands on.
    ///
    /// Enabled by default: the bar measures the output, both its pixels and its
    /// physical size, and magnifies itself so a laptop panel, a dense desktop
    /// screen and a television across the room all stay readable without a
    /// single setting. Set it to `false` to keep the configured sizes verbatim.
    #[serde(default = "default_auto_scale")]
    pub auto_scale:               bool,
    #[serde(default)]
    pub style:                    AppearanceStyle,
    #[serde(deserialize_with = "opacity_deserializer", default = "default_opacity")]
    pub opacity:                  f32,
    /// Opacity of the strip behind the whole bar.
    ///
    /// Island styles leave it fully transparent by default. A compositor skips
    /// blurring fully transparent pixels, so a bar meant to be blurred edge to
    /// edge needs a faint value here.
    #[serde(
        deserialize_with = "opacity_deserializer",
        default = "default_bar_opacity"
    )]
    pub bar_opacity:              f32,
    #[serde(default)]
    pub menu:                     MenuAppearance,
    #[serde(default)]
    pub animations:               AnimationConfig,
    /// Whether the bar greets the user while it is born.
    #[serde(default = "default_greeting")]
    pub greeting:                 bool,
    #[serde(default = "default_background_color")]
    pub background_color:         AppearanceColor,
    #[serde(default = "default_primary_color")]
    pub primary_color:            AppearanceColor,
    #[serde(default = "default_secondary_color")]
    pub secondary_color:          AppearanceColor,
    #[serde(default = "default_success_color")]
    pub success_color:            AppearanceColor,
    #[serde(default = "default_danger_color")]
    pub danger_color:             AppearanceColor,
    #[serde(default = "default_warning_color")]
    pub warning_color:            AppearanceColor,
    #[serde(default = "default_text_color")]
    pub text_color:               AppearanceColor,
    #[serde(default = "default_workspace_colors")]
    pub workspace_colors:         Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
    /// Border the islands draw, adopted from the compositor's windows.
    #[serde(skip)]
    pub window_border:            Option<WindowBorder>,
    /// Shadow the islands cast, adopted when the compositor casts one.
    #[serde(skip)]
    pub window_shadow:            Option<WindowShadow>
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font_name:                None,
            font_size:                None,
            radius:                   None,
            height:                   None,
            side_padding:             None,
            follow_hyde:              default_follow_hyde(),
            auto_scale:               default_auto_scale(),
            scale_factor:             1.0,
            style:                    AppearanceStyle::default(),
            opacity:                  default_opacity(),
            bar_opacity:              default_bar_opacity(),
            menu:                     MenuAppearance::default(),
            animations:               AnimationConfig::default(),
            greeting:                 default_greeting(),
            background_color:         default_background_color(),
            primary_color:            default_primary_color(),
            secondary_color:          default_secondary_color(),
            success_color:            default_success_color(),
            danger_color:             default_danger_color(),
            warning_color:            default_warning_color(),
            text_color:               default_text_color(),
            workspace_colors:         default_workspace_colors(),
            special_workspace_colors: None,
            window_border: None,
            window_shadow: None
        }
    }
}

/// Automatic magnification is on unless the configuration turns it off.
fn default_auto_scale() -> bool {
    true
}

fn default_follow_hyde() -> bool {
    true
}

fn scale_factor_deserializer<'de, D>(deserializer: D) -> Result<f64, D::Error>
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

pub(super) fn default_greeting() -> bool {
    true
}

pub(super) fn default_bar_opacity() -> f32 {
    0.0
}

fn default_scale_factor() -> f64 {
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

pub(super) fn default_opacity() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use serde::de::value::{Error as DeError, F32Deserializer, F64Deserializer};

    use super::*;

    #[test]
    fn default_appearance_has_expected_colors() {
        let appearance = Appearance::default();
        assert_eq!(appearance.opacity, 1.0);
        assert_eq!(appearance.workspace_colors.len(), 3);
        assert!(appearance.text_color.get_text().is_none());
    }

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

    #[test]
    fn font_size_defaults_to_none_and_deserializes() {
        assert!(Appearance::default().font_size.is_none());

        let without_font_size: Appearance =
            toml::from_str("").expect("empty appearance table should deserialize");
        assert!(without_font_size.font_size.is_none());

        let with_font_size: Appearance =
            toml::from_str("font_size = 10.0").expect("font_size should deserialize");
        assert_eq!(with_font_size.font_size, Some(10.0));
    }

    #[test]
    fn height_defaults_to_none_and_deserializes() {
        assert!(Appearance::default().height.is_none());

        let without_height: Appearance =
            toml::from_str("").expect("empty appearance table should deserialize");
        assert!(without_height.height.is_none());

        let with_height: Appearance =
            toml::from_str("height = 38.0").expect("height should deserialize");
        assert_eq!(with_height.height, Some(38.0));
    }

    #[test]
    fn side_padding_defaults_to_none_and_deserializes() {
        assert!(Appearance::default().side_padding.is_none());

        let without_padding: Appearance =
            toml::from_str("").expect("empty appearance table should deserialize");
        assert!(without_padding.side_padding.is_none());

        let with_padding: Appearance =
            toml::from_str("side_padding = 8.0").expect("side_padding should deserialize");
        assert_eq!(with_padding.side_padding, Some(8.0));
    }

    #[test]
    fn follow_hyde_is_enabled_unless_disabled_explicitly() {
        assert!(Appearance::default().follow_hyde);

        let implicit: Appearance = toml::from_str("").expect("empty appearance table");
        assert!(implicit.follow_hyde);

        let disabled: Appearance =
            toml::from_str("follow_hyde = false").expect("follow_hyde should deserialize");
        assert!(!disabled.follow_hyde);
        assert!(disabled.radius.is_none());
    }

    #[test]
    fn appearance_default_includes_animations() {
        let appearance = Appearance::default();
        assert!(appearance.animations.enabled);
        assert_eq!(appearance.animations.menu_fade_duration_ms, 200);
    }
}

/// Border of a compositor window, as the islands adopt it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowBorder {
    /// Width in the bar's layout pixels.
    pub width: f32,
    /// RGBA in unit range, the leading stop of the compositor gradient.
    pub color: [f32; 4]
}

/// Shadow of a compositor window, as the islands adopt it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowShadow {
    /// Reach of the shadow in the bar's layout pixels.
    pub range: f32,
    /// RGBA in unit range.
    pub color: [f32; 4]
}
