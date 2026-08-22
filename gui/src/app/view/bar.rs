//! The main bar surface: sections, backdrop and menu dismissal.

use std::f32::consts::PI;

use hydebar_core::{
    HEIGHT,
    menu::dismiss_area,
    style::{backdrop_color, darken_color}
};
use hydebar_proto::config::{AppearanceStyle, Position};
use iced::{
    Alignment, Color, Element, Gradient, Length, Radians, SurfaceId as Id, gradient::Linear,
    widget::container
};

use super::super::state::{App, Message};
use crate::centerbox;

/// How wide the edge of the parting grows to, as a share of the strip.
///
/// The background does not end at a line: a hard edge crossing the strip
/// reads as a wipe rather than as a background going out. It is also what
/// the two ends keep to the last, so the ends thin out rather than being
/// clipped away.
const FEATHER: f32 = 0.12;

/// The strip's background parted from the middle, `wash` of it left.
///
/// It goes from the centre outwards, the way the islands leave it: the middle
/// of the strip is bare first and the two ends last. Painted as one gradient
/// along the strip rather than as a flat colour, so what changes from frame
/// to frame is where the two edges of the opening stand, and what is left of
/// the two ends thins as they are driven into the corners.
///
/// The edge is no wider than the opening it edges. A parting that began with
/// its full edge put a quarter of the strip into a soft ramp on the frame the
/// opening was still nothing wide, which is a whole background stepping down
/// in one frame and reads as the blur being switched rather than parting. Held
/// to the opening it grows out of nothing, and a strip with nothing open is
/// the flat colour it was — the same gradient, its stops fallen together.
fn parting(ground: Color, wash: f32) -> iced::Background {
    let wash = wash.clamp(0.0, 1.0);
    let opened = (1.0 - wash) * 0.5;
    let feather = FEATHER.min(opened);
    let near = (0.5 - opened).clamp(feather, 0.5);
    let far = (0.5 + opened).clamp(0.5, 1.0 - feather);
    let left = ground.scale_alpha(wash);

    Gradient::Linear(
        Linear::new(Radians(PI / 2.0))
            .add_stop(0.0, left)
            .add_stop(near - feather, left)
            .add_stop(near, Color::TRANSPARENT)
            .add_stop(far, Color::TRANSPARENT)
            .add_stop(far + feather, left)
            .add_stop(1.0, left)
    )
    .into()
}

impl App {
    /// The colour the strip's own background is painted in.
    ///
    /// One colour for every style, because what it is wanted for is the going
    /// out: the styles differ in how the strip is painted at rest and not in
    /// what is left of it as it leaves.
    fn strip_ground(&self, theme: &iced::Theme) -> Color {
        let appearance = self.appearance();
        let opacity = if appearance.style == AppearanceStyle::Islands {
            appearance.bar_opacity
        } else {
            appearance.opacity
        };

        theme.palette().background.scale_alpha(opacity)
    }

