//! The usage meter: a filled share over a weak track, coloured by how
//! full the pool is.

use iced::{
    Border, Color, Element, Length, Theme,
    widget::{Row as BarRow, Space, container}
};

use super::super::{
    super::Message,
    metrics::METER_HEIGHT,
    model::{self, MeterLevel}
};
use crate::components::scale;

/// A usage meter: the filled share over a weak track, coloured by how
/// full the pool is.
pub(super) fn meter_view<'a>(percent: u32) -> Element<'a, Message> {
    let percent = percent.min(100);
    let radius = scale::scaled(METER_HEIGHT / 2.0);

    let mut bar = BarRow::new();

    if percent > 0 {
        bar = bar.push(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::FillPortion(percent as u16))
                .height(Length::Fill)
                .style(move |theme: &Theme| fill_style(theme, percent, radius))
        );
    }

    if percent < 100 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the remainder of a share clamped to 0..=100 fits u16"
        )]
        {
            bar = bar.push(Space::new().width(Length::FillPortion((100 - percent) as u16)));
        }
    }

    container(bar)
        .width(Length::Fill)
        .height(Length::Fixed(scale::scaled(METER_HEIGHT)))
        .style(move |theme: &Theme| track_style(theme, radius))
        .into()
}

fn fill_style(theme: &Theme, percent: u32, radius: f32) -> container::Style {
    let colour = match model::meter_level(percent) {
        MeterLevel::Calm => theme.palette().primary,
        MeterLevel::Busy => theme.palette().warning,
        MeterLevel::Critical => theme.palette().danger
    };

    rounded(colour.into(), radius)
}

fn track_style(theme: &Theme, radius: f32) -> container::Style {
    rounded(
        theme.extended_palette().background.weak.color.into(),
        radius
    )
}

fn rounded(background: iced::Background, radius: f32) -> container::Style {
    container::Style {
        background: Some(background),
        border: Border {
            width:  0.0,
            radius: radius.into(),
            color:  Color::TRANSPARENT
        },
        ..container::Style::default()
    }
}
