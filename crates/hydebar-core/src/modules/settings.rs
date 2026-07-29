//! Bar module configuring the bar itself.
//!
//! Every choice made here is written straight back into the configuration file
//! the bar was started from. The file watcher picks the change up and reloads,
//! so the menu never holds state of its own: what it draws is always what the
//! running configuration says.

mod hyde_shell;
mod layout;
mod progress;
mod tab;
mod theme_script;
mod view;
mod writer;

use std::{
    path::{Path, PathBuf},
    sync::Arc
};

use hydebar_proto::{
    config::{Config, ModuleDef, Modules, NotificationSource},
    hyde_state::{self, HydeState}
};
use iced::{Alignment, Element, Task, widget::Row};
pub use layout::{LayoutEdit, Section, Slot};
use log::{error, info, warn};
pub use progress::{FRAME_INTERVAL, Spinner};
pub use tab::Tab;
pub use writer::{SettingValue, SettingsWriteError, write_setting};

use super::{Module, OnModulePress};
use crate::{
    components::icons::{IconTheme, Icons, icon, icon_raw_sized},
    config::{AppearanceStyle, BarLayer, Position},
    menu::MenuType,
    services::hyprland_notify::{Notice, compositor_color, notify},
    utils::launcher
};

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

/// How long a notice about a refused desktop action stays on screen, in
/// milliseconds.
///
/// A theme switch is asked for deliberately, so the answer that it did not
/// happen has to outlast a glance away from the screen.
const NOTICE_DURATION: u32 = 6000;

/// Gap between the bar entry and the indicator of a running switch, in pixels.
///
/// Narrow on purpose: the two glyphs have to read as one entry that is busy
/// rather than as two entries that happen to sit next to each other.
const INDICATOR_GAP: f32 = 4.0;

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
    /// Follow the theme published by the HyDE Project, or stop following it.
    SetFollowHyde(bool),
    /// Take the text size and the bar height from the screen, or stop.
    SetAutoScale(bool),
    /// Choose who draws the notification popups.
    SetNotificationSource(NotificationSource),
    /// Show another page of the window.
    SelectTab(Tab),
    /// Rearrange the modules of the bar.
    EditLayout(LayoutEdit),
    /// Pick the module the editor acts on, or drop the pick.
    SelectSlot(Option<Slot>),
    /// Show the modules of another section.
    SelectSection(Section),
    /// Ask HyDE to switch the desktop to the named theme.
    SwitchHydeTheme(String),
    /// Report that the switch to the named theme has ended.
    ///
    /// Raised by the bar itself once the desktop's own switch has exited, so
    /// the page stops promising a switch that is over and states what the
    /// desktop actually settled on.
    HydeThemeSwitched {
        /// Theme the switch was asked for.
        theme:   String,
        /// Why the switch failed, when it did.
        failure: Option<String>
    },
    /// Move the indicator of a running switch on by one frame.
    ///
    /// Raised on a timer for as long as a switch is running, and never
    /// otherwise: the bar has no other reason to redraw itself while it waits
    /// on a desktop script, and a wait nobody can see reads as a press that was
    /// never taken.
    SwitchTick,
    /// Ask HyDE for the next wallpaper of the active theme.
    NextHydeWallpaper,
    /// Report that the wallpaper change has ended.
    HydeWallpaperChanged {
        /// Why the change failed, when it did.
        failure: Option<String>
    }
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
            Self::SetFollowHyde(_) => &["appearance", "follow_hyde"],
            Self::SetAutoScale(_) => &["appearance", "auto_scale"],
            Self::SetNotificationSource(_) => &["notifications", "source"],
            Self::SelectTab(_)
            | Self::EditLayout(_)
            | Self::SelectSlot(_)
            | Self::SelectSection(_)
            | Self::SwitchHydeTheme(_)
            | Self::HydeThemeSwitched {
                ..
            }
            | Self::SwitchTick
            | Self::NextHydeWallpaper
            | Self::HydeWallpaperChanged {
                ..
            } => &[]
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
            Self::SetFollowHyde(follow) => (*follow).into(),
            Self::SetAutoScale(auto) => (*auto).into(),
            Self::SetNotificationSource(source) => match source {
                NotificationSource::Builtin => "Builtin".into(),
                NotificationSource::Compositor => "Compositor".into(),
                NotificationSource::Daemon => "Daemon".into()
            },
            Self::SelectTab(_)
            | Self::EditLayout(_)
            | Self::SelectSlot(_)
            | Self::SelectSection(_)
            | Self::SwitchHydeTheme(_)
            | Self::HydeThemeSwitched {
                ..
            }
            | Self::SwitchTick
            | Self::NextHydeWallpaper
            | Self::HydeWallpaperChanged {
                ..
            } => SettingValue::Flag(false)
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
fn store_layout(config_path: &Path, modules: &Modules) {
    for (key, entries) in [
        ("left", &modules.left),
        ("center", &modules.center),
        ("right", &modules.right)
    ] {
        if let Err(err) = write_setting(config_path, &["modules", key], section_value(entries)) {
            warn!("failed to store the `{key}` modules: {err}");
        }
    }
}

