//! Bar module driving the desktop theme.
//!
//! Everything about the look of the desktop lives here: the installed themes,
//! the one in force, the facts `HyDE` reports about the wallpaper, and the two
//! actions that change either — switching the theme and asking for the next
//! wallpaper. The settings window is about the bar and holds none of it, so
//! there is one surface to look at rather than two that have to agree.
//!
//! The themes belong to the [HyDE Project](https://github.com/HyDE-Project)
//! rather than to the bar, so nothing chosen here is written into the bar's own
//! configuration file: pressing a theme asks `HyDE`'s own switcher to run, and
//! the desktop — the bar included — follows. What the module shows is read back
//! from `HyDE`'s state, so it reports the desktop as it is even when the change
//! came from a keybinding rather than from here.
//!
//! This is also the one place that knows a switch is running. A `HyDE` switch
//! rewrites the wallpaper, the palette and every generated stylesheet, and
//! takes seconds doing it; the module holds that wait, refuses a second switch
//! on top of it, and owns the indicator its menu and its bar entry draw.

mod progress;

/// The upstream theme catalogue, and the reader that fetches it.
mod gallery;

mod view;

use std::collections::HashMap;

use hydebar_proto::{
    config::Config,
    hyde_dirs::HydeDirs,
    hyde_state::{self, HydeState},
    theme_source::{ThemeSwatch, theme_swatch}
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
    /// Ask `HyDE` for the next theme in its own order.
    NextTheme,
    /// Ask `HyDE` for the previous theme in its own order.
    PreviousTheme,
    /// Report that a stepped switch has ended.
    Stepped {
        /// Why the desktop refused, if it did.
        failure: Option<String>
    },
    /// Ask `HyDE` to switch the desktop to the named theme.
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
    /// Deliver the swatches the themes announce themselves with.
    ///
    /// Raised by the reader the menu starts when it opens; the colours arrive
    /// a beat after the names because reading them hashes every theme's
    /// wallpaper, which is nothing the opening animation should wait on.
    SwatchesLoaded(
        HashMap<String, ThemeSwatch>,
        HashMap<String, std::path::PathBuf>
    ),
    /// Deliver the upstream catalogue the gallery section draws.
    CatalogueLoaded(Vec<gallery::GalleryTheme>, Option<String>),
    /// Install the named theme from the gallery, then switch to it.
    Install(String),
    /// Report that the install of the named theme has ended.
    Installed {
        /// Theme the install was asked for.
        theme:   String,
        /// Why the install failed, when it did.
        failure: Option<String>
    },
    /// Mark the named installed theme for removal, pending one more press.
    Condemn(String),
    /// Remove the named theme, previously condemned.
    Remove(String),
    /// Report that the removal of the named theme has ended.
    Removed {
        /// Theme the removal was asked for.
        theme:   String,
        /// Why the removal failed, when it did.
        failure: Option<String>
    },
    /// Fetch updates for one installed theme, or all of them.
    Update(Option<String>),
    /// Flip the window between the grid and the single-column layout.
    ToggleLayout,
    /// Report that an update fetch has ended.
    Updated {
        /// Why the fetch failed, when it did.
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
/// checked without a `HyDE` install. They are refusals rather than best effort
/// for the same reason: a `HyDE` switch rewrites the whole desktop over several
/// seconds and is not reentrant, so a second one started on top of the first
/// races it over the state file, the wallpaper cache and every generated
/// stylesheet; and `HyDE`'s own switcher answers a name it does not know by
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

/// Reads the swatch of every named theme from the `HyDE` install on disk.
fn read_swatches(themes: &[String]) -> HashMap<String, ThemeSwatch> {
    let Some(dirs) = HydeDirs::from_env() else {
        return HashMap::new();
    };

    themes
        .iter()
        .filter_map(|name| theme_swatch(&dirs, name).map(|swatch| (name.clone(), swatch)))
        .collect()
}

/// Bar entry listing the installed desktop themes.
#[derive(Default, Debug, Clone)]
pub struct Themes {
    /// Desktop state the module draws.
    ///
    /// Kept here rather than read while rendering: the menu is redrawn on every
    /// frame of the open animation, and reading two files that often would put
    /// the filesystem in the draw path.
    hyde:            HydeState,
    /// The colours each theme announces itself with, by theme name.
    ///
    /// Loaded off the update path — see [`Themes::load_swatches`] — and kept
    /// so the menu can paint every chip in the colours of the theme it stands
    /// for. A theme without an entry is painted like any other control.
    swatches:        HashMap<String, ThemeSwatch>,
    /// Screenshots of the desktop wearing each theme, from the local gallery
    /// database, by canonical name.
    screenshots:     HashMap<String, std::path::PathBuf>,
    /// Theme a switch is running for, while one is.
    switching:       Option<String>,
    /// Theme asked for while another switch was still running.
    ///
    /// A press mid-switch used to vanish without a word; now it waits its
    /// turn and runs the moment the desktop settles.
    pending:         Option<String>,
    /// The upstream catalogue, once the menu has loaded it.
    catalogue:       Vec<gallery::GalleryTheme>,
    /// Name the machine's git identity signs work with, once known.
    author:          Option<String>,
    /// Theme an install is running for, while one is.
    installing:      Option<String>,
    /// Theme whose removal waits for its confirming press, if one does.
    condemned:       Option<String>,
    /// Theme an update is fetching, while one is; `None` name means all.
    #[expect(
        clippy::option_option,
        reason = "the outer layer marks a running fetch, the inner one tells one theme from all"
    )]
    updating:        Option<Option<String>>,
    /// Whether the window lays cards out as one column instead of a grid.
    list_layout:     bool,
    /// Frame the indicator of a running switch is on.
    ///
    /// Advanced on a tick rather than derived from a clock read while drawing,
    /// so what the bar shows is a function of the state it holds and can be
    /// checked without one.
    spinner:         Spinner,
    /// Gallery names not installed, refreshed when either side changes.
    ///
    /// The comparison normalises every spelling; done per frame it was
    /// thousands of small strings, done here it is none.
    offered:         Vec<String>,
    /// Catalogue positions by canonical name, refreshed with the catalogue.
    catalogue_index: HashMap<String, usize>
}

