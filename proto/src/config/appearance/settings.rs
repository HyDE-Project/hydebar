//! Top-level appearance configuration.

use serde::Deserialize;

use super::{
    animation::AnimationConfig,
    color::{
        AppearanceColor, default_background_color, default_danger_color, default_primary_color,
        default_secondary_color, default_success_color, default_text_color, default_warning_color,
        default_workspace_colors
    },
    defaults::{
        default_auto_scale, default_bar_opacity, default_follow_hyde, default_greeting,
        default_opacity, default_scale_factor, opacity_deserializer, scale_factor_deserializer
    },
    menu::MenuAppearance,
    style::AppearanceStyle,
    window::{WindowBorder, WindowShadow}
};

/// Top-level appearance configuration.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent configuration switch"
)]
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Appearance {
    #[serde(default)]
    /// Family every widget is written in, taken from the desktop when unset.
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
    /// Multiplier applied to every size the bar draws at.
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
    /// bar, for example `38.0` for the height the `HyDE` waybar theme reserves.
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
    /// Whether the appearance follows the theme published by the `HyDE`
    /// Project.
    ///
    /// Enabled by default: every field the user did not set explicitly is taken
    /// from the `HyDE` theme, so the bar changes along with the rest of the
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
    /// Whether the modules stand as islands, as one strip, or solid.
    pub style:                    AppearanceStyle,
    #[serde(deserialize_with = "opacity_deserializer", default = "default_opacity")]
    /// Opacity of an island, behind the widgets it holds.
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
    /// How an open menu is drawn, and how far it stands from the bar.
    pub menu:                     MenuAppearance,
    #[serde(default)]
    /// How long the bar's motion takes, and how it eases.
    pub animations:               AnimationConfig,
    /// Whether the bar greets the user while it is born.
    #[serde(default = "default_greeting")]
    pub greeting:                 bool,
    #[serde(default = "default_background_color")]
    /// Colour of the surface a module is drawn on.
    pub background_color:         AppearanceColor,
    #[serde(default = "default_primary_color")]
    /// Colour marking what holds focus or is in force.
    pub primary_color:            AppearanceColor,
    #[serde(default = "default_secondary_color")]
    /// Colour of the surfaces standing behind the primary one.
    pub secondary_color:          AppearanceColor,
    #[serde(default = "default_success_color")]
    /// Colour of a reading that is in good order.
    pub success_color:            AppearanceColor,
    #[serde(default = "default_danger_color")]
    /// Colour of a reading that calls for attention now.
    pub danger_color:             AppearanceColor,
    #[serde(default = "default_warning_color")]
    /// Colour of a reading on its way to trouble.
    pub warning_color:            AppearanceColor,
    #[serde(default = "default_text_color")]
    /// Colour every glyph is written in.
    pub text_color:               AppearanceColor,
    #[serde(default = "default_workspace_colors")]
    /// Colours the workspaces are drawn in, taken in turn.
    pub workspace_colors:         Vec<AppearanceColor>,
    /// Colours the special workspaces are drawn in, taken in turn.
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
    /// Whether the islands adopt the compositor's window border.
    ///
    /// Off by default: a border on every island is a strong look, and the
    /// bar should only wear it when asked.
    #[serde(default)]
    pub island_borders:           bool,
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
            island_borders:           false,
            window_border:            None,
            window_shadow:            None
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    #[test]
    fn default_appearance_has_expected_colors() {
        let appearance = Appearance::default();
        assert_eq!(appearance.opacity, 1.0);
        assert_eq!(appearance.workspace_colors.len(), 3);
        assert!(appearance.text_color.get_text().is_none());
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
