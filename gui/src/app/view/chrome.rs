//! Window chrome facts the runtime asks every surface for.

use iced::{
    Alignment, Color, Element, Length, SurfaceId as Id, Theme,
    theme::Style,
    widget::{Row, container}
};

use super::super::state::{App, Message};

impl App {
    #[must_use]
    /// The name the compositor is told to call a surface.
    pub fn title(&self, _id: Id) -> String {
        String::from("hydebar")
    }

    #[must_use]
    /// The palette a surface is drawn with.
    pub fn theme(&self, _id: Id) -> Theme {
        self.theme_cache.clone()
    }

    #[must_use]
    /// The ground a surface is painted on, which is nothing at all.
    pub fn style(&self, theme: &Theme) -> Style {
        Style {
            background_color: Color::TRANSPARENT,
            text_color:       theme.palette().text
        }
    }

    #[must_use]
    /// How much the renderer magnifies a surface.
    pub const fn scale_factor(&self, _id: Id) -> f64 {
        self.appearance().scale_factor
    }

    /// The greeting shown mid-screen while the bar comes up.
    ///
    /// Drawn on the idle menu surface, which spans the screen and is raised
    /// for exactly the greeting's lifetime; it breathes in with the bar's
    /// birth and lets itself out three seconds later. An empty row whenever
    /// the greeting is over, disabled, or animations are off.
    pub(super) fn screen_greeting(&self) -> Element<'_, Message> {
        let presence = self.greeting.value().clamp(0.0, 1.0);

        if presence <= 0.004 {
            return Row::new().into();
        }

        let line = iced::widget::text(self.greeting_line.as_str())
            .size(self.appearance().font_size_px() * 2.4)
            .color(self.theme_cache.palette().text.scale_alpha(presence));

        container(line)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    }
}