impl Themes {
    /// Creates the module against the desktop state on disk.
    #[must_use]
    pub fn new() -> Self {
        let mut themes = Self {
            hyde:            hyde_state::load(),
            swatches:        HashMap::new(),
            screenshots:     HashMap::new(),
            switching:       None,
            pending:         None,
            catalogue:       Vec::new(),
            author:          None,
            installing:      None,
            condemned:       None,
            updating:        None,
            list_layout:     false,
            spinner:         Spinner::default(),
            offered:         Vec::new(),
            catalogue_index: HashMap::new()
        };
        themes.reindex();

        themes
    }

    /// Starts reading the swatch of every installed theme, off this thread.
    ///
    /// Reading one swatch hashes that theme's current wallpaper, and a dozen
    /// themes make that a moment of real work; done inline it would land in
    /// the middle of the menu's opening animation. The colours arrive through
    /// [`Message::SwatchesLoaded`] and the open menu picks them up on its next
    /// frame.
    #[must_use]
    pub fn load_swatches(&self) -> Task<Message> {
        let themes = self.hyde.themes.clone();

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    (read_swatches(&themes), gallery::local_screenshots())
                })
                .await
                .unwrap_or_default()
            },
            |(swatches, screenshots)| Message::SwatchesLoaded(swatches, screenshots)
        )
    }

    /// Desktop state the module draws.
    #[must_use]
    pub const fn hyde(&self) -> &HydeState {
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
    pub const fn spinner(&self) -> Spinner {
        self.spinner
    }

    /// Whether the bar is waiting on a switch it asked for.
    ///
    /// The application asks for the tick that moves the indicator on only while
    /// this holds.
    #[must_use]
    pub const fn is_waiting(&self) -> bool {
        if self.installing.is_some() || self.updating.is_some() {
            return true;
        }

        self.switching.is_some()
    }

    /// Re-reads the desktop state `HyDE` publishes.
    ///
    /// Called whenever the bar reloads because a `HyDE` file changed, so a
    /// switch made from a keybinding — or one made here and finished since
    /// — reaches the module without its menu having to be closed and opened
    /// again.
    pub fn refresh(&mut self) {
        self.hyde = hyde_state::load();
        self.reindex();
    }

    /// Restates the offered names and the catalogue index.
    ///
    /// Called when the installed set or the catalogue moves — the only two
    /// inputs the derivations read.
    fn reindex(&mut self) {
        let installed: std::collections::HashSet<String> = self
            .hyde
            .themes
            .iter()
            .map(|name| view::canonical(name))
            .collect();

        self.offered = self
            .catalogue
            .iter()
            .filter(|entry| !installed.contains(&view::canonical(&entry.name)))
            .map(|entry| entry.name.clone())
            .collect();

        self.catalogue_index = self
            .catalogue
            .iter()
            .enumerate()
            .map(|(index, entry)| (view::canonical(&entry.name), index))
            .collect();
    }

    /// Renders the menu the module opens.
    ///
    /// `opacity` is the menu opacity the surface is animating through, so the
    /// chips fade in with the box that holds them.
    #[must_use]
    pub fn menu_view<'a>(
        &self,
        config: &Config,
        opacity: f32,
        page_width: f32
    ) -> Element<'a, Message> {
        let font_size = config.appearance.font_size_px();

        view::view(
            &self.hyde,
            &self.swatches,
            &self.screenshots,
            self.switching(),
            &self.catalogue,
            &self.offered,
            &self.catalogue_index,
            self.author.as_deref(),
            self.installing.as_deref(),
            self.condemned.as_deref(),
            self.updating.as_ref(),
            self.list_layout,
            self.spinner,
            opacity,
            font_size,
            page_width
        )
    }

    /// The three window lengths, with the content walked exactly once.
    #[must_use]
    pub fn window_metrics(&self, config: &Config) -> crate::menu::MenuMetrics {
        let font_size = config.appearance.font_size_px();
        let width = self.content_width(config);
        let page_width = page::metrics::ROW_SLACK_EM.mul_add(-font_size, width);

        crate::menu::MenuMetrics {
            width,
            page_width,
            height: view::desired_height(
                &self.hyde,
                &self.offered,
                self.list_layout,
                font_size,
                page_width
            )
        }
    }

    /// Width the longest row of the menu needs.
    ///
    /// Measured rather than guessed for the same reason the settings window
    /// measures itself: the compositor is told how large the surface is before
    /// anything inside it has been laid out.
    #[must_use]
    pub fn content_width(&self, config: &Config) -> f32 {
        let font_size = config.appearance.font_size_px();

        page::metrics::ROW_SLACK_EM.mul_add(
            font_size,
            view::desired_width(&self.hyde, self.switching(), font_size)
        )
    }

    /// Height the menu needs.
    #[must_use]
    pub fn content_height(&self, config: &Config) -> f32 {
        self.window_metrics(config).height
    }

    /// Applies a choice made in the module.
    ///
    /// Nothing about the desktop is assumed: the module reports that a switch
    /// is running, and what the desktop settled on is read back off disk once
    /// it has, so a switch that never happened is never drawn as if it had.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::NextTheme => return Self::step("hydectl theme next"),
            Message::PreviousTheme => return Self::step("hydectl theme prev"),
            Message::Stepped {
                failure
            } => {
                if let Some(reason) = failure {
                    error!("the theme step failed: {reason}");
                    report(config, "the desktop refused to step the theme");
                }

                self.refresh();

                return self.load_swatches();
            }
            Message::Switch(theme) => {
                info!("theme chip pressed: `{theme}`");

                return self.switch(theme, config);
            }
            Message::Switched {
                theme,
                failure
            } => {
                self.switched(&theme, failure.as_deref(), config);

                if let Some(next) = self.pending.take() {
                    return Task::batch([self.load_swatches(), self.switch(next, config)]);
                }

                return self.load_swatches();
            }
            Message::Tick => {
                if self.switching.is_some() || self.installing.is_some() || self.updating.is_some()
                {
                    self.spinner.advance();
                }
            }
            Message::SwatchesLoaded(swatches, screenshots) => {
                self.swatches = swatches;
                self.screenshots = screenshots;
            }
            Message::CatalogueLoaded(catalogue, author) => {
                self.catalogue = catalogue;
                self.reindex();
                self.author = author;
                return self.auto_update();
            }
            Message::Update(scope) => return self.fetch_updates(scope),
            Message::ToggleLayout => self.list_layout = !self.list_layout,
            Message::Updated {
                failure
            } => {
                self.updating = None;

                if let Some(failure) = failure {
                    report(config, &format!("updating HyDE themes failed: {failure}"));
                }

                self.refresh();

                return self.load_swatches();
            }
            Message::Install(theme) => {
                self.condemned = None;
                return self.install(theme, config);
            }
            Message::Condemn(theme) => {
                if self.switching.is_some() {
                    report(config, "a theme switch is running, removal must wait");
                } else if self.installing.is_some() {
                    report(config, "a theme install is running, removal must wait");
                } else if self.hyde.theme.as_deref() == Some(theme.as_str()) {
                    report(
                        config,
                        "the theme in force cannot be removed, switch away first"
                    );
                } else if self.hyde.themes.contains(&theme) {
                    self.condemned = Some(theme);
                } else {
                    report(config, "this theme is not installed");
                }
            }
            Message::Remove(theme) => return self.remove(theme, config),
            Message::Removed {
                theme,
                failure
            } => return self.removed(&theme, failure.as_deref(), config),
            Message::Installed {
                theme,
                failure
            } => return self.installed(theme, failure, config)
        }

        Task::none()
    }

    /// Asks the desktop to step to a neighbouring theme in its own order.
    fn step(command: &str) -> Task<Message> {
        Task::perform(hyde_shell::run(command.to_owned()), |failure| {
            Message::Stepped {
                failure
            }
        })
    }

    /// Records what the desktop made of the removal that just ended.
    fn removed(&mut self, theme: &str, failure: Option<&str>, config: &Config) -> Task<Message> {
        match failure {
            Some(failure) => report(
                config,
                &format!("removing the HyDE theme `{theme}` failed: {failure}")
            ),
            None => report(config, &format!("the HyDE theme `{theme}` is removed"))
        }

        self.refresh();

        self.load_swatches()
    }

    /// Records what the desktop made of the install that just ended, and
    /// switches to the theme once it has landed.
    fn installed(
        &mut self,
        theme: String,
        failure: Option<String>,
        config: &Config
    ) -> Task<Message> {
        self.installing = None;

        if let Some(failure) = failure {
            report(
                config,
                &format!("installing the HyDE theme `{theme}` failed: {failure}")
            );

            return Task::none();
        }

        info!("the HyDE theme `{theme}` is installed, switching to it");
        self.refresh();

        Task::batch([self.load_swatches(), self.switch(theme, config)])
    }

    /// Hands a gallery install to the desktop's own importer.
    ///
    /// One at a time, and never beside a running switch: both write the same
    /// theme directories, and two writers racing over them is how a desktop
    /// ends up half one theme and half another.
    fn install(&mut self, theme: String, config: &Config) -> Task<Message> {
        if let Some(pending) = self.switching.as_deref() {
            info!("ignoring the install of `{theme}`: `{pending}` is still being applied");
            return Task::none();
        }

        if let Some(pending) = self.installing.as_deref() {
            info!("ignoring the install of `{theme}`: `{pending}` is still being installed");
            return Task::none();
        }

        let Some(entry) = self.catalogue.iter().find(|entry| entry.name == theme) else {
            report(
                config,
                &format!("the gallery lists no theme named `{theme}`")
            );
            return Task::none();
        };

        info!("installing the HyDE theme `{theme}` from `{}`", entry.link);
        self.installing = Some(theme.clone());

        let command = gallery::import_command(&entry.name, &entry.link);

        Task::perform(hyde_shell::run(command), move |failure| {
            Message::Installed {
                theme,
                failure
            }
        })
    }

    /// Removes a condemned theme's directory, once everything checks out.
    ///
    /// Only a theme the removal was armed for goes, never the one the desktop
    /// is on, never during a switch or an install, and strictly the directory
    /// the installed list names — nothing about the path comes from outside.
    fn remove(&mut self, theme: String, config: &Config) -> Task<Message> {
        if self.condemned.as_deref() != Some(theme.as_str()) {
            return Task::none();
        }

        self.condemned = None;

        if self.switching.is_some()
            || self.installing.is_some()
            || self.hyde.theme.as_deref() == Some(theme.as_str())
            || !self.hyde.themes.contains(&theme)
        {
            return Task::none();
        }

        let Some(directory) = dirs::config_dir().map(|dir| dir.join("hyde/themes").join(&theme))
        else {
            report(config, "the theme directory cannot be located");
            return Task::none();
        };

        info!(
            "removing the HyDE theme `{theme}` at {}",
            directory.display()
        );

        Task::perform(
            async move {
                tokio::fs::remove_dir_all(directory)
                    .await
                    .err()
                    .map(|error| error.to_string())
            },
            move |failure| Message::Removed {
                theme,
                failure
            }
        )
    }

    /// Fetches theme updates through the desktop's own importer.
    ///
    /// One writer at a time over the theme directories: a fetch never starts
    /// beside a switch, an install, or another fetch.
    fn fetch_updates(&mut self, scope: Option<String>) -> Task<Message> {
        if self.switching.is_some() || self.installing.is_some() || self.updating.is_some() {
            return Task::none();
        }

        let command = scope.as_ref().map_or_else(
            || "hyde-shell theme.import --fetch all".to_owned(),
            |theme| {
                format!(
                    "hyde-shell theme.import --fetch '{}'",
                    theme.replace('\'', "'\\''")
                )
            }
        );

        info!("fetching HyDE theme updates: {command}");
        self.updating = Some(scope);

        Task::perform(hyde_shell::run(command), |failure| Message::Updated {
            failure
        })
    }

    /// Fetches all updates quietly, at most once a day.
    ///
    /// The professional half of the update button: opening the window checks
    /// a stamp beside the catalogue cache, and a stale stamp starts the same
    /// fetch the button runs — silently, with the same one-writer guards.
    fn auto_update(&mut self) -> Task<Message> {
        const STAMP_LIFE: std::time::Duration = std::time::Duration::from_hours(24);

        let Some(stamp) = dirs::cache_dir().map(|dir| dir.join("hydebar/theme-update-stamp"))
        else {
            return Task::none();
        };

        let fresh = std::fs::metadata(&stamp)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < STAMP_LIFE);

        if fresh {
            return Task::none();
        }

        if let Some(dir) = stamp.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&stamp, b"");

        self.fetch_updates(None)
    }

    /// Starts the catalogue reader, for the gallery section of the menu.
    ///
    /// A catalogue already in hand is kept: the gallery changes on the scale
    /// of weeks, and re-reading it on every menu open would probe the
    /// capability binary and re-parse the index each time. The next bar start
    /// reads it fresh.
    #[must_use]
    pub fn load_catalogue(&self) -> Task<Message> {
        if !self.catalogue.is_empty() {
            return Task::none();
        }

        Task::perform(
            async { (gallery::load().await, gallery::local_author().await) },
            |(catalogue, author)| Message::CatalogueLoaded(catalogue, author)
        )
    }

    /// Hands a theme switch to the desktop, once it is worth handing over.
    ///
    /// Three things are settled before the desktop is disturbed, because each
    /// of them used to end in a module that claimed a switch nobody performed:
    /// a switch already under way is left alone, a theme that is not installed
    /// is refused outright — `HyDE`'s own switcher would silently keep the
    /// current one — and a missing switch script is reported instead of being
    /// logged where nobody looks.
    fn switch(&mut self, theme: String, config: &Config) -> Task<Message> {
        self.refresh();

        info!(
            "deciding `{theme}`: switching={:?} pending={:?}",
            self.switching, self.pending
        );

        match decide_switch(&theme, self.switching.as_deref(), &self.hyde.themes) {
            SwitchDecision::AlreadySwitching(running) => {
                if self.switching.as_deref() == Some(theme.as_str()) {
                    report(config, &format!("`{theme}` is being applied right now"));
                } else if self.pending.as_deref() == Some(theme.as_str()) {
                    report(config, &format!("`{theme}` is already queued next"));
                } else {
                    report(
                        config,
                        &format!("`{running}` is still being applied; `{theme}` is queued next")
                    );
                    self.pending = Some(theme);
                }

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
            theme,
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
    /// menu is not where the user is looking: a `HyDE` switch repaints the
    /// whole desktop, a menu open over it is dismissed or redrawn along
    /// with it, and the bar is the one surface that is certainly still on
    /// screen. The module icon stays where it was so the entry is still
    /// recognisable as the one that was pressed.
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
