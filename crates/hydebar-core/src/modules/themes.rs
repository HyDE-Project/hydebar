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

mod progress {
    //! Live indication of a desktop change the bar has asked for and cannot
    //! hurry.
    //!
    //! A HyDE theme switch rewrites the wallpaper, the palette and every
    //! generated stylesheet, and takes seconds doing it. For all of those
    //! seconds the bar has nothing to report except that it is still
    //! waiting, and a page that reported it with a line of static text read
    //! exactly like a page that had missed the press. What is drawn instead
    //! is a glyph that keeps moving: a still frame cannot be told from a
    //! hung one, a moving frame can.
    //!
    //! The glyphs come from the icon font the bar bundles rather than from the
    //! text font. A spinner built out of braille or box-drawing characters
    //! depends on whatever the system font happens to cover, while every
    //! other icon on the bar is already drawn from this font and is
    //! therefore certain to render.
    //!
    //! Nothing here reads a clock. The frame is state the module owns and
    //! advances on a tick, so what the indicator shows is a pure function
    //! of how many ticks have been delivered, and both the cycle and the
    //! pulse can be checked without a frame clock, a compositor or a HyDE
    //! install.

    use std::time::Duration;

    /// Glyphs the indicator cycles through, in order.
    ///
    /// The `circle-slice` series of the bundled icon font: a pie that fills one
    /// eighth at a time and starts over. It reads as work in progress rather
    /// than as a measured fraction of it, which is the honest thing to draw
    /// for a switch whose remaining time the bar has no way of knowing.
    const FRAMES: [&str; 8] = [
        "\u{f0a9e}",
        "\u{f0a9f}",
        "\u{f0aa0}",
        "\u{f0aa1}",
        "\u{f0aa2}",
        "\u{f0aa3}",
        "\u{f0aa4}",
        "\u{f0aa5}"
    ];

    /// How long one frame stays on screen.
    ///
    /// Fast enough to read as motion, slow enough that a whole switch costs a
    /// few dozen redraws of a bar that is idle anyway. The bar asks the
    /// compositor for a frame on this cadence only while a switch is
    /// running; the rest of the time this constant costs nothing.
    pub const FRAME_INTERVAL: Duration = Duration::from_millis(110);

    /// Smallest share of its colour the pulsing mark is drawn at.
    ///
    /// Above zero on purpose: a mark that faded out completely would blink
    /// rather than pulse, and a blinking chip is indistinguishable from one
    /// being redrawn wrongly.
    const MIN_PULSE: f32 = 0.55;

    /// Frame of the indicator drawn while the bar waits on the desktop.
    ///
    /// Cheap to hold and cheap to copy: one index, advanced on a tick and read
    /// while drawing.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct Spinner {
        /// Which of [`FRAMES`] is on screen.
        frame: usize
    }

    impl Spinner {
        /// Number of frames one full cycle takes.
        #[must_use]
        pub fn cycle() -> usize {
            FRAMES.len()
        }

        /// Moves the indicator on by one frame, starting the cycle over at the
        /// end.
        pub fn advance(&mut self) {
            self.frame = (self.frame + 1) % FRAMES.len();
        }

        /// Glyph this frame draws.
        #[must_use]
        pub fn glyph(self) -> &'static str {
            FRAMES[self.frame % FRAMES.len()]
        }

        /// Share of its colour a mark following this indicator is drawn at.
        ///
        /// Rises and falls over the cycle rather than sawing back to the start,
        /// so a chip tinted with it breathes instead of snapping dark
        /// once a cycle.
        #[must_use]
        pub fn pulse(self) -> f32 {
            let half = FRAMES.len() as f32 / 2.0;
            let position = self.frame as f32;
            let distance = (position - half).abs() / half;

            MIN_PULSE + (1.0 - MIN_PULSE) * distance
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_fresh_indicator_starts_at_the_first_frame() {
            assert_eq!(Spinner::default().glyph(), FRAMES[0]);
        }

        #[test]
        fn a_tick_moves_the_indicator_on() {
            let mut spinner = Spinner::default();
            spinner.advance();

            assert_eq!(spinner.glyph(), FRAMES[1]);
        }

        #[test]
        fn a_full_cycle_of_ticks_returns_to_the_first_frame() {
            let mut spinner = Spinner::default();

            for _ in 0..Spinner::cycle() {
                spinner.advance();
            }

            assert_eq!(spinner, Spinner::default());
        }

        #[test]
        fn every_frame_of_a_cycle_is_drawn_before_any_is_drawn_twice() {
            let mut spinner = Spinner::default();
            let mut seen = Vec::new();

            for _ in 0..Spinner::cycle() {
                seen.push(spinner.glyph());
                spinner.advance();
            }

            seen.sort_unstable();
            seen.dedup();

            assert_eq!(seen.len(), Spinner::cycle());
        }

        #[test]
        fn the_pulse_stays_inside_the_range_a_colour_can_be_scaled_by() {
            let mut spinner = Spinner::default();

            for _ in 0..Spinner::cycle() {
                let pulse = spinner.pulse();

                assert!((MIN_PULSE..=1.0).contains(&pulse), "{pulse}");
                spinner.advance();
            }
        }

        /// A mark that pulsed by the same amount on every frame would be a mark
        /// that does not pulse at all.
        #[test]
        fn the_pulse_moves_over_a_cycle() {
            let mut spinner = Spinner::default();
            let mut lowest = f32::MAX;
            let mut highest = f32::MIN;

            for _ in 0..Spinner::cycle() {
                lowest = lowest.min(spinner.pulse());
                highest = highest.max(spinner.pulse());
                spinner.advance();
            }

            assert!(highest - lowest > 0.2, "{lowest} to {highest}");
        }

        #[test]
        fn a_frame_lasts_long_enough_to_be_read_and_short_enough_to_be_motion() {
            assert!(FRAME_INTERVAL >= Duration::from_millis(60));
            assert!(FRAME_INTERVAL <= Duration::from_millis(200));
        }
    }
}

