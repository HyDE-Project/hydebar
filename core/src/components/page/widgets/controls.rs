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
        .style(move |theme: &Theme, status| {
            let mut style = settings_button_style(opacity)(theme, status);

            if active {
                style.text_color = theme.extended_palette().primary.base.color;
            }

            style
        })
        .into()
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
        .style(move |theme: &Theme, _status| {
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
        })
        .into()
}
