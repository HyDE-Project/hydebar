//! Bar module configuring the bar itself.
//!
//! Every choice made here is written straight back into the configuration file
//! the bar was started from. The file watcher picks the change up and reloads,
//! so the menu never holds state of its own: what it draws is always what the
//! running configuration says.

mod layout;
mod tab;
mod view;
mod writer;

use std::path::{Path, PathBuf};

use hydebar_proto::config::{Config, ModuleDef, Modules, NotificationSource};
use iced::{Element, Task};
pub use layout::{LayoutEdit, Section, Slot};
use log::warn;
pub use tab::Tab;
pub use writer::{SettingValue, SettingsWriteError, write_setting};

use super::{Module, OnModulePress};
use crate::{
    components::icons::{IconTheme, Icons, icon},
    config::{AppearanceStyle, BarLayer, Position},
    menu::MenuType,
    services::hyprland_notify::{Notice, compositor_color, notify, post_to_bus}
};

/// How long the notice announcing a new notification source stays up, in
/// milliseconds.
const ANNOUNCE_DURATION: u32 = 4000;

/// Smallest bar height the menu will step down to, in pixels.
const MIN_HEIGHT: f32 = 16.0;
/// Largest bar height the menu will step up to, in pixels.
const MAX_HEIGHT: f32 = 96.0;
/// Height added or removed by one press, in pixels.
const HEIGHT_STEP: f32 = 2.0;

/// Smallest side padding the menu will step down to, in pixels.
///
/// Zero is a deliberate choice rather than a floor: a bar told to sit flush
/// with the screen edge is what a compositor without window gaps calls for.
const MIN_SIDE_PADDING: f32 = 0.0;
/// Largest side padding the menu will step up to, in pixels.
const MAX_SIDE_PADDING: f32 = 96.0;
/// Side padding added or removed by one press, in pixels.
const SIDE_PADDING_STEP: f32 = 1.0;

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
            &compositor_color(config.appearance.primary_color.clone()),
            config.appearance.font_size_px(),
            &message
        );

        return;
    }

    post_to_bus(&message);
}

