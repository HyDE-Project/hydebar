//! Bar module driving the desktop theme.
//!
//! Everything about the look of the desktop lives here: the installed themes,
//! the one in force, the facts HyDE reports about the wallpaper, and the two
//! actions that change either — switching the theme and asking for the next
//! wallpaper. The settings window is about the bar and holds none of it, so
//! there is one surface to look at rather than two that have to agree.
//!
//! The themes belong to the [HyDE Project](https://github.com/HyDE-Project)
//! rather than to the bar, so nothing chosen here is written into the bar's own
//! configuration file: pressing a theme asks HyDE's own switcher to run, and
//! the desktop — the bar included — follows. What the module shows is read back
//! from HyDE's state, so it reports the desktop as it is even when the change
//! came from a keybinding rather than from here.
//!
//! This is also the one place that knows a switch is running. A HyDE switch
//! rewrites the wallpaper, the palette and every generated stylesheet, and
//! takes seconds doing it; the module holds that wait, refuses a second switch
//! on top of it, and owns the indicator its menu and its bar entry draw.

mod progress;
mod view;

use hydebar_proto::{
    config::Config,
    hyde_state::{self, HydeState}
};
use iced::{Element, Task};
use log::{error, info};
pub use progress::{FRAME_INTERVAL, Spinner};

use super::{Module, OnModulePress};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon, icon_raw_sized},
        page
    },
    menu::MenuType,
    services::hyprland_notify::report,
    utils::hyde_shell
};

/// Gap between the bar entry and the indicator of a running switch, in pixels.
///
/// Narrow on purpose: the two glyphs have to read as one entry that is busy
/// rather than as two entries that happen to sit next to each other.
const INDICATOR_GAP: f32 = 4.0;

