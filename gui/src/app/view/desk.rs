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
pub mod column;
mod readings;

use hydebar_core::config::ModuleName;

/// The three sections of the layout with the turn every unit takes, and how
/// many turns there are in all.
type Turns<'a> = (
    Vec<(usize, &'a ModuleName)>,
    Vec<(usize, &'a ModuleName)>,
    Vec<(usize, &'a ModuleName)>,
    usize
);
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

        let (left, centre, right, units) = Self::desk_turns(modules);

        let columns = [
            (&left, blocks::Side::Leading),
            (&centre, blocks::Side::Middle),
            (&right, blocks::Side::Trailing)
        ]
        .into_iter()
        .filter_map(|(order, side)| self.desk_column(order, id, side, ink, unfolding, units));

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

    /// The turn every unit of the layout takes, and how many there are.
    ///
    /// The middle of the strip goes first: the centre section comes down,
    /// then the sections either side of it, each unit waiting for the one
    /// before it. Numbering them here rather than per column is what lets the
    /// three sections share one queue instead of racing each other down.
    pub(crate) fn desk_turns(modules: &hydebar_core::config::Modules) -> Turns<'_> {
        let centre = Self::desk_order(&modules.center, false);
        let left = Self::desk_order(&modules.left, true);
        let right = Self::desk_order(&modules.right, false);

        let mut turn = 0;
        let mut centre_turns = Vec::with_capacity(centre.len());

        for unit in centre {
            centre_turns.push((turn, unit));
            turn += 1;
        }

        let sides = left.len().max(right.len());
        let mut left = left.into_iter();
        let mut right = right.into_iter();
        let mut left_turns = Vec::new();
        let mut right_turns = Vec::new();

        for _ in 0..sides {
            if let Some(unit) = left.next() {
                left_turns.push((turn, unit));
                turn += 1;
            }

            if let Some(unit) = right.next() {
                right_turns.push((turn, unit));
                turn += 1;
            }
        }

        (left_turns, centre_turns, right_turns, turn)
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
        app.desk_clocks.entry(None).or_default().open();
    }

    /// Starts a bar's canvas travelling, one frame in.
    fn set_off(app: &mut App) {
        let total = std::time::Duration::from_millis(900);
        let clock = app.desk_clocks.entry(None).or_default();

        *clock = hydebar_core::animation::Unfold::default();
        clock.advance(std::time::Duration::from_millis(16), total);
    }

    /// Advances every canvas by one frame of a nine hundred millisecond
    /// travel.
    fn tick(app: &mut App) -> bool {
        let total = std::time::Duration::from_millis(900);

        app.desk_clocks.values_mut().fold(false, |running, clock| {
            clock.advance(std::time::Duration::from_millis(16), total) | running
        })
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
    fn a_screen_holding_a_window_keeps_its_strip_however_long_it_stands() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        let hour = app.clock.data().format(&app.config.clock.format);

        app.desk_clocks.entry(None).or_default();

        for frame in 0..240 {
            let _ = app.advance_desk(std::time::Duration::from_millis(16));

            assert!(
                simulator(app.bar_surface(surface()))
                    .find(hour.as_str())
                    .is_ok(),
                "frame {frame}: the strip stands on a screen the canvas does not cover"
            );
        }
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

        set_off(&mut app);

        for frame in 0..96 {
            let (strip, canvas) = shapes_on_screen(&app);

            assert!(
                strip != canvas,
                "frame {frame}: strip {strip}, canvas {canvas} — one shape, never two and never none"
            );

            if !tick(&mut app) {
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

        app.desk_clocks.entry(None).or_default().fold();

        let (strip, canvas) = shapes_on_screen(&app);
        assert!(
            strip && !canvas,
            "the strip is back with the window, not travelling behind it"
        );
    }

    #[test]
    fn the_front_moves_without_a_pause_between_one_block_and_the_next() {
        for blocks in 2..8usize {
            for step in 1..400 {
                #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
                let unfolding = step as f32 / 400.0;

                let moving = (0..blocks).any(|index| {
                    #[expect(clippy::cast_precision_loss, reason = "a handful of blocks")]
                    let place = index as f32 / (blocks - 1) as f32;
                    let (travel, bloom) = hydebar_core::animation::share(unfolding, place, blocks);

                    (travel > 0.0 && travel < 1.0) || (bloom > 0.0 && bloom < 1.0)
                });

                assert!(
                    moving,
                    "{blocks} blocks: the front stands still at {unfolding:.3}"
                );
            }
        }
    }

    #[test]
    fn a_block_sets_off_after_the_one_before_it() {
        for blocks in 2..8usize {
            for step in 0..=400 {
                #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
                let unfolding = step as f32 / 400.0;

                let starts: Vec<f32> = (0..blocks)
                    .map(|index| {
                        #[expect(clippy::cast_precision_loss, reason = "a handful of blocks")]
                        let place = index as f32 / (blocks - 1) as f32;

                        hydebar_core::animation::share(unfolding, place, blocks).0
                    })
                    .collect();

                for pair in starts.windows(2) {
                    assert!(
                        pair[0] >= pair[1],
                        "{blocks} blocks at {unfolding:.3}: the leader is never behind"
                    );
                }
            }
        }
    }

    #[test]
    fn a_block_crosses_the_screen_before_it_writes_itself_out() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        let mut wrote_while_crossing = false;
        let mut crossed = false;

        for _ in 0..256 {
            let (travel, bloom) = hydebar_core::animation::share(app.desk_presence(None), 0.0, 2);
            let writing = simulator(app.desk_surface(surface()))
                .find("MEMORY")
                .is_ok();

            crossed |= travel >= 1.0;
            wrote_while_crossing |= writing && travel < 1.0;

            let _ = bloom;

            if !tick(&mut app) {
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
    fn a_block_opens_from_the_top_rather_than_a_line_at_a_time() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        let shown = |app: &App| {
            ["in use", "available", "cached"]
                .into_iter()
                .filter(|row| {
                    iced_test::simulator::Simulator::with_size(
                        iced_test::core::Settings::default(),
                        iced::Size::new(1920.0, 1080.0),
                        app.desk_surface(surface())
                    )
                    .find(*row)
                    .ok()
                    .and_then(|found| found.visible_bounds())
                    .is_some()
                })
                .count()
        };

        let mut first = None;
        let mut last = 0;

        for _ in 0..256 {
            let drawn = shown(&app);

            if drawn > 0 && first.is_none() {
                first = Some(drawn);
            }

            last = drawn;

            if !tick(&mut app) {
                break;
            }
        }

        assert_eq!(last, 3, "every reading stands once the block is open");
        assert!(
            first.is_some_and(|first| first < last),
            "the block shows less of itself while it is still opening"
        );
    }

    #[test]
    fn the_middle_of_the_strip_comes_down_before_the_sides() {
        use hydebar_core::config::{ModuleName, Modules};
        use hydebar_proto::config::ModuleDef;

        let modules = Modules {
            left:   vec![ModuleDef::Single(ModuleName::Memory)],
            center: vec![ModuleDef::Single(ModuleName::Clock)],
            right:  vec![ModuleDef::Single(ModuleName::CpuTemp)]
        };

        let (left, centre, right, units) = App::desk_turns(&modules);

        assert_eq!(units, 3);
        assert_eq!(centre[0].0, 0, "the centre section takes the first turn");
        assert!(
            left[0].0 > centre[0].0 && right[0].0 > centre[0].0,
            "both sides wait for the middle"
        );
    }

    #[test]
    fn every_module_of_a_group_takes_its_own_turn_and_its_own_pill() {
        use hydebar_core::config::ModuleName;
        use hydebar_proto::config::ModuleDef;

        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = vec![ModuleDef::Group(vec![
                ModuleName::Memory,
                ModuleName::CpuTemp,
            ])];
            config.modules.center = Vec::new();
            config.modules.right = Vec::new();
        });
        open(&mut app);

        let (left, _, _, units) = App::desk_turns(&app.config.modules);

        assert_eq!(units, 2, "the icons part, so each takes a turn of its own");
        assert!(left[0].0 < left[1].0, "one after the other, not together");

        let far = simulator(app.desk_surface(surface()))
            .find("MEMORY")
            .expect("the member that stood further from the middle")
            .bounds();
        let near = simulator(app.desk_surface(surface()))
            .find("CPU TEMPERATURE")
            .expect("the member that stood nearer the middle")
            .bounds();

        assert!(
            far.y > near.y,
            "each stands in its own place, the nearer one higher: {} against {}",
            far.y,
            near.y
        );
    }

    #[test]
    fn every_module_of_a_group_flies_home_not_only_its_first() {
        use hydebar_core::config::ModuleName;
        use hydebar_proto::config::ModuleDef;

        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = vec![ModuleDef::Group(vec![
                ModuleName::Memory,
                ModuleName::CpuTemp,
            ])];
            config.modules.center = Vec::new();
            config.modules.right = Vec::new();
        });
        app.screen_width = Some(1920.0);
        open(&mut app);

        app.desk_clocks.entry(None).or_default().fold();
        app.send_the_islands_home(surface());

        let seated = app.flip.borrow().from_map().len();

        assert!(
            seated >= 2,
            "both modules of the group are given a seat to fly from, not {seated}"
        );
    }

    #[test]
    fn the_strip_stands_until_the_last_island_has_left_it() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        let mut held_while_any_waited = true;
        let mut left_once_all_had_gone = false;

        for _ in 0..256 {
            let unfolding = app.desk_presence(None);
            let units = App::desk_turns(&app.config.modules).3;
            let waiting = (0..units).any(|turn| {
                #[expect(clippy::cast_precision_loss, reason = "a handful of units")]
                let place = if units > 1 {
                    turn as f32 / (units - 1) as f32
                } else {
                    0.0
                };

                hydebar_core::animation::share(unfolding, place, units).0 <= 0.0
            });

            if waiting {
                held_while_any_waited &= app.strip_still_holds(None);
            } else {
                left_once_all_had_gone |= !app.strip_still_holds(None);
            }

            if !tick(&mut app) {
                break;
            }
        }

        assert!(
            held_while_any_waited,
            "the strip stands while an island is still on it"
        );
        assert!(
            left_once_all_had_gone,
            "the strip leaves once the last island has set off"
        );
    }

    #[test]
    fn a_module_is_drawn_by_one_shape_at_a_time_while_the_strip_empties() {
        use hydebar_core::config::ModuleName;

        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        let watched = [ModuleName::Clock, ModuleName::SystemInfo];

        for _ in 0..256 {
            for module in &watched {
                let left = app.has_left_the_strip(module, None);
                let hour = app.clock.data().format(&app.config.clock.format);
                let probe = if matches!(module, ModuleName::Clock) {
                    hour.as_str()
                } else {
                    continue;
                };

                let on_strip = simulator(app.bar_surface(surface())).find(probe).is_ok();
                let on_canvas = simulator(app.desk_surface(surface())).find(probe).is_ok();

                assert!(
                    on_strip != on_canvas,
                    "the clock is drawn once: strip {on_strip}, canvas {on_canvas}, left {left}"
                );
                assert_eq!(
                    on_canvas, left,
                    "the canvas draws it exactly once it has left the strip"
                );
            }

            if !tick(&mut app) {
                break;
            }
        }
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

        app.desk_clocks.entry(None).or_default().fold();
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
    fn a_module_with_no_readings_opens_into_the_same_shape_as_the_rest() {
        use hydebar_core::config::ModuleName;
        use hydebar_proto::config::ModuleDef;

        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = vec![ModuleDef::Single(ModuleName::AppLauncher)];
            config.modules.center = Vec::new();
            config.modules.right = Vec::new();
        });
        open(&mut app);

        let mut ui = simulator(app.desk_surface(surface()));

        assert!(
            ui.find("APP LAUNCHER").is_ok(),
            "the module opens under its own heading, blanks where its readings would be"
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
