//! Bar module configuring the bar itself.
//!
//! Every choice made here is written straight back into the configuration file
//! the bar was started from. The file watcher picks the change up and reloads,
//! so the menu never holds state of its own: what it draws is always what the
//! running configuration says.

mod menu;
mod writer;

use std::path::{Path, PathBuf};

use iced::Element;
use log::warn;
pub use writer::{SettingValue, SettingsWriteError, write_setting};

use super::{Module, OnModulePress};
use crate::{
    components::icons::{IconTheme, Icons, icon},
    config::{AppearanceStyle, BarLayer, Position},
    menu::MenuType
};

/// Smallest bar height the menu will step down to, in pixels.
const MIN_HEIGHT: f32 = 16.0;
/// Largest bar height the menu will step up to, in pixels.
const MAX_HEIGHT: f32 = 96.0;
/// Height added or removed by one press, in pixels.
const HEIGHT_STEP: f32 = 2.0;

/// Smallest font size the menu will step down to, in pixels.
const MIN_FONT_SIZE: f32 = 6.0;
/// Largest font size the menu will step up to, in pixels.
const MAX_FONT_SIZE: f32 = 32.0;
/// Font size added or removed by one press, in pixels.
const FONT_SIZE_STEP: f32 = 1.0;

/// Opacity added or removed by one press.
const OPACITY_STEP: f32 = 0.05;

/// Choice made in the settings menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Move the bar to the given screen edge.
    SetPosition(Position),
    /// Place the bar on the given compositor layer.
    SetLayer(BarLayer),
    /// Draw the bar in the given style.
    SetStyle(AppearanceStyle),
    /// Set the bar height, in pixels.
    SetHeight(f32),
    /// Set the default text size, in pixels.
    SetFontSize(f32),
    /// Set the opacity of the module pills.
    SetOpacity(f32),
    /// Follow the theme published by the HyDE Project, or stop following it.
    SetFollowHyde(bool)
}

impl Message {
    /// Dotted path of the configuration key this choice writes.
    fn path(&self) -> &'static [&'static str] {
        match self {
            Self::SetPosition(_) => &["position"],
            Self::SetLayer(_) => &["layer"],
            Self::SetStyle(_) => &["appearance", "style"],
            Self::SetHeight(_) => &["appearance", "height"],
            Self::SetFontSize(_) => &["appearance", "font_size"],
            Self::SetOpacity(_) => &["appearance", "opacity"],
            Self::SetFollowHyde(_) => &["appearance", "follow_hyde"]
        }
    }

    /// Value this choice writes at [`Self::path`].
    fn value(&self) -> SettingValue {
        match self {
            Self::SetPosition(position) => match position {
                Position::Top => "Top".into(),
                Position::Bottom => "Bottom".into()
            },
            Self::SetLayer(layer) => match layer {
                BarLayer::Background => "Background".into(),
                BarLayer::Bottom => "Bottom".into(),
                BarLayer::Top => "Top".into(),
                BarLayer::Overlay => "Overlay".into()
            },
            Self::SetStyle(style) => match style {
                AppearanceStyle::Islands => "Islands".into(),
                AppearanceStyle::Solid => "Solid".into(),
                AppearanceStyle::Gradient => "Gradient".into()
            },
            Self::SetHeight(height) => (*height).into(),
            Self::SetFontSize(size) => (*size).into(),
            Self::SetOpacity(opacity) => (*opacity).into(),
            Self::SetFollowHyde(follow) => (*follow).into()
        }
    }

    /// Writes this choice into the configuration file at `config_path`.
    ///
    /// Failures are logged rather than propagated: a settings menu that cannot
    /// persist a choice should still leave the bar running.
    pub fn apply(&self, config_path: &Path) {
        if let Err(err) = write_setting(config_path, self.path(), self.value()) {
            warn!("failed to store the setting: {err}");
        }
    }
}

/// Bar entry opening the settings of the bar.
#[derive(Default, Debug, Clone)]
pub struct Settings {
    /// File the choices are written to.
    config_path: PathBuf
}

