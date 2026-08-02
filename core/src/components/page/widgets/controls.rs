//! The rows that act: buttons, choices, steppers, status readouts and the
//! chips that draw a picture of the bar.

use iced::{
    Alignment, Background, Border, Element, Theme,
    widget::{button, container, text}
};

use super::scaffold::{controls, labelled_row};
use crate::{
    components::{icons::icon_raw_sized, page::style},
    style::settings_button_style
};

/// Renders a compact button carrying `label`.
///
/// `font_size` is the page text size, not the button's: the button scales it
/// down itself so no caller can hand it a size of its own invention.
///
/// An `active` button is tinted with the accent colour, which is how the window
/// shows the choice currently in force.
pub fn choice_button<'a, M: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    message: M,
    active: bool,
    font_size: f32,
    opacity: f32
) -> Element<'a, M> {
    let control = style::control_size(font_size);

    button(text(label).size(control))
        .padding([
            style::BUTTON_PADDING_EM[0] * control,
            style::BUTTON_PADDING_EM[1] * control
        ])
        .on_press(message)
        .style(choice_button_style(active, opacity))
        .into()
}

/// Builds the paint of a choice button.
///
/// The settings paint with the accent lifted onto the label of the choice in
/// force, so the row shows which choice is picked without a second control.
fn choice_button_style(
    active: bool,
    opacity: f32
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let mut style = settings_button_style(opacity)(theme, status);

        if active {
            style.text_color = theme.extended_palette().primary.base.color;
        }

        style
    }
}

/// Renders a label followed by a row of mutually exclusive choices.
pub fn choice_row<'a, T, M>(
    label: &'a str,
    choices: Vec<(&'a str, T, bool)>,
    to_message: impl Fn(T) -> M + 'a,
    font_size: f32,
    opacity: f32
) -> Element<'a, M>
where
    T: Clone + 'a,
    M: Clone + 'a
{
    let mut buttons = controls(font_size);

    for (name, choice, active) in choices {
        buttons = buttons.push(choice_button(
            name,
            to_message(choice),
            active,
            font_size,
            opacity
        ));
    }

    labelled_row(label, buttons.into(), font_size)
}

/// Renders a label followed by a value the window only reports.
///
/// A fact the bar does not own is drawn as text rather than as a control, so
/// the page never offers a button that quietly does nothing when pressed. The
/// value takes the control text size all the same, so a reporting row lines up
/// with an acting one.
///
/// An `indicator` is the moving glyph a row carries while the bar waits on a
/// change it has asked the desktop for. It is drawn in front of the value
/// rather than after it, so the row reads as "something is happening to this"
/// from its first character.
pub fn status_row<'a, M: 'a>(
    label: &'a str,
    value: String,
    indicator: Option<&'static str>,
    font_size: f32
) -> Element<'a, M> {
    let control = style::control_size(font_size);

    let content: Element<'a, M> = match indicator {
        Some(glyph) => controls(font_size)
            .push(icon_raw_sized(glyph.to_owned(), Some(control)))
            .push(text(value).size(control))
            .into(),
        None => text(value).size(control).into()
    };

    labelled_row(label, content, font_size)
}

/// Renders a label with a value that steps down and up.
pub fn stepper_row<'a, M: Clone + 'a>(
    label: &'a str,
    current: String,
    down: M,
    up: M,
    font_size: f32,
    opacity: f32
) -> Element<'a, M> {
    let control = style::control_size(font_size);

    let stepper = controls(font_size)
        .push(choice_button("\u{2212}", down, false, font_size, opacity))
        .push(
            container(text(current).size(control))
                .padding([
                    style::BUTTON_PADDING_EM[0] * control,
                    style::BUTTON_PADDING_EM[1] * control
                ])
                .align_x(Alignment::Center)
        )
        .push(choice_button("+", up, false, font_size, opacity));

    labelled_row(label, stepper.into(), font_size)
}

/// Renders a chip standing for a module placed on the bar.
///
/// A chip is not a button in disguise: it is filled when picked and outlined
/// when not, so the row reads as a picture of the bar rather than as a row of
/// controls.
///
/// `cell` fixes the chip to a grid cell of that width; chips in a grid all
/// take the same cell so the grid keeps its shape whatever the labels
/// measure, while [`None`] leaves the chip the size of its own label for the
/// rows that draw a picture of the bar.
pub fn chip<'a, M: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    message: M,
    picked: bool,
    font_size: f32,
    opacity: f32,
    cell: Option<f32>
) -> Element<'a, M> {
    let control = style::control_size(font_size);

    let mut label = text(label).size(control);
    if cell.is_some() {
        label = label
            .width(iced::Length::Fill)
            .align_x(iced::Alignment::Center);
    }

    let mut chip = button(label).padding([
        style::CHIP_PADDING_EM[0] * control,
        style::CHIP_PADDING_EM[1] * control
    ]);

    if let Some(cell) = cell {
        chip = chip.width(cell);
    }

    chip.on_press(message)
        .style(chip_style(picked, font_size, opacity))
        .into()
}