/// The upstream theme catalogue, and the reader that fetches it.
mod gallery {
    use std::time::Duration;

    use serde::Deserialize;

    /// Where the catalogue lives.
    const INDEX_URL: &str = "https://raw.githubusercontent.com/HyDE-Project/hyde-gallery/hyde-gallery/hyde-themes.json";

    /// How long a fetched catalogue serves before it is fetched again.
    const CACHE_LIFE: Duration = Duration::from_secs(24 * 60 * 60);

    /// One theme the gallery offers.
    #[derive(Debug, Clone, PartialEq)]
    pub struct GalleryTheme {
        pub name:        String,
        pub link:        String,
        pub owner:       String,
        pub description: String,
        /// The two colours the index announces the theme with.
        pub colors:      [iced::Color; 2]
    }

    /// One entry as the index spells it.
    #[derive(Debug, Deserialize)]
    struct Entry {
        #[serde(rename = "THEME")]
        theme:       String,
        #[serde(rename = "LINK")]
        link:        String,
        #[serde(rename = "OWNER", default)]
        owner:       String,
        #[serde(rename = "DESCRIPTION", default)]
        description: String,
        #[serde(rename = "COLORSCHEME", default)]
        colors:      Vec<String>
    }

    /// Parses the catalogue, which is several JSON arrays laid end to end.
    ///
    /// The published file is not one document — strict parsing rejects it —
    /// so the arrays are decoded in sequence and joined, and entries whose
    /// colours do not parse are dropped rather than failing the rest.
    pub fn parse(raw: &str) -> Vec<GalleryTheme> {
        let mut themes = Vec::new();

        for chunk in serde_json::Deserializer::from_str(raw).into_iter::<Vec<Entry>>() {
            let Ok(entries) = chunk else {
                break;
            };

            for entry in entries {
                let Some(colors) = announced_colors(&entry.colors) else {
                    continue;
                };

                themes.push(GalleryTheme {
                    name: entry.theme,
                    link: entry.link,
                    owner: entry.owner,
                    description: entry.description,
                    colors
                });
            }
        }

        themes
    }

    /// The two announced colours, when both spell valid hex.
    fn announced_colors(colors: &[String]) -> Option<[iced::Color; 2]> {
        let first = hex(colors.first()?)?;
        let second = hex(colors.get(1)?)?;

        Some([first, second])
    }

    /// A colour as the index spells it.
    fn hex(value: &str) -> Option<iced::Color> {
        let parsed = hex_color::HexColor::parse(value).ok()?;

        Some(iced::Color::from_rgb8(parsed.r, parsed.g, parsed.b))
    }

    /// The command the install runs, quoted for the shell.
    pub fn import_command(name: &str, link: &str) -> String {
        format!(
            "hydectl theme import --name '{}' --url '{}'",
            name.replace('\'', "'\\''"),
            link.replace('\'', "'\\''")
        )
    }

    /// Where the fetched catalogue is kept between fetches.
    fn cache_path() -> Option<std::path::PathBuf> {
        Some(dirs::cache_dir()?.join("hydebar").join("hyde-gallery.json"))
    }

    /// Whether the installer the import runs through exists at all.
    async fn importer_present() -> bool {
        tokio::process::Command::new("hydectl")
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Reads the catalogue, from the cache while it is fresh.
    ///
    /// Every failure — no installer, no network, a bad response — answers
    /// with an empty catalogue, and the window simply shows no gallery.
    pub async fn load() -> Vec<GalleryTheme> {
        if !importer_present().await {
            return Vec::new();
        }

        let cache = cache_path();

        if let Some(path) = &cache
            && let Ok(metadata) = tokio::fs::metadata(path).await
            && metadata
                .modified()
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|age| age < CACHE_LIFE)
            && let Ok(raw) = tokio::fs::read_to_string(path).await
        {
            let themes = parse(&raw);

            if !themes.is_empty() {
                return themes;
            }
        }

        let Ok(response) = reqwest::get(INDEX_URL).await else {
            return stale(cache.as_deref()).await;
        };
        let Ok(raw) = response.text().await else {
            return stale(cache.as_deref()).await;
        };

        let themes = parse(&raw);

        if themes.is_empty() {
            return stale(cache.as_deref()).await;
        }

        if let Some(path) = &cache {
            if let Some(dir) = path.parent() {
                let _ = tokio::fs::create_dir_all(dir).await;
            }
            let _ = tokio::fs::write(path, &raw).await;
        }

        themes
    }

    /// Whatever the cache still holds, fresh or not.
    async fn stale(cache: Option<&std::path::Path>) -> Vec<GalleryTheme> {
        match cache {
            Some(path) => match tokio::fs::read_to_string(path).await {
                Ok(raw) => parse(&raw),
                Err(_) => Vec::new()
            },
            None => Vec::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_index_is_read_even_when_it_is_two_arrays_end_to_end() {
            let raw = r##"[{"THEME":"A","LINK":"l","OWNER":"o","DESCRIPTION":"d",
                "COLORSCHEME":["#111111","#222222"]}]
                [{"THEME":"B","LINK":"m","OWNER":"p","DESCRIPTION":"e",
                "COLORSCHEME":["#333333","#444444"]}]"##;

            let themes = parse(raw);

            assert_eq!(themes.len(), 2);
            assert_eq!(themes[0].name, "A");
            assert_eq!(themes[1].name, "B");
        }

