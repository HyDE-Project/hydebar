//! Row shapes shared by the pages of the settings window.

use iced::{
    Alignment, Background, Border, Element, Length, Theme,
    widget::{Column, Row, button, container, text}
};

use crate::{modules::settings::Message, style::settings_button_style};

/// Padding of a button inside the window, in multiples of the text size.
const BUTTON_PADDING_EM: [f32; 2] = [0.6, 1.2];

/// Gap between the controls of a row, in multiples of the text size.
pub(super) const ROW_GAP_EM: f32 = 0.8;

/// Renders a compact button carrying `label`.
///
/// An `active` button is tinted with the accent colour, which is how the window
/// shows the choice currently in force.
pub(super) fn choice_button<'a>(
    label: impl text::IntoFragment<'a>,
    message: Message,
    active: bool,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    button(text(label).size(font_size))
        .padding([
            BUTTON_PADDING_EM[0] * font_size,
            BUTTON_PADDING_EM[1] * font_size
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
pub(super) fn choice_row<'a, T>(
    label: &'a str,
    choices: Vec<(&'a str, T, bool)>,
    to_message: impl Fn(T) -> Message + 'a,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message>
where
    T: Clone + 'a
{
    let mut buttons = Row::new().spacing(ROW_GAP_EM * font_size);

    for (name, choice, active) in choices {
        buttons = buttons.push(choice_button(
            name,
            to_message(choice),
            active,
            font_size,
            opacity
        ));
    }

    Row::new()
        .push(text(label).size(font_size).width(Length::Fill))
        .push(buttons)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

/// Renders a label followed by a value the window only reports.
///
/// A fact the bar does not own is drawn as text rather than as a control, so
/// the page never offers a button that quietly does nothing when pressed.
pub(super) fn status_row<'a>(
    label: &'a str,
    value: String,
    font_size: f32
) -> Element<'a, Message> {
    Row::new()
        .push(text(label).size(font_size).width(Length::Fill))
        .push(text(value).size(font_size))
        .spacing(ROW_GAP_EM * font_size)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

/// Renders a label with a value that steps down and up.
pub(super) fn stepper_row<'a>(
    label: &'a str,
    current: String,
    down: Message,
    up: Message,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    Row::new()
        .push(text(label).size(font_size).width(Length::Fill))
        .push(choice_button("−", down, false, font_size, opacity))
        .push(
            container(text(current).size(font_size))
                .padding([BUTTON_PADDING_EM[0] * font_size, 0.5 * font_size])
                .align_x(Alignment::Center)
        )
        .push(choice_button("+", up, false, font_size, opacity))
        .spacing(ROW_GAP_EM * font_size)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

/// Renders a chip standing for a module placed on the bar.
///
/// A chip is not a button in disguise: it is filled when picked and outlined
/// when not, so the row reads as a picture of the bar rather than as a row of
/// controls.
pub(super) fn chip<'a>(
    label: impl text::IntoFragment<'a>,
    message: Message,
    picked: bool,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    button(text(label).size(font_size))
        .padding([0.35 * font_size, 0.7 * font_size])
        .on_press(message)
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
                border: Border::default().rounded(font_size * 0.45),
                ..button::Style::default()
            }
        })
        .into()
}

/// Renders a caption above a group of controls.
pub(super) fn caption<'a>(label: &'a str, font_size: f32) -> Element<'a, Message> {
    text(label).size(font_size * 0.8).into()
}

/// Renders a labelled group of actions, so a button never stands alone without
/// saying what it acts on.
pub(super) fn action_group<'a>(
    label: &'a str,
    actions: Vec<Element<'a, Message>>,
    font_size: f32
) -> Element<'a, Message> {
    let mut row = Row::new().spacing(ROW_GAP_EM * font_size * 0.5);

    for action in actions {
        row = row.push(action);
    }

    Column::new()
        .push(caption(label, font_size))
        .push(row)
        .spacing(ROW_GAP_EM * font_size * 0.35)
        .into()
}

/// Renders a card the detail of a picked entry lives in.
pub(super) fn card<'a>(
    content: Element<'a, Message>,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    container(content)
        .padding(0.8 * font_size)
        .width(Length::Fill)
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity * 0.5)
            )),
            border: Border::default().rounded(font_size * 0.5),
            ..container::Style::default()
        })
        .into()
}