/// What a press on a theme chip leads to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SwitchDecision {
    /// A switch is already running for the named theme, so this press is
    /// dropped.
    AlreadySwitching(String),
    /// No theme of that name is installed, so nothing is asked of the desktop.
    NotInstalled,
    /// The desktop is asked to switch.
    Start
}

/// Decides what to do with a press on the chip of `theme`.
///
/// Kept apart from the module state so both refusals can be stated once and
/// checked without a HyDE install. They are refusals rather than best effort
/// for the same reason: a HyDE switch rewrites the whole desktop over several
/// seconds and is not reentrant, so a second one started on top of the first
/// races it over the state file, the wallpaper cache and every generated
/// stylesheet; and HyDE's own switcher answers a name it does not know by
/// quietly keeping the current theme, which from the bar looks exactly like a
/// press that did nothing.
fn decide_switch(theme: &str, switching: Option<&str>, installed: &[String]) -> SwitchDecision {
    if let Some(pending) = switching {
        return SwitchDecision::AlreadySwitching(pending.to_owned());
    }

    if !installed.iter().any(|candidate| candidate == theme) {
        return SwitchDecision::NotInstalled;
    }

    SwitchDecision::Start
}

/// Runs a desktop command, reporting why it failed if it did.
///
/// The command is run detached rather than with its output collected: a HyDE
/// switch leaves background children behind that would keep a collected stream
/// open long after the switch itself is over, and the bar has to hear that the
/// switch ended when it ends. What the command printed goes to the bar's own
/// log, which is where the detail belongs.
async fn run_desktop_command(command: String) -> Option<String> {
    let command: Arc<str> = Arc::from(command);

    launcher::run_detached(&command)
        .await
        .err()
        .map(|error| error.to_string())
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
    section:     Section,
    /// Desktop state the HyDE page draws.
    ///
    /// Kept here rather than read while rendering: the page is redrawn on every
    /// frame of the open animation, and reading two files that often would put
    /// the filesystem in the draw path.
    hyde:        HydeState,
    /// Theme a switch is running for, while one is.
    ///
    /// A HyDE switch rewrites the whole desktop over several seconds and is not
    /// reentrant, so this is both what the page reports and what stops a second
    /// press from starting a switch that would race the first.
    switching:   Option<String>,
    /// Frame the indicator of a running switch is on.
    ///
    /// Advanced on a tick rather than derived from a clock read while drawing,
    /// so what the bar shows is a function of the state it holds and can be
    /// checked without one.
    spinner:     Spinner
}