        #[test]
        fn an_entry_with_broken_colours_is_dropped_alone() {
            let raw = r##"[{"THEME":"A","LINK":"l","OWNER":"o","DESCRIPTION":"d",
                "COLORSCHEME":["nope","#222222"]},
                {"THEME":"B","LINK":"m","OWNER":"p","DESCRIPTION":"e",
                "COLORSCHEME":["#333333","#444444"]}]"##;

            let themes = parse(raw);

            assert_eq!(themes.len(), 1);
            assert_eq!(themes[0].name, "B");
        }

        #[test]
        fn one_theme_is_recognised_under_every_spelling() {
            use crate::modules::themes::view::same_theme;

            assert!(same_theme("Catppuccin Mocha", "Catppuccin-Mocha"));
            assert!(same_theme("one_dark", "One Dark"));
            assert!(!same_theme("Tokyo Night", "Nordic Blue"));
        }

        #[test]
        fn the_import_command_survives_a_quoted_name() {
            let command = import_command("O'Dark", "https://x/y");

            assert!(command.starts_with("hydectl theme import --name "));
            assert!(command.contains("https://x/y"));
        }
    }
}

mod view {
    //! Menu of the theme module: the desktop's look, and every way of changing
    //! it.
    //!
    //! The menu is the only place the bar draws any of this. It states what the
    //! desktop is on and lists what it could be on. The settings window used to
    //! hold a page of its own and no longer does, so there is one list, one set
    //! of chip states and one wait indicator rather than two that have to
    //! be kept in step.

    use std::collections::HashMap;

    use hydebar_proto::{
        hyde_state::HydeState,
        theme_source::{Rgba, ThemeSwatch}
    };
    use iced::Element;

    use super::{Message, Spinner};
    use crate::components::{
        icons::Icons,
        page::{
            metrics::{
                chip_cell_width, chip_width, indicator_width, status_row_width, text_width,
                wrap_chips_into_rows
            },
            style, widgets,
            widgets::{
                ChipPaint, ThemeChip, grid, group, note, page, rows as row_stack, section,
                status_row, theme_chip
            }
        }
    };

    /// Height of a card's action row, in multiples of the control size.
    const ACTIONS_ROW_EM: f32 = 1.8;

    /// Height of the update-all row, in multiples of the control size.
    const UPDATE_ALL_ROW_EM: f32 = 2.0;

    /// Theme chips a row is sized to hold.
    ///
    /// The menu has to name a width before it knows how the chips wrap, and
    /// this is the number that keeps a typical HyDE install to a handful of
    /// rows without making the menu wider than it needs to be.
    const THEMES_PER_ROW: f32 = 3.0;

    /// Title of the section listing the installed themes.
    const THEMES: &str = "Installed";

    /// Title of the section listing the gallery the desktop can install from.
    const GALLERY: &str = "Available";

    /// Label of the row naming what the desktop is on.
    const ACTIVE: &str = "Active";

    /// Shown in place of the theme name while HyDE has recorded none.
    const UNKNOWN: &str = "unknown";

    /// Placed between the theme in force and the one being switched to.
    ///
    /// A switch takes seconds, and for most of them the desktop is still on the
    /// old theme; naming both is the only honest thing the menu can draw,
    /// and it is also what tells the user the press was taken.
    const SWITCHING_TO: &str = " \u{2192} ";

    /// Shown in place of the theme list while none are installed.
    const NO_THEMES: &str = "no HyDE themes found on this machine";

    /// Rows the menu spends on the row naming the active theme.
    const ACTIVE_ROWS: f32 = 1.0;

    /// Sections the menu draws.
    const SECTION_COUNT: f32 = 1.0;

