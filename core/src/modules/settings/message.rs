//! Choices made in the settings menu and the keys they write.
//!
//! Every choice knows the dotted path of the configuration key it
//! stands for and the value the reader will find there, so writing a
//! choice back is one lookup rather than a table kept elsewhere.

use std::path::Path;

use hydebar_proto::config::{Config, HydeBranch, NotificationSource};
use log::warn;

use super::{LayoutEdit, Section, SettingValue, Slot, Tab, write_setting};
use crate::{
    config::{AppearanceStyle, BarLayer, Position},
    services::hyprland_notify::{Notice, compositor_color, notify, post_to_bus}
};

/// How long the notice announcing a new notification source stays up, in
/// milliseconds.
const ANNOUNCE_DURATION: u32 = 4000;

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
    /// Set the padding between the screen edge and the outermost island, in
    /// pixels.
    ///
    /// Writing it pins the padding: the bar stops taking the gap the
    /// compositor keeps around its windows.
    SetSidePadding(f32),
    /// Set the default text size, in pixels.
    SetFontSize(f32),
    /// Set the opacity of the module pills.
    SetOpacity(f32),
    /// Choose who draws the notification popups.
    SetNotificationSource(NotificationSource),
    /// Follow the given branch of the `HyDE` clone.
    SetHydeBranch(HydeBranch),
    /// Show another page of the window.
    SelectTab(Tab),
    /// Rearrange the modules of the bar.
    EditLayout(LayoutEdit),
    /// Pick the module the editor acts on, or drop the pick.
    SelectSlot(Option<Slot>),
    /// Show the modules of another section.
    SelectSection(Section)
}

/// Announces the notification source the user just picked, through that
/// very source.
///
/// A setting whose effect is invisible until something else happens is a
/// setting nobody can tell they changed. Sending one notice the moment the
/// choice is made answers the only question the choice raises — where will
/// my notifications appear now — by showing it.
pub fn announce_source(source: NotificationSource, config: &Config) {
    let message = format!("notifications are now shown by {}", source.label());

    if source.hands_to_compositor() {
        notify(
            Notice::Info,
            ANNOUNCE_DURATION,
            &compositor_color(config.appearance.primary_color),
            config.appearance.font_size_px(),
            &message
        );

        return;
    }

    post_to_bus(&message);
}

impl Message {
    /// Dotted path of the configuration key this choice writes.
    const fn path(&self) -> &'static [&'static str] {
        match self {
            Self::SetPosition(_) => &["position"],
            Self::SetLayer(_) => &["layer"],
            Self::SetStyle(_) => &["appearance", "style"],
            Self::SetHeight(_) => &["appearance", "height"],
            Self::SetSidePadding(_) => &["appearance", "side_padding"],
            Self::SetFontSize(_) => &["appearance", "font_size"],
            Self::SetOpacity(_) => &["appearance", "opacity"],
            Self::SetNotificationSource(_) => &["notifications", "source"],
            Self::SetHydeBranch(_) => &["updates", "hyde_branch"],
            Self::SelectTab(_)
            | Self::EditLayout(_)
            | Self::SelectSlot(_)
            | Self::SelectSection(_) => &[]
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
            Self::SetSidePadding(padding) => (*padding).into(),
            Self::SetFontSize(size) => (*size).into(),
            Self::SetOpacity(opacity) => (*opacity).into(),
            Self::SetNotificationSource(source) => match source {
                NotificationSource::Builtin => "Builtin".into(),
                NotificationSource::Compositor => "Compositor".into(),
                NotificationSource::Daemon => "Daemon".into()
            },
            Self::SetHydeBranch(branch) => match branch {
                HydeBranch::Master => "Master".into(),
                HydeBranch::Dev => "Dev".into()
            },
            Self::SelectTab(_)
            | Self::EditLayout(_)
            | Self::SelectSlot(_)
            | Self::SelectSection(_) => SettingValue::Flag(false)
        }
    }

    /// Writes this choice into the configuration file at `config_path`.
    ///
    /// Failures are logged rather than propagated: a settings menu that cannot
    /// persist a choice should still leave the bar running.
    pub(super) fn apply(&self, config_path: &Path) {
        let path = self.path();

        if path.is_empty() {
            return;
        }

        if let Err(err) = write_setting(config_path, path, self.value()) {
            warn!("failed to store the setting: {err}");
        }
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
            Message::SetSidePadding(8.0).path(),
            &["appearance", "side_padding"]
        );
        assert_eq!(
            Message::SetFontSize(10.0).path(),
            &["appearance", "font_size"]
        );
        assert_eq!(Message::SetOpacity(0.8).path(), &["appearance", "opacity"]);
        assert_eq!(
            Message::SetNotificationSource(NotificationSource::Daemon).path(),
            &["notifications", "source"]
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
            Message::SetNotificationSource(NotificationSource::Builtin).value(),
            SettingValue::Text("Builtin".to_owned())
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
}