/// Choice made in the theme module.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Ask HyDE to switch the desktop to the named theme.
    Switch(String),
    /// Report that the switch to the named theme has ended.
    ///
    /// Raised by the bar itself once the desktop's own switch has exited, so
    /// the module stops promising a switch that is over and states what the
    /// desktop actually settled on.
    Switched {
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
    Tick,
    /// Ask HyDE for the next wallpaper of the theme in force.
    NextWallpaper,
    /// Report that the wallpaper change has ended.
    WallpaperChanged {
        /// Why the change failed, when it did.
        failure: Option<String>
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

/// Bar entry listing the installed desktop themes.
#[derive(Default, Debug, Clone)]
pub struct Themes {
    /// Desktop state the module draws.
    ///
    /// Kept here rather than read while rendering: the menu is redrawn on every
    /// frame of the open animation, and reading two files that often would put
    /// the filesystem in the draw path.
    hyde:      HydeState,
    /// Theme a switch is running for, while one is.
    switching: Option<String>,
    /// Frame the indicator of a running switch is on.
    ///
    /// Advanced on a tick rather than derived from a clock read while drawing,
    /// so what the bar shows is a function of the state it holds and can be
    /// checked without one.
    spinner:   Spinner
}

impl Themes {
    /// Creates the module against the desktop state on disk.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hyde:      hyde_state::load(),
            switching: None,
            spinner:   Spinner::default()
        }
    }

    /// Desktop state the module draws.
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
    /// Read while drawing the bar entry, the module menu and the settings
    /// window alike, so every mark of one wait moves together rather than each
    /// surface keeping a clock of its own.
    #[must_use]
    pub fn spinner(&self) -> Spinner {
        self.spinner
    }

    /// Whether the bar is waiting on a switch it asked for.
    ///
    /// The application asks for the tick that moves the indicator on only while
    /// this holds.
    #[must_use]
    pub fn is_waiting(&self) -> bool {
        self.switching.is_some()
    }

    /// Re-reads the desktop state HyDE publishes.
    ///
    /// Called whenever the bar reloads because a HyDE file changed, so a switch
    /// made from a keybinding — or one made here and finished since — reaches
    /// the module without its menu having to be closed and opened again.
    pub fn refresh(&mut self) {
        self.hyde = hyde_state::load();
    }

    /// Renders the menu the module opens.
    ///
    /// `opacity` is the menu opacity the surface is animating through, so the
    /// chips fade in with the box that holds them.
    #[must_use]
    pub fn menu_view<'a>(&self, config: &Config, opacity: f32) -> Element<'a, Message> {
        let font_size = config.appearance.font_size_px();

        view::view(
            &self.hyde,
            self.switching(),
            self.spinner,
            opacity,
            font_size,
            self.page_width(config)
        )
    }

    /// Width the menu draws into, the slack a row keeps excluded.
    fn page_width(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size_px();

        self.content_width(config) - page::metrics::ROW_SLACK_EM * font_size
    }

    /// Width the longest row of the menu needs.
    ///
    /// Measured rather than guessed for the same reason the settings window
    /// measures itself: the compositor is told how large the surface is before
    /// anything inside it has been laid out.
    #[must_use]
    pub fn content_width(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size_px();

        view::desired_width(&self.hyde, self.switching(), font_size)
            + page::metrics::ROW_SLACK_EM * font_size
    }

    /// Height the menu needs.
    #[must_use]
    pub fn content_height(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size_px();

        view::desired_height(&self.hyde, font_size, self.page_width(config))
    }

    /// Applies a choice made in the module.
    ///
    /// Nothing about the desktop is assumed: the module reports that a switch
    /// is running, and what the desktop settled on is read back off disk once
    /// it has, so a switch that never happened is never drawn as if it had.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::Switch(theme) => return self.switch(theme, config),
            Message::Switched {
                theme,
                failure
            } => self.switched(&theme, failure.as_deref(), config),
            Message::Tick => {
                if self.switching.is_some() {
                    self.spinner.advance();
                }
            }
            Message::NextWallpaper => {
                return Task::perform(hyde_shell::run(hyde_shell::next_wallpaper()), |failure| {
                    Message::WallpaperChanged {
                        failure
                    }
                });
            }
            Message::WallpaperChanged {
                failure
            } => {
                if let Some(reason) = failure {
                    error!("the wallpaper could not be changed: {reason}");
                    report(config, "the desktop refused to change the wallpaper");
                }
            }
        }

        Task::none()
    }

    /// Hands a theme switch to the desktop, once it is worth handing over.
    ///
    /// Three things are settled before the desktop is disturbed, because each
    /// of them used to end in a module that claimed a switch nobody performed:
    /// a switch already under way is left alone, a theme that is not installed
    /// is refused outright — HyDE's own switcher would silently keep the
    /// current one — and a missing switch script is reported instead of being
    /// logged where nobody looks.
    fn switch(&mut self, theme: String, config: &Config) -> Task<Message> {
        self.refresh();

        match decide_switch(&theme, self.switching.as_deref(), &self.hyde.themes) {
            SwitchDecision::AlreadySwitching(pending) => {
                info!("ignoring the switch to `{theme}`: `{pending}` is still being applied");
                return Task::none();
            }
            SwitchDecision::NotInstalled => {
                report(
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
                report(config, &format!("cannot switch the HyDE theme: {error}"));
                return Task::none();
            }
        };

        info!("switching the desktop to the HyDE theme `{theme}`");
        self.begin(theme.clone());

        Task::perform(hyde_shell::run(command), move |failure| Message::Switched {
            theme: theme.clone(),
            failure
        })
    }

    /// Starts the wait on the switch to `theme`.
    ///
    /// The indicator is put back to its first frame here rather than left where
    /// the last switch abandoned it, so every wait looks the same from the
    /// press onwards instead of starting at whatever frame the previous one
    /// happened to end on.
    fn begin(&mut self, theme: String) {
        self.switching = Some(theme);
        self.spinner = Spinner::default();
    }

    /// Records what the desktop made of the switch that just ended.
    fn switched(&mut self, theme: &str, failure: Option<&str>, config: &Config) {
        self.switching = None;
        self.refresh();

        match failure {
            Some(reason) => {
                error!("the switch to the HyDE theme `{theme}` failed: {reason}");
                report(
                    config,
                    &format!("the desktop refused to switch to `{theme}`")
                );
            }
            None => info!("the desktop finished switching to the HyDE theme `{theme}`")
        }
    }
}