    /// Renders the menu against the desktop state last read from disk.
    ///
    /// `available_width` is how wide the menu may draw, so the chips wrap
    /// inside it instead of running past its edge.
    #[must_use]
    pub(super) fn view<'a>(
        state: &HydeState,
        swatches: &HashMap<String, ThemeSwatch>,
        switching: Option<&str>,
        catalogue: &[super::gallery::GalleryTheme],
        installing: Option<&str>,
        condemned: Option<&str>,
        updating: Option<&Option<String>>,
        list_layout: bool,
        spinner: Spinner,
        opacity: f32,
        font_size: f32,
        available_width: f32
    ) -> Element<'a, Message> {
        let busy_glyph = if switching.is_some() || updating.is_some() {
            Some(spinner.glyph())
        } else {
            None
        };
        let active = row_stack(font_size).push(status_row(
            ACTIVE,
            active_label(state, switching),
            busy_glyph,
            font_size
        ));

        let offered = offered_names(state, catalogue);
        let cell = shared_cell(state, &offered, list_layout, font_size, available_width);

        let mut window = page(font_size).push(active).push(section(
            THEMES,
            themes(
                state,
                swatches,
                switching,
                condemned,
                updating,
                installing,
                catalogue,
                spinner,
                opacity,
                font_size,
                available_width,
                cell,
                list_layout
            ),
            font_size
        ));

        if !offered.is_empty() {
            window = window.push(section(
                GALLERY,
                offer(
                    state,
                    catalogue,
                    switching,
                    installing,
                    spinner,
                    opacity,
                    font_size,
                    available_width,
                    cell,
                    list_layout
                ),
                font_size
            ));
        }

        window.into()
    }

    /// Names the gallery offers that are not installed yet.
    ///
    /// Matched through [`same_theme`], not equality: the catalogue spells
    /// names with dashes where installed directories carry spaces, and a
    /// literal comparison left every installed theme in the gallery, one
    /// press away from downloading itself again.
    pub(super) fn offered_names(
        state: &HydeState,
        catalogue: &[super::gallery::GalleryTheme]
    ) -> Vec<String> {
        catalogue
            .iter()
            .filter(|entry| {
                !state
                    .themes
                    .iter()
                    .any(|installed| same_theme(installed, &entry.name))
            })
            .map(|entry| entry.name.clone())
            .collect()
    }

    /// One card width for both sections, whatever the layout.
    ///
    /// The two grids used to size their cells from their own names and came
    /// out unequal; sizing from every name at once is what makes an installed
    /// card and an available one the same card.
    fn shared_cell(
        state: &HydeState,
        offered: &[String],
        list_layout: bool,
        font_size: f32,
        available_width: f32
    ) -> f32 {
        if list_layout {
            return available_width;
        }

        let mut names = state.themes.clone();
        names.extend(offered.iter().cloned());

        chip_cell_width(&names, font_size)
    }

    /// Rows of card indices for the layout in force.
    fn card_rows(
        names: &[String],
        list_layout: bool,
        font_size: f32,
        available_width: f32
    ) -> Vec<Vec<usize>> {
        if list_layout {
            (0..names.len()).map(|index| vec![index]).collect()
        } else {
            wrap_chips_into_rows(
                names,
                available_width,
                font_size,
                style::group_gap(font_size)
            )
        }
    }

    /// Whether two spellings name one theme.
    ///
    /// Dashes and underscores stand in for spaces across the gallery, its
    /// branches and the installed directories, and case drifts between them.
    pub(super) fn same_theme(a: &str, b: &str) -> bool {
        let canon = |name: &str| {
            name.chars()
                .map(|c| match c {
                    '-' | '_' => ' ',
                    other => other.to_ascii_lowercase()
                })
                .collect::<String>()
        };

        canon(a) == canon(b)
    }

    /// Renders the gallery as a grid of chips painted in announced colours.
    ///
    /// One install at a time, and none beside a running switch: a chip being
    /// installed carries the spinner, and every other chip waits unpressable
    /// rather than starting a second writer over the same directories.
    fn offer<'a>(
        state: &HydeState,
        catalogue: &[super::gallery::GalleryTheme],
        switching: Option<&str>,
        installing: Option<&str>,
        spinner: Spinner,
        opacity: f32,
        font_size: f32,
        available_width: f32,
        cell: f32,
        list_layout: bool
    ) -> Element<'a, Message> {
        let names = offered_names(state, catalogue);
        let busy = switching.is_some() || installing.is_some();
        let mut block = grid(font_size);

        for indices in card_rows(&names, list_layout, font_size, available_width) {
            let mut row = group(font_size);

            for index in indices {
                let name = &names[index];
                let entry = catalogue.iter().find(|entry| &entry.name == name);

                let chip_state = if installing == Some(name.as_str()) {
                    ThemeChip::Applying(spinner)
                } else if busy {
                    ThemeChip::Blocked
                } else {
                    ThemeChip::Idle
                };

                row = row.push(theme_chip(
                    name.clone(),
                    Message::Install(name.clone()),
                    chip_state,
                    font_size,
                    opacity,
                    cell,
                    entry.map(offer_paint),
                    vec![(
                        Icons::Download.default_glyph(),
                        Message::Install(name.clone()),
                        !busy
                    )]
                ));
            }

            block = block.push(row);
        }

        block.into()
    }

    /// Paint for a gallery chip, from the two colours the index announces.
    ///
    /// The index does not promise an order, so the darker of the two is taken
    /// as the surface and the lighter as the ink — a chip the other way round
    /// would be a swatch nobody can read.
    fn offer_paint(entry: &super::gallery::GalleryTheme) -> ChipPaint {
        let luma = |color: iced::Color| 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;

        let [first, second] = entry.colors;
        let (surface, ink) = if luma(first) <= luma(second) {
            (first, second)
        } else {
            (second, first)
        };

        ChipPaint {
            background: surface,
            text:       ink,
            accent:     ink,
            palette:    entry.colors.to_vec()
        }
    }

    /// Renders the installed themes as a grid of chips.
    ///
    /// The theme in force is drawn as picked, so the grid doubles as the answer
    /// to "which one am I on" without the menu repeating the name twice.
    ///
    /// While a switch runs the grid stops being a set of choices: the theme
    /// being applied is the only one lit, and every other chip is dimmed
    /// and carries no press at all. A second switch started on top of the
    /// first would race it over the state file, the wallpaper cache and
    /// every generated stylesheet, and the module refuses one anyway — a
    /// grid that still looked pressable would only be hiding that refusal
    /// until after the click.
    fn themes<'a>(
        state: &HydeState,
        swatches: &HashMap<String, ThemeSwatch>,
        switching: Option<&str>,
        condemned: Option<&str>,
        updating: Option<&Option<String>>,
        installing: Option<&str>,
        catalogue: &[super::gallery::GalleryTheme],
        spinner: Spinner,
        opacity: f32,
        font_size: f32,
        available_width: f32,
        cell: f32,
        list_layout: bool
    ) -> Element<'a, Message> {
        if state.themes.is_empty() {
            return note(NO_THEMES, font_size);
        }

        let busy = switching.is_some() || installing.is_some() || updating.is_some();
        let mut block =
            grid(font_size).push(update_all_row(busy, list_layout, font_size, opacity));

        for indices in card_rows(&state.themes, list_layout, font_size, available_width) {
            let mut row = group(font_size);

            for index in indices {
                let name = &state.themes[index];
                let doomed = condemned == Some(name.as_str());
                let fetching = matches!(updating, Some(Some(one)) if one == name);

                let (chip_state, press) = if doomed {
                    (ThemeChip::Condemned, Message::Remove(name.clone()))
                } else if fetching {
                    (ThemeChip::Applying(spinner), Message::Switch(name.clone()))
                } else {
                    (
                        chip_state(state, switching, spinner, name),
                        Message::Switch(name.clone())
                    )
                };

                let paint = swatches.get(name).map(chip_paint).or_else(|| {
                    catalogue
                        .iter()
                        .find(|entry| same_theme(&entry.name, name))
                        .map(offer_paint)
                });

                let trash = if doomed {
                    Message::Remove(name.clone())
                } else {
                    Message::Condemn(name.clone())
                };

                let chip = theme_chip(
                    name.clone(),
                    press,
                    chip_state,
                    font_size,
                    opacity,
                    cell,
                    paint,
                    vec![
                        (
                            Icons::Refresh.default_glyph(),
                            Message::Update(Some(name.clone())),
                            !busy
                        ),
                        (Icons::Trash.default_glyph(), trash, !busy || doomed),
                    ]
                );

                row = row.push(
                    iced::widget::mouse_area(chip).on_right_press(Message::Condemn(name.clone()))
                );
            }

            block = block.push(row);
        }

        block.into()
    }

    /// The row offering the one fetch that updates every installed theme.
    fn update_all_row<'a>(
        busy: bool,
        list_layout: bool,
        font_size: f32,
        opacity: f32
    ) -> Element<'a, Message> {
        use iced::widget::{Row, button};

        use crate::{
            components::{icons::icon_raw, scale},
            style::ghost_button_style
        };

        let control = style::control_size(font_size);
        let mut update = button(
            Row::new()
                .push(icon_raw(Icons::Refresh.default_glyph().to_owned()))
                .push(crate::components::text::text("Update all").size(control))
                .spacing(scale::icon_gap())
                .align_y(iced::Alignment::Center)
        )
        .padding(control * 0.25)
        .style(ghost_button_style(opacity));

        if !busy {
            update = update.on_press(Message::Update(None));
        }

        let layout_glyph = if list_layout {
            Icons::ViewGrid.default_glyph()
        } else {
            Icons::ViewList.default_glyph()
        };
        let layout = button(icon_raw(layout_glyph.to_owned()))
            .padding(control * 0.25)
            .style(ghost_button_style(opacity))
            .on_press(Message::ToggleLayout);

        Row::new()
            .push(update)
            .push(iced::widget::Space::new().width(iced::Length::Fill))
            .push(layout)
            .align_y(iced::Alignment::Center)
            .width(iced::Length::Fill)
            .into()
    }

    /// Restates a theme's swatch in the colours the renderer paints with.
    fn chip_paint(swatch: &ThemeSwatch) -> ChipPaint {
        ChipPaint {
            background: colour(swatch.background),
            text:       colour(swatch.text),
            accent:     colour(swatch.accent),
            palette:    swatch.palette.map(colour).to_vec()
        }
    }

    /// One palette colour, as the renderer spells it.
    fn colour(rgba: Rgba) -> iced::Color {
        iced::Color::from_rgba8(rgba.r, rgba.g, rgba.b, rgba.a)
    }

    /// What the chip of `name` stands for, given what the desktop reports and
    /// what the bar is waiting on.
    ///
    /// The theme being applied wins over the theme in force, because HyDE
    /// reports the new name in its state file long before the switch is
    /// anywhere near done: a grid that only looked at the state file would
    /// light the new chip within a moment of the press and then sit
    /// completely still for the seconds that actually matter.
    fn chip_state(
        state: &HydeState,
        switching: Option<&str>,
        spinner: Spinner,
        name: &str
    ) -> ThemeChip {
        match switching {
            Some(pending) if pending == name => ThemeChip::Applying(spinner),
            Some(_) => ThemeChip::Blocked,
            None if state.is_active(name) => ThemeChip::Active,
            None => ThemeChip::Idle
        }
    }

    /// Renders what the desktop is on, and what it is on its way to.
    ///
    /// The theme in force always comes from HyDE's own state file; a theme the
    /// bar asked for is drawn beside it rather than in its place, because
    /// until the switch has finished the desktop is still on the old one
    /// and a menu that already named the new one would be reporting
    /// something that may yet fail.
    fn active_label(state: &HydeState, switching: Option<&str>) -> String {
        let active = state.theme.as_deref().unwrap_or(UNKNOWN);

        match switching {
            Some(pending) if !state.is_active(pending) => {
                format!("{active}{SWITCHING_TO}{pending}")
            }
            _ => active.to_owned()
        }
    }

    /// Rows the theme grid fills when laid out `available_width` wide.
    fn theme_rows(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
        theme_rows_in(state, false, font_size, available_width)
    }

    /// Rows the installed grid fills in the layout in force.
    fn theme_rows_in(
        state: &HydeState,
        list_layout: bool,
        font_size: f32,
        available_width: f32
    ) -> f32 {
        if list_layout {
            return state.themes.len().max(1) as f32;
        }

        if state.themes.is_empty() {
            return 1.0;
        }

        wrap_chips_into_rows(
            &state.themes,
            available_width,
            font_size,
            style::group_gap(font_size)
        )
        .len() as f32
    }

    /// Rows this menu draws when laid out `available_width` wide, its heading
    /// counted in.
    #[must_use]
    pub(super) fn rows(state: &HydeState, font_size: f32, available_width: f32) -> f32 {
        SECTION_COUNT * style::SECTION_TITLE_ROWS
            + ACTIVE_ROWS
            + theme_rows(state, font_size, available_width)
    }

    /// Longest line the active row can ever grow to.
    ///
    /// While a switch runs the row names both themes, `active → pending`, and
    /// any installed theme can be the pending one. Reserving for the
    /// longest of those lines up front is what keeps the menu the same size
    /// through a press: a window that jumps wider the instant it is pressed
    /// jumps at the one moment the user is looking straight at it.
    fn widest_active_row(state: &HydeState, switching: Option<&str>, font_size: f32) -> f32 {
        let shown = status_row_width(&active_label(state, switching), font_size);
        let active = state.theme.as_deref().unwrap_or(UNKNOWN);

        state
            .themes
            .iter()
            .filter(|name| !state.is_active(name))
            .map(|name| status_row_width(&format!("{active}{SWITCHING_TO}{name}"), font_size))
            .fold(shown, f32::max)
    }

    /// Longest line of this menu, which is how wide it has to be.
    ///
    /// The grid is measured as a row of the widest themes rather than as the
    /// whole list: it wraps into whatever width the menu settles on, and
    /// sizing the menu to hold every theme side by side would make it far
    /// wider than the screen.
    ///
    /// Room for the live indicator and for the longest possible switch line is
    /// reserved whether a switch is running or not, so starting one moves
    /// nothing — see [`widest_active_row`].
    #[must_use]
    pub(super) fn desired_width(
        state: &HydeState,
        switching: Option<&str>,
        font_size: f32
    ) -> f32 {
        let active = widest_active_row(state, switching, font_size) + indicator_width(font_size);

        let control = style::control_size(font_size);
        let gap = style::group_gap(font_size);

        let grid = if state.themes.is_empty() {
            text_width(NO_THEMES, style::caption_size(font_size))
        } else {
            let widest = state
                .themes
                .iter()
                .map(|name| chip_width(name, control))
                .fold(0.0_f32, f32::max);

            widest * THEMES_PER_ROW + gap * (THEMES_PER_ROW - 1.0)
        };

        active.max(grid)
    }

    /// Height this menu needs when drawn `available_width` wide.
    ///
    /// A painted chip is taller than a plain one by its row of palette dots, so
    /// every grid row adds that height on top of the shared row pitch — an
    /// estimate without it would clip the last row of themes.
    #[must_use]
    pub(super) fn desired_height(
        state: &HydeState,
        offered: &[String],
        list_layout: bool,
        font_size: f32,
        available_width: f32
    ) -> f32 {
        let offered_rows = if offered.is_empty() {
            0.0
        } else {
            card_rows(offered, list_layout, font_size, available_width).len() as f32
        };
        let offered_sections = if offered.is_empty() { 0.0 } else { 1.0 };

        let chip_rows =
            theme_rows_in(state, list_layout, font_size, available_width) + offered_rows;
        let control = style::control_size(font_size);
        let actions = chip_rows * ACTIONS_ROW_EM * control + UPDATE_ALL_ROW_EM * control;
        let dots = chip_rows * widgets::DOT_ROW_EM * control + actions;

        style::page_height(
            rows(state, font_size, available_width)
                + offered_rows
                + offered_sections * style::SECTION_TITLE_ROWS,
            font_size
        ) + dots
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn state(themes: &[&str], active: Option<&str>) -> HydeState {
            HydeState {
                theme:            active.map(str::to_owned),
                themes:           themes.iter().map(|name| (*name).to_owned()).collect(),
                wallpaper_colors: true,
                shader:           Some("wallbash".to_owned())
            }
        }

        #[test]
        fn a_menu_that_is_not_switching_names_the_theme_in_force() {
            let state = state(&["Nord", "Mocha"], Some("Nord"));

            assert_eq!(active_label(&state, None), "Nord");
        }

        #[test]
        fn a_running_switch_names_both_themes_and_keeps_the_old_one_first() {
            let state = state(&["Nord", "Mocha"], Some("Nord"));

            let label = active_label(&state, Some("Mocha"));

            assert!(label.starts_with("Nord"), "{label}");
            assert!(label.ends_with("Mocha"), "{label}");
        }

        /// HyDE writes the new name into its state file long before the switch
        /// is over, so the menu has to stop drawing an arrow that
        /// points at the theme it already reports.
        #[test]
        fn a_switch_the_state_file_already_reports_is_named_only_once() {
            let state = state(&["Nord", "Mocha"], Some("Mocha"));

            assert_eq!(active_label(&state, Some("Mocha")), "Mocha");
        }

        #[test]
        fn a_desktop_without_a_theme_still_names_the_one_being_switched_to() {
            let state = state(&["Nord"], None);

            assert_eq!(
                active_label(&state, Some("Nord")),
                format!("{UNKNOWN}{SWITCHING_TO}Nord")
            );
        }

        #[test]
        fn a_menu_nobody_is_waiting_on_marks_the_theme_in_force_and_offers_the_rest() {
            let state = state(&["Nord", "Mocha"], Some("Nord"));
            let spinner = Spinner::default();

            assert_eq!(chip_state(&state, None, spinner, "Nord"), ThemeChip::Active);
            assert_eq!(chip_state(&state, None, spinner, "Mocha"), ThemeChip::Idle);
        }

        #[test]
        fn a_running_switch_marks_the_theme_it_is_applying() {
            let state = state(&["Nord", "Mocha"], Some("Nord"));
            let spinner = Spinner::default();

            assert_eq!(
                chip_state(&state, Some("Mocha"), spinner, "Mocha"),
                ThemeChip::Applying(spinner)
            );
        }

        #[test]
        fn no_chip_can_start_a_second_switch_while_one_runs() {
            let state = state(&["Nord", "Mocha", "Latte"], Some("Nord"));
            let spinner = Spinner::default();

            for name in &state.themes {
                assert!(
                    !chip_state(&state, Some("Mocha"), spinner, name).is_pressable(),
                    "{name}"
                );
            }
        }

        /// The one case the state file cannot settle: HyDE records the new
        /// theme within a moment of the press and the switch runs for
        /// seconds afterwards, so a grid that trusted the file would go
        /// still exactly when the user is waiting hardest.
        #[test]
        fn the_theme_being_applied_stays_marked_once_the_state_file_names_it() {
            let state = state(&["Nord", "Mocha"], Some("Mocha"));
            let spinner = Spinner::default();

            assert_eq!(
                chip_state(&state, Some("Mocha"), spinner, "Mocha"),
                ThemeChip::Applying(spinner)
            );
        }

        #[test]
        fn the_marked_chip_keeps_moving_as_the_indicator_advances() {
            let state = state(&["Nord", "Mocha"], Some("Nord"));
            let mut spinner = Spinner::default();
            let first = chip_state(&state, Some("Mocha"), spinner, "Mocha");

            spinner.advance();

            assert_ne!(chip_state(&state, Some("Mocha"), spinner, "Mocha"), first);
        }

        #[test]
        fn a_menu_without_themes_still_asks_for_a_width() {
            assert!(desired_width(&HydeState::default(), None, 16.0) > 0.0);
        }

        #[test]
        fn a_menu_without_themes_reserves_one_row_for_the_notice() {
            assert_eq!(theme_rows(&HydeState::default(), 16.0, 400.0), 1.0);
        }

        #[test]
        fn a_long_theme_name_widens_the_menu() {
            let short = state(&["Nord"], Some("Nord"));
            let long = state(&["An Extremely Long Theme Name"], Some("Nord"));

            assert!(desired_width(&long, None, 16.0) > desired_width(&short, None, 16.0));
        }

        #[test]
        fn more_themes_than_a_row_holds_make_the_menu_taller() {
            let few = state(&["Nord", "Mocha"], Some("Nord"));
            let many = state(
                &[
                    "Nord",
                    "Mocha",
                    "Latte",
                    "Decay Green",
                    "Edge Runner",
                    "Synth Wave",
                    "Tokyo Night",
                    "Gruvbox Retro"
                ],
                Some("Nord")
            );
            let width = desired_width(&few, None, 16.0);

            assert!(
                desired_height(&many, &[], false, 16.0, width)
                    > desired_height(&few, &[], false, 16.0, width)
            );
        }

        #[test]
        fn every_theme_lands_in_exactly_one_row() {
            let themes = state(
                &["Nord", "Mocha", "Latte", "Decay Green", "Edge Runner"],
                None
            );
            let rows = wrap_chips_into_rows(&themes.themes, 200.0, 16.0, 8.0);

            assert_eq!(
                rows.iter().map(Vec::len).sum::<usize>(),
                themes.themes.len()
            );
        }

        #[test]
        fn a_menu_showing_an_indicator_is_no_wider_than_one_that_is_not() {
            let state = state(&["Nord", "Mocha"], Some("Nord"));

            assert_eq!(
                desired_width(&state, Some("Nord"), 16.0),
                desired_width(&state, None, 16.0)
            );
        }

        #[test]
        fn the_menu_reserves_the_indicator_before_anything_is_running() {
            let state = state(&["A Theme Long Enough To Set The Width"], None);
            let font_size = 16.0;

            assert!(
                desired_width(&state, None, font_size)
                    >= status_row_width(&active_label(&state, None), font_size)
                        + indicator_width(font_size)
            );
        }

        /// Starting a switch must not move the menu: the width already holds
        /// the longest line the active row can become.
        #[test]
        fn starting_a_switch_changes_no_size() {
            let themes = state(
                &["Nord", "A Theme With A Very Long Name Indeed"],
                Some("Nord")
            );
            let font_size = 16.0;

            let resting = desired_width(&themes, None, font_size);
            let switching = desired_width(
                &themes,
                Some("A Theme With A Very Long Name Indeed"),
                font_size
            );

            assert_eq!(resting, switching);
            assert_eq!(
                desired_height(&themes, &[], false, font_size, resting),
                desired_height(&themes, &[], false, font_size, switching)
            );
        }

        #[test]
        fn the_menu_reserves_a_row_for_every_line_it_draws() {
            let themes = state(&["Nord"], Some("Nord"));
            let font_size = 16.0;
            let width = desired_width(&themes, None, font_size);

            assert_eq!(
                rows(&themes, font_size, width),
                SECTION_COUNT * style::SECTION_TITLE_ROWS
                    + ACTIVE_ROWS
                    + theme_rows(&themes, font_size, width)
            );
        }

        #[test]
        fn the_menu_height_counts_dots_actions_and_the_update_row() {
            let font_size = 16.0;
            let themes = state(&["Nord", "Mocha"], Some("Nord"));
            let control = style::control_size(font_size);
            let chip_rows = theme_rows(&themes, font_size, 400.0);

            assert_eq!(
                desired_height(&themes, &[], false, font_size, 400.0),
                style::page_height(rows(&themes, font_size, 400.0), font_size)
                    + chip_rows * widgets::DOT_ROW_EM * control
                    + chip_rows * ACTIONS_ROW_EM * control
                    + UPDATE_ALL_ROW_EM * control
            );
        }
    }
}

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
    /// Deliver the swatches the themes announce themselves with.
    ///
    /// Raised by the reader the menu starts when it opens; the colours arrive
    /// a beat after the names because reading them hashes every theme's
    /// wallpaper, which is nothing the opening animation should wait on.
    SwatchesLoaded(HashMap<String, ThemeSwatch>),
    /// Deliver the upstream catalogue the gallery section draws.
    CatalogueLoaded(Vec<gallery::GalleryTheme>),
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