impl Message {
    /// Dotted path of the configuration key this choice writes.
    fn path(&self) -> &'static [&'static str] {
        match self {
            Self::SetPosition(_) => &["position"],
            Self::SetLayer(_) => &["layer"],
            Self::SetStyle(_) => &["appearance", "style"],
            Self::SetHeight(_) => &["appearance", "height"],
            Self::SetSidePadding(_) => &["appearance", "side_padding"],
            Self::SetFontSize(_) => &["appearance", "font_size"],
            Self::SetOpacity(_) => &["appearance", "opacity"],
            Self::SetNotificationSource(_) => &["notifications", "source"],
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
    fn apply(&self, config_path: &Path) {
        let path = self.path();

        if path.is_empty() {
            return;
        }

        if let Err(err) = write_setting(config_path, path, self.value()) {
            warn!("failed to store the setting: {err}");
        }
    }
}

/// Keeps the pick on the module the edit acted on.
///
/// A module that moved would otherwise leave the pick pointing at whatever
/// took its place, and the next button press would act on the wrong module.
fn follow(edit: LayoutEdit, modules: &Modules) -> Option<Slot> {
    let slot = edit.slot()?;

    match edit {
        LayoutEdit::Remove(_) => None,
        LayoutEdit::MoveEarlier(_) => Some(Slot {
            section: slot.section,
            index:   slot.index.saturating_sub(1)
        }),
        LayoutEdit::MoveLater(_) => Some(Slot {
            section: slot.section,
            index:   slot.index + 1
        }),
        LayoutEdit::MoveToPreviousSection(_) => slot.section.before().map(|section| Slot {
            section,
            index: section.entries(modules).len().saturating_sub(1)
        }),
        LayoutEdit::MoveToNextSection(_) => slot.section.after().map(|section| Slot {
            section,
            index: 0
        }),
        _ => Some(slot)
    }
}

/// Renders a bar entry as the value the configuration stores.
fn entry_value(entry: &ModuleDef) -> SettingValue {
    match entry {
        ModuleDef::Single(name) => SettingValue::Text(name.as_str().to_owned()),
        ModuleDef::Group(group) => SettingValue::List(
            group
                .iter()
                .map(|name| SettingValue::Text(name.as_str().to_owned()))
                .collect()
        )
    }
}

/// Renders a section as the list the configuration stores.
fn section_value(entries: &[ModuleDef]) -> SettingValue {
    SettingValue::List(entries.iter().map(entry_value).collect())
}

/// Writes every section of `modules` into the configuration file.
///
/// In one write on purpose: the file is watched, and three writes in a row
/// would reload the whole bar up to three times for one edit.
fn store_layout(config_path: &Path, modules: &Modules) {
    let settings = vec![
        (["modules", "left"].as_slice(), section_value(&modules.left)),
        (
            ["modules", "center"].as_slice(),
            section_value(&modules.center)
        ),
        (
            ["modules", "right"].as_slice(),
            section_value(&modules.right)
        ),
    ];

    if let Err(err) = writer::write_settings(config_path, settings) {
        warn!("failed to store the module layout: {err}");
    }
}

/// Bar entry opening the settings of the bar.
#[derive(Default, Debug, Clone)]
pub struct Settings {
    /// File the choices are written to.
    config_path: PathBuf,
    /// Page the window currently shows.
    tab:         Tab,
    /// Module the editor acts on, once one is picked.
    selected:    Option<Slot>,
    /// Section the editor is showing.
    section:     Section
}

impl Settings {
    /// Creates the module writing to `config_path`.
    #[must_use]
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            tab: Tab::default(),
            selected: None,
            section: Section::Left
        }
    }

    /// Section the editor is showing.
    #[must_use]
    pub fn section(&self) -> Section {
        self.section
    }

    /// Module the editor acts on, once one is picked.
    #[must_use]
    pub fn selected(&self) -> Option<Slot> {
        self.selected
    }

    /// Page the window currently shows.
    #[must_use]
    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// Applies a choice made in the window.
    ///
    /// Picking a tab is the only choice kept in memory; everything else lands
    /// in the configuration file and comes back through the reload.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::SelectTab(tab) => self.tab = tab,
            Message::SelectSlot(slot) => self.selected = slot,
            Message::SelectSection(section) => {
                self.section = section;
                self.selected = None;
            }
            Message::EditLayout(edit) => {
                let modules = layout::apply(&config.modules, &edit);
                store_layout(&self.config_path, &modules);
                self.selected = follow(edit, &modules);

                if let Some(slot) = self.selected {
                    self.section = slot.section;
                }
            }
            other => other.apply(&self.config_path)
        }

        Task::none()
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

    /// Side padding one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn side_padding_below(current: f32) -> f32 {
        (current - SIDE_PADDING_STEP).clamp(MIN_SIDE_PADDING, MAX_SIDE_PADDING)
    }

    /// Side padding one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn side_padding_above(current: f32) -> f32 {
        (current + SIDE_PADDING_STEP).clamp(MIN_SIDE_PADDING, MAX_SIDE_PADDING)
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

    #[test]
    fn the_height_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::height_above(38.0), 40.0);
        assert_eq!(Settings::height_below(38.0), 36.0);
        assert_eq!(Settings::height_below(MIN_HEIGHT), MIN_HEIGHT);
        assert_eq!(Settings::height_above(MAX_HEIGHT), MAX_HEIGHT);
    }

    #[test]
    fn the_side_padding_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::side_padding_above(8.0), 9.0);
        assert_eq!(Settings::side_padding_below(8.0), 7.0);
        assert_eq!(
            Settings::side_padding_below(MIN_SIDE_PADDING),
            MIN_SIDE_PADDING
        );
        assert_eq!(
            Settings::side_padding_above(MAX_SIDE_PADDING),
            MAX_SIDE_PADDING
        );
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