impl<M> Module<M> for Themes
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    /// Renders the bar entry, with the indicator of a running switch beside it.
    ///
    /// The indicator belongs on the bar and not only in the menu because the
    /// menu is not where the user is looking: a HyDE switch repaints the whole
    /// desktop, a menu open over it is dismissed or redrawn along with it, and
    /// the bar is the one surface that is certainly still on screen. The module
    /// icon stays where it was so the entry is still recognisable as the one
    /// that was pressed.
    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let entry: Element<'static, M> = if self.is_waiting() {
            iced::widget::Row::new()
                .push(icon(icons, Icons::Themes))
                .push(icon_raw_sized(
                    self.spinner.glyph().to_owned(),
                    icons.size()
                ))
                .spacing(INDICATOR_GAP)
                .align_y(iced::Alignment::Center)
                .into()
        } else {
            icon(icons, Icons::Themes).into()
        };

        Some((entry, Some(OnModulePress::ToggleMenu(MenuType::Themes))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn waiting_on(theme: &str) -> Themes {
        Themes {
            switching: Some(theme.to_owned()),
            ..Themes::default()
        }
    }

    fn tick(themes: &mut Themes) {
        let _ = themes.update(Message::Tick, &Config::default());
    }

    #[test]
    fn a_finished_switch_releases_the_module_for_the_next_one() {
        let mut themes = waiting_on("Tokyo Night");

        let _ = themes.update(
            Message::Switched {
                theme:   "Tokyo Night".to_owned(),
                failure: None
            },
            &Config::default()
        );

        assert_eq!(themes.switching(), None);
        assert!(!themes.is_waiting());
    }

    #[test]
    fn a_module_that_is_not_switching_reports_nothing_pending() {
        assert_eq!(Themes::default().switching(), None);
        assert!(!Themes::default().is_waiting());
    }

    #[test]
    fn a_tick_moves_the_indicator_of_a_running_switch_on() {
        let mut themes = waiting_on("Tokyo Night");
        let before = themes.spinner();

        tick(&mut themes);

        assert_ne!(themes.spinner(), before);
    }

    /// The tick is only asked for while a switch runs, but a tick already in
    /// flight when one ends must not leave the indicator on a frame nobody
    /// draws.
    #[test]
    fn a_tick_arriving_after_the_switch_ended_moves_nothing() {
        let mut themes = Themes::default();
        let before = themes.spinner();

        tick(&mut themes);

        assert_eq!(themes.spinner(), before);
    }

    #[test]
    fn the_indicator_returns_to_its_first_frame_for_every_new_switch() {
        let mut themes = waiting_on("Tokyo Night");

        tick(&mut themes);
        assert_ne!(themes.spinner(), Spinner::default());

        themes.begin("Gruvbox Retro".to_owned());

        assert_eq!(themes.switching(), Some("Gruvbox Retro"));
        assert_eq!(themes.spinner(), Spinner::default());
    }

    /// The wallpaper action and the theme switch are two different desktop
    /// commands, and only the switch is the one the module refuses to run twice
    /// over; asking for a wallpaper must therefore neither start a wait nor end
    /// one that is running.
    #[test]
    fn asking_for_the_next_wallpaper_does_not_disturb_a_running_switch() {
        let mut themes = waiting_on("Tokyo Night");

        let _ = themes.update(Message::NextWallpaper, &Config::default());

        assert_eq!(themes.switching(), Some("Tokyo Night"));
        assert!(themes.is_waiting());
    }

    #[test]
    fn asking_for_the_next_wallpaper_starts_no_wait_of_its_own() {
        let mut themes = Themes::default();

        let _ = themes.update(Message::NextWallpaper, &Config::default());

        assert!(!themes.is_waiting());
    }

    #[test]
    fn a_refused_wallpaper_change_leaves_the_module_as_it_found_it() {
        let mut themes = waiting_on("Tokyo Night");

        let _ = themes.update(
            Message::WallpaperChanged {
                failure: Some("the script died".to_owned())
            },
            &Config::default()
        );

        assert_eq!(themes.switching(), Some("Tokyo Night"));
    }

    #[test]
    fn a_failed_switch_takes_the_indicator_off_the_bar() {
        let mut themes = waiting_on("Tokyo Night");

        let _ = themes.update(
            Message::Switched {
                theme:   "Tokyo Night".to_owned(),
                failure: Some("the script died".to_owned())
            },
            &Config::default()
        );

        assert!(!themes.is_waiting());
    }
}
