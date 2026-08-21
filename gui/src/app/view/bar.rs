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

impl App {
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
            background: match self.appearance().style {
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
    fn the_strip_keeps_its_background_a_while_after_its_islands_leave() {
        let mut app = test_app_with(|config| config.desk.enabled = true);

        assert_eq!(
            app.strip_wash(None),
            1.0,
            "a strip with a window is painted"
        );

        let clock = app.desk_clocks.entry(None).or_default();
        *clock = hydebar_core::animation::Unfold::default();
        clock.advance(
            std::time::Duration::from_millis(16),
            std::time::Duration::from_millis(900)
        );

        let mut wash = app.strip_wash(None);

        assert!(
            wash > 0.0 && wash < 1.0,
            "the background is on its way out, not gone: {wash}"
        );

        for _ in 0..256 {
            let now = app.strip_wash(None);

            assert!(now <= wash, "the background only ever goes out");
            wash = now;

            let running = app.desk_clocks.values_mut().fold(false, |running, clock| {
                clock.advance(
                    std::time::Duration::from_millis(16),
                    std::time::Duration::from_millis(900)
                ) | running
            });

            if !running {
                break;
            }
        }

        assert_eq!(
            wash, 0.0,
            "the strip is bare once the unfolding is under way"
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