    /// Wraps the bar so a press on it takes the open menu down.
    ///
    /// The menu backdrop covers the screen the bar leaves free and nothing
    /// else, so the bar is the one place the rule that a press outside a menu
    /// dismisses it has to be applied from. The wrapper is only there while a
    /// menu is open, so an ordinary press on the bar costs nothing.
    fn dismisses_the_open_menu<'a>(&self, bar: Element<'a, Message>) -> Element<'a, Message> {
        if self.outputs.menu_is_open() {
            dismiss_area(bar, Message::BarPressed, Message::BarReleased).into()
        } else {
            bar
        }
    }

    /// Draws the bar strip of one output: three sections over the backdrop.
    ///
    /// In the islands style the dim of an open menu lands on top of the
    /// strip's own wash, never in its place: the wash is what the compositor's
    /// blur threshold sees, and replacing it with a fully clear backdrop
    /// turned the blur off for as long as a menu was open.
    #[expect(
        clippy::too_many_lines,
        reason = "the backdrop of every appearance style is painted from one match"
    )]
    pub(super) fn bar_surface(&self, id: Id) -> Element<'_, Message> {
        let screen = self.outputs.screen_of(id).flatten();

        let wash = self.strip_wash(screen);

        if self.desk_holds(screen) && !self.strip_still_holds(screen) && wash <= 0.0 {
            return iced::widget::Row::new().into();
        }

        let opacity = self.appearance().opacity;
        let left = self.modules_section(&self.config.modules.left, id, opacity, 0);
        let center = self.modules_section(
            &self.config.modules.center,
            id,
            opacity,
            self.config.modules.left.len()
        );
        let right = self.modules_section(
            &self.config.modules.right,
            id,
            opacity,
            self.config.modules.left.len() + self.config.modules.center.len()
        );

        #[expect(
            clippy::cast_possible_truncation,
            reason = "the bar height constant is exactly representable in f32"
        )]
        let bar_height = self.appearance().height.unwrap_or(HEIGHT as f32);

        let centerbox = centerbox::Centerbox::new([left, center, right])
            .spacing(self.appearance().group_gap())
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .height(if self.appearance().style == AppearanceStyle::Islands {
                bar_height
            } else {
                bar_height - 8.
            })
            .padding(if self.appearance().style == AppearanceStyle::Islands {
                self.appearance().bar_padding()
            } else {
                [0.0, 0.0]
            });

        let bar = container(centerbox).style(move |t| container::Style {
            background: if wash < 1.0 {
                Some(parting(self.strip_ground(t), wash))
            } else {
                match self.appearance().style {
                    AppearanceStyle::Gradient => Some({
                        let start_color = t
                            .palette()
                            .background
                            .scale_alpha(self.appearance().opacity * wash);

                        let start_color = if self.outputs.menu_is_open() {
                            darken_color(start_color, self.appearance().menu.backdrop)
                        } else {
                            start_color
                        };

                        let end_color = if self.outputs.menu_is_open() {
                            backdrop_color(self.appearance().menu.backdrop)
                        } else {
                            Color::TRANSPARENT
                        };

                        Gradient::Linear(
                            Linear::new(Radians(PI))
                                .add_stop(
                                    0.0,
                                    match self.config.position {
                                        Position::Top => start_color,
                                        Position::Bottom => end_color
                                    }
                                )
                                .add_stop(
                                    1.0,
                                    match self.config.position {
                                        Position::Top => end_color,
                                        Position::Bottom => start_color
                                    }
                                )
                        )
                        .into()
                    }),
                    AppearanceStyle::Solid => Some({
                        let bg = t
                            .palette()
                            .background
                            .scale_alpha(self.appearance().opacity * wash);
                        if self.outputs.menu_is_open() {
                            darken_color(bg, self.appearance().menu.backdrop)
                        } else {
                            bg
                        }
                        .into()
                    }),
                    AppearanceStyle::Islands => {
                        let painted = (self.appearance().bar_opacity * wash > 0.0).then(|| {
                            t.palette()
                                .background
                                .scale_alpha(self.appearance().bar_opacity * wash)
                        });

                        match (painted, self.outputs.menu_is_open()) {
                            (Some(painted), true) => {
                                Some(darken_color(painted, self.appearance().menu.backdrop).into())
                            }
                            (Some(painted), false) => Some(painted.into()),
                            (None, true) => {
                                Some(backdrop_color(self.appearance().menu.backdrop).into())
                            }
                            (None, false) => None
                        }
                    }
                }
            },
            ..Default::default()
        });

        self.dismisses_the_open_menu(bar.into())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced_test::simulator;

    use super::{super::super::state::test_support::test_app_with, *};

    fn surface() -> Id {
        Id::unique()
    }

    #[test]
    fn every_appearance_style_paints_a_bar() {
        for style in [
            AppearanceStyle::Islands,
            AppearanceStyle::Solid,
            AppearanceStyle::Gradient
        ] {
            let app = test_app_with(|config| config.appearance.style = style);
            let mut ui = simulator(app.bar_surface(surface()));

            assert!(ui.snapshot(&iced::Theme::Dark).is_ok(), "{style:?} paints");
        }
    }

    #[test]
    fn a_gradient_bar_is_painted_from_either_edge() {
        for position in [Position::Top, Position::Bottom] {
            let app = test_app_with(|config| {
                config.appearance.style = AppearanceStyle::Gradient;
                config.position = position;
            });
            let mut ui = simulator(app.bar_surface(surface()));

            assert!(
                ui.snapshot(&iced::Theme::Dark).is_ok(),
                "{position:?} paints"
            );
        }
    }

    #[test]
    fn the_background_goes_out_behind_the_islands_and_never_before_them() {
        let mut app = test_app_with(|config| config.desk.enabled = true);

        assert_eq!(
            app.strip_wash(None),
            1.0,
            "a strip with a window is painted"
        );

        let total = std::time::Duration::from_millis(900);
        let clock = app.desk_clocks.entry(None).or_default();
        *clock = hydebar_core::animation::Unfold::default();
        clock.advance(std::time::Duration::from_millis(16), total);

        assert_eq!(
            app.strip_wash(None),
            1.0,
            "nothing of the background has gone while every island is still in the air"
        );

        let nearest = hydebar_core::animation::landed(App::reach(0, app.deepest_column()));
        let furthest = hydebar_core::animation::landed(1.0);
        let mut wash = 1.0;

        for _ in 0..256 {
            let now = app.strip_wash(None);
            let progress = app.desk_presence(None);

            assert!(now <= wash, "the background only ever goes out");

            if progress < nearest {
                assert_eq!(now, 1.0, "it waits for the first island to land");
            }

            if progress < furthest {
                assert!(now > 0.0, "it is still under the islands yet to land");
            }

            wash = now;

            let running = app.desk_clocks.values_mut().fold(false, |running, clock| {
                clock.advance(std::time::Duration::from_millis(16), total) | running
            });

            if !running {
                break;
            }
        }

        assert_eq!(wash, 0.0, "the strip is bare once the last island is down");
    }

    #[test]
    fn the_background_comes_back_from_the_ends_inwards_ahead_of_the_islands() {
        let mut app = test_app_with(|config| config.desk.enabled = true);

        app.desk_clocks.entry(None).or_default().open();
        app.screen_width = Some(1920.0);
        app.desk_clocks.entry(None).or_default().fold();
        app.send_the_islands_home(surface());

        assert!(
            app.desk_returning.is_running(),
            "the strip knows it is on its way back"
        );
        assert!(
            app.strip_wash(None) < 0.05,
            "next to nothing of the background is painted as the islands set off"
        );

        let mut wash = 0.0;
        let mut whole_before_they_land = false;

        for _ in 0..256 {
            let now = app.strip_wash(None);

            assert!(now >= wash, "the background only ever comes back");
            wash = now;

            let sliding = app.relayout.advance(std::time::Duration::from_millis(16));
            let _ = app.advance_desk(std::time::Duration::from_millis(16));

            whole_before_they_land |= wash >= 1.0 && sliding;

            if !sliding {
                break;
            }
        }

        assert!(
            whole_before_they_land,
            "the background is whole while the islands are still flying in"
        );
    }

    #[test]
    fn the_background_leaves_the_middle_of_the_strip_before_its_ends() {
        let ground = Color::from_rgba(1.0, 1.0, 1.0, 1.0);

        let stops = |wash: f32| match parting(ground, wash) {
            iced::Background::Gradient(Gradient::Linear(linear)) => linear,
            iced::Background::Color(_) => panic!("the parting is painted as a gradient")
        };

        let clear = |wash: f32| {
            stops(wash)
                .stops
                .into_iter()
                .flatten()
                .filter(|stop| stop.color.a == 0.0)
                .map(|stop| stop.offset)
                .collect::<Vec<f32>>()
        };

        let half = clear(0.5);

        assert_eq!(half.len(), 2, "the opening has two edges");
        assert!(
            half[0] < 0.5 && half[1] > 0.5,
            "the opening stands on the middle of the strip: {half:?}"
        );

        let wider = clear(0.2);

        assert!(
            wider[0] < half[0] && wider[1] > half[1],
            "the opening only ever widens: {wider:?} against {half:?}"
        );

        let ends = |wash: f32| {
            stops(wash)
                .stops
                .into_iter()
                .flatten()
                .filter(|stop| stop.offset == 0.0 || stop.offset == 1.0)
                .map(|stop| stop.color.a)
                .collect::<Vec<f32>>()
        };

        assert!(
            ends(0.1).iter().all(|alpha| *alpha < 0.2),
            "what is left at the two ends thins as it is driven out: {:?}",
            ends(0.1)
        );
    }

    #[test]
    fn a_strip_with_nothing_open_is_painted_as_flatly_as_one_that_never_parted() {
        let ground = Color::from_rgba(1.0, 1.0, 1.0, 1.0);

        let stops = |wash: f32| match parting(ground, wash) {
            iced::Background::Gradient(Gradient::Linear(linear)) => linear
                .stops
                .into_iter()
                .flatten()
                .map(|stop| (stop.offset, stop.color.a))
                .collect::<Vec<(f32, f32)>>(),
            iced::Background::Color(_) => panic!("the parting is painted as a gradient")
        };

        let painted = |wash: f32| {
            let stops = stops(wash);

            (0..=1000)
                .map(|step| {
                    #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
                    let along = step as f32 / 1000.0;
                    let after = stops
                        .iter()
                        .position(|(offset, _)| *offset >= along)
                        .unwrap_or(stops.len() - 1)
                        .max(1);
                    let (from, left) = stops[after - 1];
                    let (to, right) = stops[after];
                    let within = ((along - from) / (to - from).max(f32::EPSILON)).clamp(0.0, 1.0);

                    (right - left).mul_add(within, left)
                })
                .sum::<f32>()
                / 1001.0
        };

        assert_eq!(
            painted(1.0),
            1.0,
            "a strip with nothing open is painted end to end"
        );

        let mut before = 1.0_f32;

        for step in 0..=200 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let wash = 1.0 - step as f32 / 200.0;
            let now = painted(wash);

            assert!(
                before - now < 0.05,
                "the background steps from {before:.3} to {now:.3} at a wash of {wash:.3}"
            );

            before = now;
        }

        assert!(
            before < 0.01,
            "nothing of the background is left at the end: {before:.3}"
        );
    }

    #[test]
    fn an_island_bar_without_a_wash_is_left_unpainted() {
        let app = test_app_with(|config| {
            config.appearance.style = AppearanceStyle::Islands;
            config.appearance.bar_opacity = 0.0;
        });

        let mut ui = simulator(app.bar_surface(surface()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn an_island_bar_with_a_wash_carries_it() {
        let app = test_app_with(|config| {
            config.appearance.style = AppearanceStyle::Islands;
            config.appearance.bar_opacity = 0.5;
        });

        let mut ui = simulator(app.bar_surface(surface()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn a_bar_with_no_menu_open_takes_no_dismissal_wrapper() {
        let app = test_app_with(|_| {});

        assert!(!app.outputs.menu_is_open());

        let mut ui = simulator(app.bar_surface(surface()));
        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn a_bar_naming_its_own_height_is_drawn_to_it() {
        let tall = test_app_with(|config| config.appearance.height = Some(48.0));
        let short = test_app_with(|config| config.appearance.height = Some(24.0));

        let tall = simulator(tall.bar_surface(surface()))
            .snapshot(&iced::Theme::Dark)
            .is_ok();
        let short = simulator(short.bar_surface(surface()))
            .snapshot(&iced::Theme::Dark)
            .is_ok();

        assert!(tall && short);
    }

    #[test]
    fn the_three_sections_are_drawn_side_by_side() {
        let app = test_app_with(|config| {
            config.modules.left = vec![hydebar_proto::config::ModuleDef::Single(
                hydebar_core::config::ModuleName::Clock
            )];
            config.modules.center = Vec::new();
            config.modules.right = vec![hydebar_proto::config::ModuleDef::Single(
                hydebar_core::config::ModuleName::Settings
            )];
        });

        let mut ui = simulator(app.bar_surface(surface()));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }
}
