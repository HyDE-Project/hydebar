//! The desk surface: the shape the bar takes when the screen is bare.
//!
//! Not a second bar and not a second set of readouts — the same modules, in
//! the same layout, come down off the strip and stand over the wallpaper at a
//! size a room away can read. The three sections of the layout become three
//! columns, each keeping its own order and its own groups, and each module
//! keeps the presses it answers to on the strip.
//!
//! One folder, one room so far: [`column`] stacks a section into a column.

mod blocks;
mod column;
mod face;
mod readings;

use hydebar_core::outputs::HasOutput;
use iced::{
    Alignment, Element, Length, Padding, SurfaceId as Id,
    widget::{Row, container}
};

use super::super::state::{App, Message};

impl App {
    /// Draws the unfolded bar of one output, or nothing while a window holds
    /// the screen.
    ///
    /// The canvas unfolds rather than appears: it fades in over the wallpaper
    /// and rises the last stretch into place while the strip fades out under
    /// it, and folds back the same way the moment a window maps. The travel is
    /// the screen's own spring, so a second monitor still holding a window is
    /// untouched by it.
    pub(super) fn desk_surface(&self, id: Id) -> Element<'_, Message> {
        let screen = self.outputs.screen_of(id).flatten();

        if !self.desk_holds(screen) {
            return Row::new().into();
        }

        let presence = self.desk_presence(screen);

        let ink = blocks::Ink {
            value: self.theme_cache.palette().text,
            size:  self.appearance().font_size_px()
        };
        let margin = ink.size * 2.0;
        let modules = &self.config.modules;

        let columns = [
            (&modules.left, blocks::Side::Leading),
            (&modules.center, blocks::Side::Leading),
            (&modules.right, blocks::Side::Trailing)
        ]
        .into_iter()
        .filter_map(|(section, side)| self.desk_column(section, id, side, ink, presence));