impl Settings {
    /// Creates the module writing to `config_path`.
    #[must_use]
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path
        }
    }

    /// File the choices are written to.
    #[must_use]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Height one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn height_below(current: f32) -> f32 {
        (current - HEIGHT_STEP).clamp(MIN_HEIGHT, MAX_HEIGHT)
    }

    /// Height one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn height_above(current: f32) -> f32 {
        (current + HEIGHT_STEP).clamp(MIN_HEIGHT, MAX_HEIGHT)
    }

    /// Font size one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn font_size_below(current: f32) -> f32 {
        (current - FONT_SIZE_STEP).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
    }

    /// Font size one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn font_size_above(current: f32) -> f32 {
        (current + FONT_SIZE_STEP).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
    }

    /// Opacity one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn opacity_below(current: f32) -> f32 {
        ((current - OPACITY_STEP) * 100.0).round() / 100.0
    }

    /// Opacity one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn opacity_above(current: f32) -> f32 {
        ((current + OPACITY_STEP) * 100.0).round() / 100.0
    }
}

impl<M> Module<M> for Settings
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        Some((
            icon(icons, Icons::Settings).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Settings))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_choice_names_the_key_it_writes() {
        assert_eq!(Message::SetPosition(Position::Bottom).path(), &["position"]);
        assert_eq!(Message::SetLayer(BarLayer::Top).path(), &["layer"]);
        assert_eq!(
            Message::SetStyle(AppearanceStyle::Solid).path(),
            &["appearance", "style"]
        );
        assert_eq!(Message::SetHeight(38.0).path(), &["appearance", "height"]);
        assert_eq!(
            Message::SetFontSize(10.0).path(),
            &["appearance", "font_size"]
        );
        assert_eq!(Message::SetOpacity(0.8).path(), &["appearance", "opacity"]);
        assert_eq!(
            Message::SetFollowHyde(true).path(),
            &["appearance", "follow_hyde"]
        );
    }

    #[test]
    fn named_variants_are_written_the_way_the_reader_spells_them() {
        assert_eq!(
            Message::SetPosition(Position::Bottom).value(),
            SettingValue::Text("Bottom".to_owned())
        );
        assert_eq!(
            Message::SetLayer(BarLayer::Overlay).value(),
            SettingValue::Text("Overlay".to_owned())
        );
        assert_eq!(
            Message::SetStyle(AppearanceStyle::Gradient).value(),
            SettingValue::Text("Gradient".to_owned())
        );
        assert_eq!(
            Message::SetFollowHyde(false).value(),
            SettingValue::Flag(false)
        );
    }

    #[test]
    fn a_written_variant_reads_back_as_the_same_value() {
        let position: Position = toml::from_str("v = \"Bottom\"\n")
            .map(|w: Wrapper<Position>| w.v)
            .expect("position");
        assert_eq!(position, Position::Bottom);

        let style: AppearanceStyle = toml::from_str("v = \"Gradient\"\n")
            .map(|w: Wrapper<AppearanceStyle>| w.v)
            .expect("style");
        assert_eq!(style, AppearanceStyle::Gradient);

        let layer: BarLayer = toml::from_str("v = \"Overlay\"\n")
            .map(|w: Wrapper<BarLayer>| w.v)
            .expect("layer");
        assert_eq!(layer, BarLayer::Overlay);
    }

    #[derive(serde::Deserialize)]
    struct Wrapper<T> {
        v: T
    }

    #[test]
    fn the_height_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::height_above(38.0), 40.0);
        assert_eq!(Settings::height_below(38.0), 36.0);
        assert_eq!(Settings::height_below(MIN_HEIGHT), MIN_HEIGHT);
        assert_eq!(Settings::height_above(MAX_HEIGHT), MAX_HEIGHT);
    }

    #[test]
    fn the_font_size_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::font_size_above(10.0), 11.0);
        assert_eq!(Settings::font_size_below(10.0), 9.0);
        assert_eq!(Settings::font_size_below(MIN_FONT_SIZE), MIN_FONT_SIZE);
        assert_eq!(Settings::font_size_above(MAX_FONT_SIZE), MAX_FONT_SIZE);
    }

    #[test]
    fn the_opacity_steps_keep_two_decimals() {
        assert_eq!(Settings::opacity_above(0.8), 0.85);
        assert_eq!(Settings::opacity_below(0.8), 0.75);
    }
}