/// Builds the paint of a chip.
///
/// A chip keeps the same paint under every status: it stands for a module
/// placed on the bar rather than for a control, so hovering one must not make
/// it look like something that is about to happen.
fn chip_style(
    picked: bool,
    font_size: f32,
    opacity: f32
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, _status| {
        let palette = theme.extended_palette();
        let background = if picked {
            palette.primary.base.color
        } else {
            palette.background.weak.color
        };

        button::Style {
            background: Some(Background::Color(background.scale_alpha(opacity))),
            text_color: if picked {
                palette.primary.base.text
            } else {
                palette.background.base.text
            },
            border: Border::default().rounded(style::corner_radius(font_size)),
            ..button::Style::default()
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::{Length, widget::button::Status};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {
        Down,
        Up,
        Picked(u8)
    }

    const FONT: f32 = 14.0;

    fn fill(style: &button::Style) -> Option<iced::Color> {
        match style.background {
            Some(Background::Color(color)) => Some(color),
            _ => None
        }
    }

    #[test]
    fn a_choice_button_hugs_its_label() {
        let choice: Element<'_, Msg> = choice_button("Islands", Msg::Up, false, FONT, 1.0);

        assert_eq!(choice.as_widget().size().width, Length::Shrink);
    }

    #[test]
    fn the_choice_in_force_carries_the_accent_on_its_label() {
        let theme = Theme::Dark;
        let picked = choice_button_style(true, 1.0)(&theme, Status::Active);
        let idle = choice_button_style(false, 1.0)(&theme, Status::Active);

        assert_eq!(
            picked.text_color,
            theme.extended_palette().primary.base.color
        );
        assert_eq!(idle.text_color, theme.palette().text);
    }

    #[test]
    fn a_choice_button_keeps_the_settings_paint_under_every_status() {
        let theme = Theme::Dark;

        for status in [
            Status::Active,
            Status::Hovered,
            Status::Pressed,
            Status::Disabled
        ] {
            assert_eq!(
                fill(&choice_button_style(true, 1.0)(&theme, status)),
                fill(&settings_button_style(1.0)(&theme, status))
            );
        }
    }

    #[test]
    fn a_choice_row_fills_the_width_and_lists_every_choice() {
        let row: Element<'_, Msg> = choice_row(
            "Style",
            vec![("Islands", 0u8, true), ("Solid", 1u8, false)],
            Msg::Picked,
            FONT,
            1.0
        );

        assert_eq!(row.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn an_empty_choice_row_still_draws_its_label() {
        let row: Element<'_, Msg> = choice_row("Style", Vec::new(), Msg::Picked, FONT, 1.0);

        assert_eq!(row.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_status_row_reports_a_plain_value() {
        let row: Element<'_, Msg> = status_row("Theme", "Mocha".to_owned(), None, FONT);

        assert_eq!(row.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_waiting_status_row_leads_with_its_indicator() {
        let row: Element<'_, Msg> = status_row("Theme", "Mocha".to_owned(), Some("\u{f110}"), FONT);

        assert_eq!(row.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_stepper_row_offers_a_step_in_each_direction() {
        let row: Element<'_, Msg> =
            stepper_row("Scale", "1.25".to_owned(), Msg::Down, Msg::Up, FONT, 1.0);

        assert_eq!(row.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_picked_chip_is_filled_with_the_accent() {
        let theme = Theme::Dark;
        let picked = chip_style(true, FONT, 1.0)(&theme, Status::Active);
        let palette = theme.extended_palette();

        assert_eq!(fill(&picked), Some(palette.primary.base.color));
        assert_eq!(picked.text_color, palette.primary.base.text);
    }

    #[test]
    fn an_unpicked_chip_is_filled_with_the_weak_background() {
        let theme = Theme::Dark;
        let idle = chip_style(false, FONT, 1.0)(&theme, Status::Active);
        let palette = theme.extended_palette();

        assert_eq!(fill(&idle), Some(palette.background.weak.color));
        assert_eq!(idle.text_color, palette.background.base.text);
    }

    #[test]
    fn a_chip_looks_the_same_whether_or_not_it_is_hovered() {
        let theme = Theme::Dark;
        let styled = chip_style(true, FONT, 1.0);
        let resting = fill(&styled(&theme, Status::Active));

        for status in [Status::Hovered, Status::Pressed, Status::Disabled] {
            assert_eq!(fill(&styled(&theme, status)), resting);
        }
    }

    #[test]
    fn the_chip_fill_fades_with_the_opacity_it_is_handed() {
        let theme = Theme::Dark;
        let faded = fill(&chip_style(true, FONT, 0.5)(&theme, Status::Active));
        let opaque = fill(&chip_style(true, FONT, 1.0)(&theme, Status::Active));

        assert!(faded.map(|color| color.a) < opaque.map(|color| color.a));
    }

    #[test]
    fn a_chip_without_a_cell_hugs_its_own_label() {
        let chip: Element<'_, Msg> = chip("Clock", Msg::Up, false, FONT, 1.0, None);

        assert_eq!(chip.as_widget().size().width, Length::Shrink);
    }

    #[test]
    fn a_chip_given_a_cell_takes_that_width() {
        let chip: Element<'_, Msg> = chip("Clock", Msg::Up, true, FONT, 1.0, Some(96.0));

        assert_eq!(chip.as_widget().size().width, Length::Fixed(96.0));
    }
}
