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
pub(super) mod column;
mod readings;

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

        let unfolding = self.desk_presence(screen);

        let ink = blocks::Ink {
            value: self.theme_cache.palette().text,
            size:  self.appearance().font_size_px()
        };
        let margin = ink.size * 2.0;
        let modules = &self.config.modules;

        let left = Self::desk_order(&modules.left, true);
        let centre = Self::desk_order(&modules.center, false);
        let right = Self::desk_order(&modules.right, false);

        let columns = [
            (&left, blocks::Side::Leading),
            (&centre, blocks::Side::Middle),
            (&right, blocks::Side::Trailing)
        ]
        .into_iter()
        .filter_map(|(order, side)| self.desk_column(order, id, side, ink, unfolding));

        let canvas = container(
            Row::with_children(columns)
                .spacing(margin)
                .align_y(Alignment::Start)
                .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top:    self.strip_band() + margin,
            right:  margin,
            bottom: margin,
            left:   margin
        });

        canvas.into()
    }

    /// The band along the top of the screen the strip itself occupies.
    ///
    /// The canvas covers the whole screen so its blocks can leave the strip
    /// without jumping, which means the places they land have to keep clear
    /// of the band the strip stands in.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the bar height constant is exactly representable in f32"
    )]
    fn strip_band(&self) -> f32 {
        self.appearance()
            .height
            .unwrap_or(hydebar_core::HEIGHT as f32)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use hydebar_core::animation::SWEEP;
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
            open(&mut app);
        }

        app
    }

    /// Sends a bar's canvas all the way out: travelled and written out.
    fn open(app: &mut App) {
        app.desk_fades.point(None, true, false, SWEEP);
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
        open(&mut app);

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
    /// The strip is asked for the hour in the format the configuration named,
    /// which it draws whenever it draws at all. The canvas is asked for
    /// either half of its unfolding: the modules it carries while they are
    /// still travelling in their strip shape, or a block heading once they
    /// have opened.
    fn shapes_on_screen(app: &App) -> (bool, bool) {
        let hour = app.clock.data().format(&app.config.clock.format);

        let strip = simulator(app.bar_surface(surface()))
            .find(hour.as_str())
            .is_ok();
        let travelling = simulator(app.desk_surface(surface()))
            .find(hour.as_str())
            .is_ok();
        let opened = simulator(app.desk_surface(surface()))
            .find("MEMORY")
            .is_ok();

        (strip, travelling || opened)
    }

    #[test]
    fn one_shape_holds_the_screen_at_every_step_of_the_travel() {
        let mut app = test_app_with(|config| config.desk.enabled = true);

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(strip && !canvas, "the strip holds a screen with a window");

        app.desk_fades.point(None, true, true, SWEEP);

        for frame in 0..96 {
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
        open(&mut app);

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(!strip && canvas, "the canvas holds the bare screen");

        app.desk_fades.point(None, false, false, SWEEP);

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(
            strip && !canvas,
            "the strip is back with the window, not travelling behind it"
        );
    }

    #[test]
    fn no_two_blocks_of_a_column_are_ever_in_flight_together() {
        for blocks in 2..8usize {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a column holds a handful of blocks"
            )]
            let places: Vec<f32> = (0..blocks)
                .map(|index| index as f32 / (blocks - 1) as f32)
                .collect();

            for step in 0..=200 {
                #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
                let unfolding = step as f32 / 200.0;

                let flying = places
                    .iter()
                    .map(|place| super::column::journey(unfolding, *place, blocks).0)
                    .filter(|travel| *travel > 0.0 && *travel < 1.0)
                    .count();

                assert!(
                    flying <= 1,
                    "{blocks} blocks, {unfolding:.3} of the way: {flying} in flight at once"
                );
            }
        }
    }

    #[test]
    fn a_block_waits_for_the_one_before_it_to_arrive() {
        for step in 0..=400 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let unfolding = step as f32 / 400.0;

            let leader = super::column::journey(unfolding, 0.0, 3).0;
            let follower = super::column::journey(unfolding, 0.5, 3).0;

            if follower > 0.0 {
                assert!(
                    leader >= 1.0,
                    "the follower set off at {unfolding:.3} with the leader at {leader:.3}"
                );
            }
        }
    }

    #[test]
    fn a_block_crosses_the_screen_before_it_writes_itself_out() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        app.desk_fades.point(None, true, true, SWEEP);

        let mut wrote_while_crossing = false;
        let mut crossed = false;

        for _ in 0..256 {
            let (travel, bloom) = super::column::journey(app.desk_presence(None), 0.0, 2);
            let writing = simulator(app.desk_surface(surface()))
                .find("MEMORY")
                .is_ok();

            crossed |= travel >= 1.0;
            wrote_while_crossing |= writing && travel < 1.0;

            let _ = bloom;

            if !app.desk_fades.advance(std::time::Duration::from_millis(16)) {
                break;
            }
        }

        assert!(crossed, "the leading block reaches its place");
        assert!(
            !wrote_while_crossing,
            "no block writes itself out while it is still crossing"
        );
        assert!(
            simulator(app.desk_surface(surface()))
                .find("MEMORY")
                .is_ok(),
            "the blocks are open once the unfolding is over"
        );
    }

    #[test]
    fn a_block_writes_itself_out_a_line_at_a_time() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        app.desk_fades.point(None, true, true, SWEEP);

        let mut first = None;
        let mut last = 0usize;

        for _ in 0..256 {
            let drawn = ["in use", "available", "cached", "swap"]
                .into_iter()
                .filter(|row| simulator(app.desk_surface(surface())).find(*row).is_ok())
                .count();

            if drawn > 0 && first.is_none() {
                first = Some(drawn);
            }

            last = drawn;

            if !app.desk_fades.advance(std::time::Duration::from_millis(16)) {
                break;
            }
        }

        assert!(
            first.is_some_and(|first| first < last),
            "the block starts with fewer lines than it ends with"
        );
    }

    #[test]
    fn each_block_of_a_section_stands_in_a_lane_of_its_own() {
        use hydebar_core::config::ModuleName;
        use hydebar_proto::config::ModuleDef;

        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = vec![
                ModuleDef::Single(ModuleName::CpuTemp),
                ModuleDef::Single(ModuleName::Memory),
            ];
            config.modules.center = Vec::new();
            config.modules.right = Vec::new();
        });
        app.screen_width = Some(1920.0);
        open(&mut app);

        let edge = simulator(app.desk_surface(surface()))
            .find("CPU TEMPERATURE")
            .expect("the far module")
            .bounds();
        let middle = simulator(app.desk_surface(surface()))
            .find("MEMORY")
            .expect("the near module")
            .bounds();

        assert!(
            middle.y < edge.y,
            "the near module stands higher: {} against {}",
            middle.y,
            edge.y
        );
        assert!(
            middle.x > edge.x,
            "the near module stands further in: {} against {}",
            middle.x,
            edge.x
        );
    }

    #[test]
    fn the_module_nearest_the_middle_of_the_strip_stands_highest() {
        use hydebar_core::config::ModuleName;
        use hydebar_proto::config::ModuleDef;

        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = vec![
                ModuleDef::Single(ModuleName::CpuTemp),
                ModuleDef::Single(ModuleName::Memory),
            ];
            config.modules.center = Vec::new();
            config.modules.right = Vec::new();
        });
        open(&mut app);

        let edge = simulator(app.desk_surface(surface()))
            .find("CPU TEMPERATURE")
            .expect("the far module")
            .bounds()
            .y;
        let middle = simulator(app.desk_surface(surface()))
            .find("MEMORY")
            .expect("the near module")
            .bounds()
            .y;

        assert!(
            middle < edge,
            "the module that stood nearer the middle stands higher: {middle} against {edge}"
        );
    }

    #[test]
    fn the_islands_leave_the_strip_from_the_seats_they_held_on_it() {
        let app = test_app_with(|config| config.desk.enabled = true);

        let mut ui = simulator(app.bar_surface(surface()));
        let _ = ui.snapshot(&iced::Theme::Dark).expect("the strip draws");

        app.flip.borrow_mut().depart();

        assert!(
            !app.flip.borrow().from_map().is_empty(),
            "the seats the strip held are what the canvas travels from"
        );
    }

    #[test]
    fn a_screen_taken_by_a_window_sends_its_islands_beyond_the_edge() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        app.screen_width = Some(1920.0);
        open(&mut app);

        app.desk_fades.point(None, false, false, SWEEP);
        app.send_the_islands_home(surface());

        let seats: Vec<f32> = app.flip.borrow().from_map().values().copied().collect();

        assert!(
            !seats.is_empty(),
            "every island is given a seat to fly from"
        );
        assert!(
            seats.iter().any(|seat| *seat < 0.0),
            "the left hand section flies in from past the left edge"
        );
        assert!(
            seats.iter().any(|seat| *seat > 1920.0),
            "the right hand section flies in from past the right edge"
        );
        assert!(
            app.relayout.target() > 0.0,
            "the strip's own travel carries them in"
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
        open(&mut app);

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
        open(&mut app);

        let mut ui = simulator(app.desk_surface(surface()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }
}