        let canvas = container(
            Row::with_children(columns)
                .spacing(margin)
                .align_y(Alignment::Start)
                .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top:    margin * presence,
            right:  margin,
            bottom: margin,
            left:   margin
        });

        canvas.into()
    }

    /// How much larger than the strip the canvas of `id` is drawn.
    ///
    /// One for every surface but the desk: the modules are drawn from the
    /// same views the strip uses, and the whole surface is magnified instead,
    /// which is what lets one layout serve a thirty pixel strip and a whole
    /// screen without a second set of sizes.
    pub(crate) fn desk_magnification(&self, id: Id) -> f64 {
        if !self.config.desk.enabled || !matches!(self.outputs.has(id), Some(HasOutput::Desk)) {
            return 1.0;
        }

        f64::from(self.config.desk.magnification())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_core::animation::GENTLE;
    use iced_test::simulator;

    use super::{
        super::super::state::{App, test_support::test_app_with},
        *
    };

    /// The bar with the desk switched on, unfolded as far as `presence`.
    ///
    /// The spring is snapped rather than animated: a test asserting what the
    /// canvas draws must not also wait for a frame clock.
    fn unfolded(presence: f32) -> App {
        let mut app = test_app_with(|config| config.desk.enabled = true);

        if presence > 0.0 {
            app.desk_fades.point(None, true, false, GENTLE);
        }

        app
    }

    fn surface() -> Id {
        Id::unique()
    }

    #[test]
    fn a_bare_screen_unfolds_the_bar_into_its_blocks() {
        let app = unfolded(1.0);
        let mut ui = simulator(app.desk_surface(surface()));

        assert!(
            ui.find("MEMORY").is_ok(),
            "the memory block stands on the canvas"
        );
        assert!(
            ui.find("PROCESSOR").is_ok(),
            "the processor block stands on the canvas"
        );
    }

    #[test]
    fn a_screen_holding_a_window_draws_no_canvas() {
        let app = unfolded(0.0);
        let mut ui = simulator(app.desk_surface(surface()));

        assert!(
            ui.find("MEMORY").is_err(),
            "nothing is drawn while a window holds the screen"
        );
    }

    #[test]
    fn the_desk_switched_off_draws_nothing_however_bare_the_screen() {
        let mut app = test_app_with(|config| config.desk.enabled = false);
        app.desk_fades.point(None, true, false, GENTLE);

        let mut ui = simulator(app.desk_surface(surface()));

        assert!(ui.find("MEMORY").is_err());
    }

    #[test]
    fn the_strip_empties_the_moment_its_blocks_leave_it() {
        let app = unfolded(1.0);
        let mut ui = simulator(app.bar_surface(surface()));

        assert!(
            ui.snapshot(&iced::Theme::Dark).is_ok(),
            "the strip still draws, empty"
        );
    }

    #[test]
    fn the_clock_stands_over_the_month_it_opens() {
        let app = unfolded(1.0);
        let month = app.clock.data().format("%B %Y");
        let mut ui = simulator(app.desk_surface(surface()));

        assert!(
            ui.find(month.as_str()).is_ok(),
            "the month grid stands under the hour"
        );
    }

    /// The two probes that tell which shape holds the screen.
    ///
    /// One text apiece, each drawn by one shape only: a block heading stands
    /// on the canvas alone, and the strip states the hour in the format the
    /// configuration named, which the canvas never uses.
    fn shapes_on_screen(app: &App) -> (bool, bool) {
        let hour = app.clock.data().format(&app.config.clock.format);

        let mut strip = simulator(app.bar_surface(surface()));
        let mut canvas = simulator(app.desk_surface(surface()));

        (
            strip.find(hour.as_str()).is_ok(),
            canvas.find("MEMORY").is_ok()
        )
    }

    #[test]
    fn one_shape_holds_the_screen_at_every_step_of_the_travel() {
        let mut app = test_app_with(|config| config.desk.enabled = true);

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(strip && !canvas, "the strip holds a screen with a window");

        app.desk_fades.point(None, true, true, GENTLE);

        for frame in 0..64 {
            let (strip, canvas) = shapes_on_screen(&app);

            assert!(
                strip != canvas,
                "frame {frame}: strip {strip}, canvas {canvas} — one shape, never two and never none"
            );

            if !app.desk_fades.advance(std::time::Duration::from_millis(16)) {
                break;
            }
        }

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(!strip && canvas, "the canvas holds a bare screen");
    }

    #[test]
    fn the_strip_is_back_on_the_very_frame_a_window_takes_the_screen() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        app.desk_fades.point(None, true, false, GENTLE);

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(!strip && canvas, "the canvas holds the bare screen");

        app.desk_fades.point(None, false, false, GENTLE);

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(
            strip && !canvas,
            "the strip is back with the window, not travelling behind it"
        );
    }

    #[test]
    fn the_strip_takes_the_screen_back_the_moment_the_travel_ends() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        app.desk_fades.point(None, true, false, GENTLE);
        app.desk_fades.point(None, false, true, GENTLE);

        for frame in 0..64 {
            let (strip, canvas) = shapes_on_screen(&app);

            assert!(
                strip != canvas,
                "frame {frame} of the way back shows one shape"
            );

            if !app.desk_fades.advance(std::time::Duration::from_millis(16)) {
                break;
            }
        }

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(
            strip && !canvas,
            "the strip is back once the blocks are home"
        );
    }

    #[test]
    fn every_column_of_the_layout_reaches_the_canvas() {
        let app = unfolded(1.0);
        let mut ui = simulator(app.desk_surface(surface()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn a_module_of_the_right_hand_section_states_its_own_block() {
        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.right = vec![hydebar_proto::config::ModuleDef::Single(
                hydebar_core::config::ModuleName::CpuTemp
            )];
        });
        app.desk_fades.point(None, true, false, GENTLE);

        let mut ui = simulator(app.desk_surface(surface()));

        assert!(
            ui.find("CPU TEMPERATURE").is_ok(),
            "the right hand section carries a block of its own"
        );
    }

    #[test]
    fn a_module_with_nothing_longer_to_say_still_stands_on_the_canvas() {
        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = vec![hydebar_proto::config::ModuleDef::Single(
                hydebar_core::config::ModuleName::Clock
            )];
            config.modules.center = Vec::new();
            config.modules.right = vec![hydebar_proto::config::ModuleDef::Single(
                hydebar_core::config::ModuleName::AppLauncher
            )];
        });
        app.desk_fades.point(None, true, false, GENTLE);

        let mut ui = simulator(app.desk_surface(surface()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }
}