/// Reads the swatch of every named theme from the HyDE install on disk.
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
    hyde:        HydeState,
    /// The colours each theme announces itself with, by theme name.
    ///
    /// Loaded off the update path — see [`Themes::load_swatches`] — and kept
    /// so the menu can paint every chip in the colours of the theme it stands
    /// for. A theme without an entry is painted like any other control.
    swatches:    HashMap<String, ThemeSwatch>,
    /// Theme a switch is running for, while one is.
    switching:   Option<String>,
    /// The upstream catalogue, once the menu has loaded it.
    catalogue:   Vec<gallery::GalleryTheme>,
    /// Theme an install is running for, while one is.
    installing:  Option<String>,
    /// Theme whose removal waits for its confirming press, if one does.
    condemned:   Option<String>,
    /// Theme an update is fetching, while one is; `None` name means all.
    updating:    Option<Option<String>>,
    /// Whether the window lays cards out as one column instead of a grid.
    list_layout: bool,
    /// Frame the indicator of a running switch is on.
    ///
    /// Advanced on a tick rather than derived from a clock read while drawing,
    /// so what the bar shows is a function of the state it holds and can be
    /// checked without one.
    spinner:     Spinner
}

impl Themes {
    /// Creates the module against the desktop state on disk.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hyde:        hyde_state::load(),
            swatches:    HashMap::new(),
            switching:   None,
            catalogue:   Vec::new(),
            installing:  None,
            condemned:   None,
            updating:    None,
            list_layout: false,
            spinner:     Spinner::default()
        }
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
                tokio::task::spawn_blocking(move || read_swatches(&themes))
                    .await
                    .unwrap_or_default()
            },
            Message::SwatchesLoaded
        )
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
        if self.installing.is_some() || self.updating.is_some() {
            return true;
        }

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
            self.switching(),
            &self.catalogue,
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
        let page_width = width - page::metrics::ROW_SLACK_EM * font_size;

        crate::menu::MenuMetrics {
            width,
            page_width,
            height: view::desired_height(
                &self.hyde,
                &view::offered_names(&self.hyde, &self.catalogue),
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

        view::desired_width(&self.hyde, self.switching(), font_size)
            + page::metrics::ROW_SLACK_EM * font_size
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
            Message::Switch(theme) => return self.switch(theme, config),
            Message::Switched {
                theme,
                failure
            } => {
                self.switched(&theme, failure.as_deref(), config);

                return self.load_swatches();
            }
            Message::Tick => {
                if self.switching.is_some() || self.installing.is_some() || self.updating.is_some()
                {
                    self.spinner.advance();
                }
            }
            Message::SwatchesLoaded(swatches) => self.swatches = swatches,
            Message::CatalogueLoaded(catalogue) => {
                self.catalogue = catalogue;
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
                if self.switching.is_none()
                    && self.installing.is_none()
                    && self.hyde.theme.as_deref() != Some(theme.as_str())
                    && self.hyde.themes.contains(&theme)
                {
                    self.condemned = Some(theme);
                }
            }
            Message::Remove(theme) => return self.remove(theme, config),
            Message::Removed {
                theme,
                failure
            } => {
                if let Some(failure) = failure {
                    report(
                        config,
                        &format!("removing the HyDE theme `{theme}` failed: {failure}")
                    );
                } else {
                    info!("the HyDE theme `{theme}` is removed");
                }

                self.refresh();

                return self.load_swatches();
            }
            Message::Installed {
                theme,
                failure
            } => {
                self.installing = None;

                match failure {
                    Some(failure) => {
                        report(
                            config,
                            &format!("installing the HyDE theme `{theme}` failed: {failure}")
                        );
                    }
                    None => {
                        info!("the HyDE theme `{theme}` is installed, switching to it");
                        self.refresh();

                        return Task::batch([self.load_swatches(), self.switch(theme, config)]);
                    }
                }
            }
        }

        Task::none()
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
                theme: theme.clone(),
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

        info!("removing the HyDE theme `{theme}` at {directory:?}");

        Task::perform(
            async move {
                tokio::fs::remove_dir_all(directory)
                    .await
                    .err()
                    .map(|error| error.to_string())
            },
            move |failure| Message::Removed {
                theme: theme.clone(),
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

        let command = match &scope {
            Some(theme) => format!(
                "hyde-shell theme.import --fetch '{}'",
                theme.replace('\'', "'\\''")
            ),
            None => "hyde-shell theme.import --fetch all".to_owned()
        };

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
        const STAMP_LIFE: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

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
    pub fn load_catalogue(&self) -> Task<Message> {
        Task::perform(gallery::load(), Message::CatalogueLoaded)
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