impl Settings {
    /// Creates the module writing to `config_path`.
    #[must_use]
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            tab: Tab::default(),
            selected: None,
            section: Section::Left,
            hyde: hyde_state::load(),
            switching: None,
            spinner: Spinner::default()
        }
    }

    /// Desktop state the HyDE page draws.
    #[must_use]
    pub fn hyde(&self) -> &HydeState {
        &self.hyde
    }

    /// Theme a switch is running for, while one is.
    #[must_use]
    pub fn switching(&self) -> Option<&str> {
        self.switching.as_deref()
    }

    /// Frame the indicator of a running switch is on.
    ///
    /// Read while drawing both the bar entry and the HyDE page, so the mark on
    /// the bar and the mark in the window move together rather than each
    /// keeping a clock of its own.
    #[must_use]
    pub fn spinner(&self) -> Spinner {
        self.spinner
    }

    /// Whether the bar is waiting on a desktop change it asked for.
    ///
    /// The bar entry draws its indicator from this, and the application asks
    /// for the tick that moves the indicator on only while it holds.
    #[must_use]
    pub fn is_waiting(&self) -> bool {
        self.switching.is_some()
    }

    /// Re-reads the desktop state HyDE publishes.
    ///
    /// Called whenever the bar reloads because a HyDE file changed, so a switch
    /// made from a keybinding — or one made here and finished since — reaches
    /// the page without it having to be closed and opened again.
    pub fn refresh_hyde(&mut self) {
        self.hyde = hyde_state::load();
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
    /// Picking a tab is the only choice about the bar kept in memory;
    /// everything else lands in the configuration file and comes back through
    /// the reload.
    ///
    /// The HyDE choices are not about the bar at all: they are handed to the
    /// desktop's own scripts, which take seconds to run. Nothing about the
    /// desktop is therefore assumed here — the page reports that a switch is
    /// running, and what the desktop settled on is read back off disk once it
    /// has, so a switch that never happened is never drawn as if it had.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::SelectTab(tab) => {
                if tab == Tab::Hyde {
                    self.refresh_hyde();
                }

                self.tab = tab;
            }
            Message::SwitchHydeTheme(theme) => return self.switch_hyde_theme(theme, config),
            Message::HydeThemeSwitched {
                theme,
                failure
            } => self.hyde_theme_switched(&theme, failure.as_deref(), config),
            Message::SwitchTick => {
                if self.switching.is_some() {
                    self.spinner.advance();
                }
            }
            Message::NextHydeWallpaper => {
                return Task::perform(
                    run_desktop_command(hyde_shell::next_wallpaper()),
                    |failure| Message::HydeWallpaperChanged {
                        failure
                    }
                );
            }
            Message::HydeWallpaperChanged {
                failure
            } => {
                if let Some(reason) = failure {
                    error!("the wallpaper could not be changed: {reason}");
                    self.report(config, "the desktop refused to change the wallpaper");
                }
            }
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

    /// Hands a theme switch to the desktop, once it is worth handing over.
    ///
    /// Three things are settled before the desktop is disturbed, because each
    /// of them used to end in a page that claimed a switch nobody performed:
    /// a switch already under way is left alone, a theme that is not installed
    /// is refused outright — HyDE's own switcher would silently keep the
    /// current one — and a missing switch script is reported instead of being
    /// logged where nobody looks.
    fn switch_hyde_theme(&mut self, theme: String, config: &Config) -> Task<Message> {
        self.refresh_hyde();

        match decide_switch(&theme, self.switching.as_deref(), &self.hyde.themes) {
            SwitchDecision::AlreadySwitching(pending) => {
                info!("ignoring the switch to `{theme}`: `{pending}` is still being applied");
                return Task::none();
            }
            SwitchDecision::NotInstalled => {
                self.report(
                    config,
                    &format!("no HyDE theme named `{theme}` is installed")
                );
                return Task::none();
            }
            SwitchDecision::Start => {}
        }

        let command = match hyde_shell::switch_theme(&theme) {
            Ok(command) => command,
            Err(error) => {
                self.report(config, &format!("cannot switch the HyDE theme: {error}"));
                return Task::none();
            }
        };

        info!("switching the desktop to the HyDE theme `{theme}`");
        self.begin_switch(theme.clone());

        Task::perform(run_desktop_command(command), move |failure| {
            Message::HydeThemeSwitched {
                theme: theme.clone(),
                failure
            }
        })
    }

    /// Starts the wait on the switch to `theme`.
    ///
    /// The indicator is put back to its first frame here rather than left where
    /// the last switch abandoned it, so every wait looks the same from the
    /// press onwards instead of starting at whatever frame the previous one
    /// happened to end on.
    fn begin_switch(&mut self, theme: String) {
        self.switching = Some(theme);
        self.spinner = Spinner::default();
    }

    /// Records what the desktop made of the switch that just ended.
    fn hyde_theme_switched(&mut self, theme: &str, failure: Option<&str>, config: &Config) {
        self.switching = None;
        self.refresh_hyde();

        match failure {
            Some(reason) => {
                error!("the switch to the HyDE theme `{theme}` failed: {reason}");
                self.report(
                    config,
                    &format!("the desktop refused to switch to `{theme}`")
                );
            }
            None => info!("the desktop finished switching to the HyDE theme `{theme}`")
        }
    }

    /// Puts `message` in front of the user as well as in the log.
    ///
    /// A desktop action the bar could not perform has no other way of being
    /// noticed: the window that asked for it draws the desktop as it is, so a
    /// refused switch simply leaves the page unchanged and reads as the press
    /// having been missed.
    fn report(&self, config: &Config, message: &str) {
        warn!("{message}");

        notify(
            Notice::Error,
            NOTICE_DURATION,
            &compositor_color(config.appearance.primary_color.clone()),
            config.appearance.font_size_px(),
            message
        );
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

    /// Renders the bar entry, with the indicator of a running switch beside it.
    ///
    /// The indicator belongs on the bar and not only in the window because the
    /// window is not where the user is looking: a HyDE switch repaints the
    /// whole desktop, the settings window is dismissed or redrawn along with
    /// it, and the bar is the one surface that is certainly still on screen.
    /// The gear stays where it was so the entry is still recognisable as the
    /// one that was pressed.
    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let entry: Element<'static, M> = if self.is_waiting() {
            Row::new()
                .push(icon(icons, Icons::Settings))
                .push(icon_raw_sized(
                    self.spinner.glyph().to_owned(),
                    icons.size()
                ))
                .spacing(INDICATOR_GAP)
                .align_y(Alignment::Center)
                .into()
        } else {
            icon(icons, Icons::Settings).into()
        };

        Some((entry, Some(OnModulePress::ToggleMenu(MenuType::Settings))))
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

    fn installed() -> Vec<String> {
        vec!["Gruvbox Retro".to_owned(), "Tokyo Night".to_owned()]
    }

    #[test]
    fn an_installed_theme_is_handed_to_the_desktop() {
        assert_eq!(
            decide_switch("Tokyo Night", None, &installed()),
            SwitchDecision::Start
        );
    }

    #[test]
    fn a_theme_that_is_not_installed_is_refused_rather_than_attempted() {
        assert_eq!(
            decide_switch("Nordic Blue", None, &installed()),
            SwitchDecision::NotInstalled
        );
    }

    #[test]
    fn a_second_press_while_a_switch_runs_is_dropped() {
        assert_eq!(
            decide_switch("Tokyo Night", Some("Gruvbox Retro"), &installed()),
            SwitchDecision::AlreadySwitching("Gruvbox Retro".to_owned())
        );
    }

    #[test]
    fn pressing_the_theme_already_being_switched_to_does_not_start_it_twice() {
        assert_eq!(
            decide_switch("Tokyo Night", Some("Tokyo Night"), &installed()),
            SwitchDecision::AlreadySwitching("Tokyo Night".to_owned())
        );
    }

    #[test]
    fn a_machine_without_hyde_offers_no_theme_to_switch_to() {
        assert_eq!(
            decide_switch("Tokyo Night", None, &[]),
            SwitchDecision::NotInstalled
        );
    }

    #[test]
    fn a_finished_switch_releases_the_page_for_the_next_one() {
        let mut settings = Settings {
            switching: Some("Tokyo Night".to_owned()),
            ..Settings::default()
        };

        let _ = settings.update(
            Message::HydeThemeSwitched {
                theme:   "Tokyo Night".to_owned(),
                failure: None
            },
            &Config::default()
        );

        assert_eq!(settings.switching(), None);
    }

    #[test]
    fn a_page_that_is_not_switching_reports_nothing_pending() {
        assert_eq!(Settings::default().switching(), None);
    }

    fn tick(settings: &mut Settings) {
        let _ = settings.update(Message::SwitchTick, &Config::default());
    }

    #[test]
    fn a_bar_waiting_on_nothing_shows_no_indicator() {
        assert!(!Settings::default().is_waiting());
    }

    #[test]
    fn a_tick_moves_the_indicator_of_a_running_switch_on() {
        let mut settings = Settings {
            switching: Some("Tokyo Night".to_owned()),
            ..Settings::default()
        };
        let before = settings.spinner();

        tick(&mut settings);

        assert!(settings.is_waiting());
        assert_ne!(settings.spinner(), before);
    }

    /// The tick is only asked for while a switch runs, but a tick already in
    /// flight when one ends must not leave the indicator on a frame nobody
    /// draws.
    #[test]
    fn a_tick_arriving_after_the_switch_ended_moves_nothing() {
        let mut settings = Settings::default();
        let before = settings.spinner();

        tick(&mut settings);

        assert_eq!(settings.spinner(), before);
    }

    #[test]
    fn the_indicator_returns_to_its_first_frame_for_every_new_switch() {
        let mut settings = Settings {
            switching: Some("Tokyo Night".to_owned()),
            ..Settings::default()
        };

        tick(&mut settings);
        assert_ne!(settings.spinner(), Spinner::default());

        settings.begin_switch("Gruvbox Retro".to_owned());

        assert_eq!(settings.switching(), Some("Gruvbox Retro"));
        assert_eq!(settings.spinner(), Spinner::default());
    }

    #[test]
    fn a_finished_switch_takes_the_indicator_off_the_bar() {
        let mut settings = Settings {
            switching: Some("Tokyo Night".to_owned()),
            ..Settings::default()
        };

        let _ = settings.update(
            Message::HydeThemeSwitched {
                theme:   "Tokyo Night".to_owned(),
                failure: Some("the script died".to_owned())
            },
            &Config::default()
        );

        assert!(!settings.is_waiting());
    }

    #[test]
    fn the_indicator_never_writes_anything_into_the_configuration() {
        assert!(Message::SwitchTick.path().is_empty());
    }
}
