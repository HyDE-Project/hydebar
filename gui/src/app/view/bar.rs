//! The main bar surface: sections, backdrop and menu dismissal.
//!
//! Two rooms: the strip itself is built here, and [`backdrop`] answers for
//! everything painted behind it — the wash of each appearance style and what
//! is left of the strip as it parts for the canvas.

mod backdrop;

use hydebar_core::{HEIGHT, menu::dismiss_area};
use hydebar_proto::config::AppearanceStyle;
use iced::{Alignment, Element, Length, SurfaceId as Id, widget::container};

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
            background: self.strip_backdrop(t, wash),
            ..Default::default()
        });

        self.dismisses_the_open_menu(bar.into())
    }
}
