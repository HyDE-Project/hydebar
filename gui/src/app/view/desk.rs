//! The desk surface: the shape the bar takes when the screen is bare.
//!
//! Not a second bar and not a second set of readouts — the same modules, in
//! the same layout, come down off the strip and stand over the wallpaper at a
//! size a room away can read. The three sections of the layout become three
//! columns, each keeping its own order and its own groups, and each module
//! keeps the presses it answers to on the strip.
//!
//! [`column`] stacks a section into a column, [`blocks`] draws what a unit
//! opens into, [`readings`] settles what each block says, and [`fit`] sizes
//! the writing so the deepest column ends on the screen.

mod blocks;
pub mod column;
mod fit;
mod readings;
mod trails;

use hydebar_core::config::ModuleName;

/// The three sections of the layout, each in the order its column stands in.
type Columns<'a> = (
    Vec<&'a ModuleName>,
    Vec<&'a ModuleName>,
    Vec<&'a ModuleName>
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
            size:  fit::ink_size(self, id, self.canvas_room(id))
        };
        let margin = ink.size * 2.0;
        let room = self.canvas_room(id);
        let modules = &self.config.modules;

        let (left, centre, right) = Self::desk_columns(modules);
        let deepest = self.deepest_column();

        let columns = [
            (&left, blocks::Side::Leading),
            (&centre, blocks::Side::Middle),
            (&right, blocks::Side::Trailing)
        ]
        .into_iter()
        .filter_map(|(order, side)| {
            self.desk_column(order, id, side, ink, unfolding, deepest, room)
        });

        let canvas = container(
            Row::with_children(columns)
                .spacing(margin)
                .align_y(Alignment::Start)
                .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top:    self.strip_band(id) + margin,
            right:  margin,
            bottom: margin,
            left:   margin
        });

        let ways = self.desk_ways(id, unfolding, deepest, ink, room);

        if ways.is_empty() {
            return canvas.into();
        }

        iced::widget::Stack::new()
            .push(trails::trails(ways, ink.value, ink.size))
            .push(canvas)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The way every block of this screen has come, for the trails behind them.
    ///
    /// Read off the seats the last frame recorded rather than measured again:
    /// the places do not move while a block travels to them, and a canvas
    /// that laid itself out twice a frame to draw a streak would pay for the
    /// streak in the very smoothness it is there to show.
    fn desk_ways(
        &self,
        id: Id,
        unfolding: f32,
        deepest: usize,
        ink: blocks::Ink,
        room: f32
    ) -> Vec<trails::Way> {
        if unfolding >= 1.0 {
            return Vec::new();
        }

        let memo = self.flip.borrow();
        let (left, centre, right) = Self::desk_columns(&self.config.modules);

        [&left, &centre, &right]
            .into_iter()
            .flat_map(|order| {
                self.desk_runs(order, id, ink, room)
                    .into_iter()
                    .flatten()
                    .filter_map(|within| {
                        let key = self.flip_key(order[within], id);
                        let travel = hydebar_core::animation::share(
                            unfolding,
                            Self::reach(within, deepest)
                        )
                        .0;

                        Some(trails::Way {
                            seat: *memo.seats().get(&key)?,
                            from_x: *memo.from_map().get(&key)?,
                            from_y: self.strip_row(id),
                            travel
                        })
                    })
                    .collect::<Vec<trails::Way>>()
            })
            .collect()
    }

    /// The three columns of the canvas, each in the order it stands in.
    ///
    /// Read from the middle of the strip outwards: the unit that stood
    /// nearest the centre heads its column and the one that stood at an edge
    /// ends it, which is what puts the ends of the bar down in the corners of
    /// the screen. Nothing here says when a unit moves — they all move at
    /// once — only where each of them is bound.
    pub(crate) fn desk_columns(modules: &hydebar_core::config::Modules) -> Columns<'_> {
        (
            Self::desk_order(&modules.left, true),
            Self::desk_order(&modules.center, false),
            Self::desk_order(&modules.right, false)
        )
    }

    /// How many places the longest column of the canvas stands.
    ///
    /// The measure every block's journey is stated against: the block at the
    /// bottom of this column is the one with the furthest to go.
    pub(crate) fn deepest_column(&self) -> usize {
        let (left, centre, right) = Self::desk_columns(&self.config.modules);

        left.len().max(centre.len()).max(right.len())
    }

    /// The height the columns have to end within.
    ///
    /// Everything below the strip's own band, less the margin the canvas keeps
    /// around itself, on a screen the bar has been told the height of. A
    /// screen it has not is answered with nothing to fit into, which leaves
    /// the writing at the size the theme asked for.
    fn canvas_room(&self, id: Id) -> f32 {
        let margin = self.appearance().font_size_px() * 2.0;

        self.screen_height
            .map_or(f32::INFINITY, |height| {
                height - self.strip_band(id) - margin * 2.0
            })
            .max(0.0)
    }

    /// The band along the top of the screen the strip itself occupies.
    ///
    /// The canvas covers the whole screen so its blocks can leave the strip
    /// without jumping, which means the places they land have to keep clear
    /// of the band the strip stands in — the strip's own height, and whatever
    /// reserved a band above it before the strip was put there.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the bar height constant is exactly representable in f32"
    )]
    fn strip_band(&self, id: Id) -> f32 {
        self.strip_top(id)
            + self
                .appearance()
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

    /// A simulator the size of the screen the canvas is drawn for.
    ///
    /// The blocks take the room they will need from the first frame, so a
    /// canvas drawn into a window smaller than a screen has its lower blocks
    /// off the bottom of it and nothing to say about what is visible.
    fn on_screen(app: &App) -> iced_test::simulator::Simulator<'_, Message> {
        iced_test::simulator::Simulator::with_size(
            iced_test::core::Settings::default(),
            iced::Size::new(1920.0, 1080.0),
            app.desk_surface(surface())
        )
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
    fn the_month_grid_is_whole_once_the_clock_has_opened() {
        let app = unfolded(1.0);

        for week in ["27", "17", "31"] {
            assert!(
                on_screen(&app)
                    .find(week)
                    .ok()
                    .and_then(|found| found.visible_bounds())
                    .is_some(),
                "the row holding {week} is inside the room the month was given"
            );
        }
    }

    #[test]
    fn the_month_grid_opens_rather_than_standing_there_whole() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        let month = app.clock.data().format("%B %Y");
        let mut seen_partly = false;

        for _ in 0..256 {
            let heading = on_screen(&app)
                .find(month.as_str())
                .ok()
                .and_then(|found| found.visible_bounds())
                .is_some();
            let last = on_screen(&app)
                .find("17")
                .ok()
                .and_then(|found| found.visible_bounds())
                .is_some();

            seen_partly |= heading && !last;

            if !tick(&mut app) {
                break;
            }
        }

        assert!(
            seen_partly,
            "the grid is written out from the top like every other block"
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
    fn the_whole_bar_is_under_way_on_the_first_frame() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        assert!(
            app.has_left_the_strip(None),
            "no island stands waiting while its neighbours fly"
        );

        let hour = app.clock.data().format(&app.config.clock.format);

        assert!(
            simulator(app.bar_surface(surface()))
                .find(hour.as_str())
                .is_err(),
            "nothing of the bar is left standing on the strip"
        );
        assert!(
            simulator(app.desk_surface(surface()))
                .find(hour.as_str())
                .is_ok(),
            "the canvas carries it from the first frame of the travel"
        );
    }

    #[test]
    fn a_block_drops_to_its_level_before_it_writes_itself_out() {
        // a layout of two, so the block being watched is on the test surface
        // whatever every other module grows into
        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = Vec::new();
            config.modules.center = Vec::new();
            config.modules.right = vec![
                hydebar_proto::config::ModuleDef::Single(hydebar_core::config::ModuleName::Clock),
                hydebar_proto::config::ModuleDef::Single(hydebar_core::config::ModuleName::Memory),
            ];
        });
        set_off(&mut app);

        let mut wrote_before_landing = false;

        for _ in 0..256 {
            let (_, _, right) = App::desk_columns(&app.config.modules);
            let reach = App::reach(1, right.len());
            let down = hydebar_core::animation::share(app.desk_presence(None), reach).1 > 0.0;
            let writing = on_screen(&app)
                .find("MEMORY")
                .ok()
                .and_then(|found| found.visible_bounds())
                .is_some();

            wrote_before_landing |= writing && !down;

            if !tick(&mut app) {
                break;
            }
        }

        assert!(
            !wrote_before_landing,
            "no block writes itself out before it is down on its level"
        );
        assert!(
            on_screen(&app)
                .find("MEMORY")
                .ok()
                .and_then(|found| found.visible_bounds())
                .is_some(),
            "the blocks are open once the unfolding is over"
        );
    }

    #[test]
    fn the_block_with_less_way_to_go_opens_first() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        let mut near = None;
        let mut far = None;

        for frame in 0..256 {
            let seen = |title: &str| {
                on_screen(&app)
                    .find(title)
                    .ok()
                    .and_then(|found| found.visible_bounds())
                    .is_some()
            };

            if near.is_none() && seen("SYSTEM") {
                near = Some(frame);
            }

            if far.is_none() && seen("APP LAUNCHER") {
                far = Some(frame);
            }

            if !tick(&mut app) {
                break;
            }
        }

        let near = near.expect("the block nearest the strip opens");
        let far = far.expect("the block furthest from the strip opens");

        assert!(
            near < far,
            "the near block opened on frame {near} and the far one on {far}: it waited"
        );
    }

    #[test]
    fn nothing_shifts_in_the_column_while_the_blocks_open() {
        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        let mut seat = None;

        for _ in 0..256 {
            if hydebar_core::animation::share(app.desk_presence(None), 1.0).0 >= 1.0 {
                let now = on_screen(&app)
                    .find("PROCESSOR")
                    .expect("the block under the first one")
                    .bounds()
                    .y;
                let first = *seat.get_or_insert(now);

                assert!(
                    (now - first).abs() < 0.5,
                    "the block below moved from {first} to {now} as the one above it opened"
                );
            }

            if !tick(&mut app) {
                break;
            }
        }

        assert!(seat.is_some(), "the blocks reach their places");
    }

    #[test]
    fn a_block_opens_from_the_top_rather_than_a_line_at_a_time() {
        // one module in the layout, so what is asserted is the block opening
        // rather than how much of a full column a test surface has room for
        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.modules.left = vec![hydebar_proto::config::ModuleDef::Single(
                hydebar_core::config::ModuleName::Memory
            )];
            config.modules.center = Vec::new();
            config.modules.right = Vec::new();
        });
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
    fn a_column_is_read_from_the_middle_of_the_strip_outwards() {
        use hydebar_core::config::{ModuleName, Modules};
        use hydebar_proto::config::ModuleDef;

        let modules = Modules {
            left:   vec![
                ModuleDef::Single(ModuleName::Memory),
                ModuleDef::Single(ModuleName::Clock),
            ],
            center: Vec::new(),
            right:  vec![
                ModuleDef::Single(ModuleName::CpuTemp),
                ModuleDef::Single(ModuleName::Battery),
            ]
        };

        let (left, _, right) = App::desk_columns(&modules);

        assert_eq!(
            left,
            vec![&ModuleName::Clock, &ModuleName::Memory],
            "the left section is read towards the middle, so it is turned around"
        );
        assert_eq!(
            right,
            vec![&ModuleName::CpuTemp, &ModuleName::Battery],
            "the right section already reads outwards"
        );
    }

    #[test]
    fn every_module_of_a_group_takes_its_own_place_and_its_own_pill() {
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

        let (left, _, _) = App::desk_columns(&app.config.modules);

        assert_eq!(
            left.len(),
            2,
            "the icons part, so each takes a place of its own"
        );

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
    fn the_strip_stands_while_its_islands_are_on_it_and_not_after() {
        let mut app = test_app_with(|config| config.desk.enabled = true);

        assert!(
            app.strip_still_holds(None),
            "the strip holds its islands while nothing has set off"
        );

        set_off(&mut app);

        for _ in 0..256 {
            assert!(
                !app.strip_still_holds(None),
                "the strip is empty from the frame the islands set off"
            );

            if !tick(&mut app) {
                break;
            }
        }
    }

    #[test]
    fn a_module_is_drawn_by_one_shape_at_a_time_while_the_strip_empties() {
        use hydebar_core::config::ModuleName;

        let mut app = test_app_with(|config| config.desk.enabled = true);
        set_off(&mut app);

        let watched = [ModuleName::Clock, ModuleName::SystemInfo];

        for _ in 0..256 {
            for module in &watched {
                let left = app.has_left_the_strip(None);
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
    fn every_block_of_a_side_section_stands_on_the_same_edge() {
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

        let far = simulator(app.desk_surface(surface()))
            .find("CPU TEMPERATURE")
            .expect("the far module")
            .bounds();
        let near = simulator(app.desk_surface(surface()))
            .find("MEMORY")
            .expect("the near module")
            .bounds();

        assert!(
            near.y < far.y,
            "the near module stands higher: {} against {}",
            near.y,
            far.y
        );
        assert!(
            (near.x - far.x).abs() < 1.0,
            "and both stand on the edge of the screen: {} against {}",
            near.x,
            far.x
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
