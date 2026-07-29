//! Row shapes shared by the pages of the settings window.
//!
//! Every shape reads its sizes from [`super::style`] and every caller passes
//! the page text size rather than a scaled one, so a row drawn on one tab is
//! the same height, carries the same text size and starts its controls at the
//! same x as the matching row on the next tab.

use iced::{
    Alignment, Background, Border, Element, Length, Theme,
    widget::{Column, Row, button, container, text}
};

use super::style;
use crate::{
    components::icons::icon_raw_sized,
    modules::settings::{Message, Spinner},
    style::settings_button_style
};

/// Share of its colour a control that cannot be pressed right now is drawn at.
///
/// Low enough to read as unavailable at a glance, high enough that the label
/// stays legible: a chip the user cannot press still has to say which theme it
/// stands for, or the grid turns into a row of blanks while a switch runs.
const BLOCKED_ALPHA: f32 = 0.35;

/// Renders a compact button carrying `label`.
///
/// `font_size` is the page text size, not the button's: the button scales it
/// down itself so no caller can hand it a size of its own invention.
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
pub(super) fn status_row<'a>(
    label: &'a str,
    value: String,
    indicator: Option<&'static str>,
    font_size: f32
) -> Element<'a, Message> {
    let control = style::control_size(font_size);

    let content: Element<'a, Message> = match indicator {
        Some(glyph) => controls(font_size)
            .push(icon_raw_sized(glyph.to_owned(), Some(control)))
            .push(text(value).size(control))
            .into(),
        None => text(value).size(control).into()
    };

    labelled_row(label, content, font_size)
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
pub(super) fn chip<'a>(
    label: impl text::IntoFragment<'a>,
    message: Message,
    picked: bool,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let control = style::control_size(font_size);

    button(text(label).size(control))
        .padding([
            style::CHIP_PADDING_EM[0] * control,
            style::CHIP_PADDING_EM[1] * control
        ])
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
                border: Border::default().rounded(style::corner_radius(font_size)),
                ..button::Style::default()
            }
        })
        .into()
}

/// What a chip of the theme grid stands for right now.
///
/// The grid is the only place on the page where a press starts something the
/// bar cannot take back or hurry, so the four cases are named rather than
/// squeezed into a pair of flags: a chip is the theme in force, a theme that
/// can be switched to, the theme being applied, or a theme that cannot be
/// pressed because another one is being applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ThemeChip {
    /// The theme the desktop is on.
    Active,
    /// A theme the desktop can be switched to.
    Idle,
    /// The theme the running switch is on its way to.
    Applying(Spinner),
    /// A theme that cannot be pressed while another switch runs.
    Blocked
}

impl ThemeChip {
    /// Whether a press on this chip is allowed to start a switch.
    ///
    /// A chip that cannot start one carries no press handler at all, so the
    /// refusal is something the pointer meets rather than something the module
    /// has to log after the fact.
    pub(super) fn is_pressable(self) -> bool {
        matches!(self, Self::Active | Self::Idle)
    }
}

/// Renders a chip of the theme grid.
///
/// Kept apart from [`chip`], which draws a picture of the bar and is never
/// blocked, because this one has to say three things at once: which theme the
/// desktop is on, which one it is moving to, and that nothing else can be asked
/// for until it gets there.
pub(super) fn theme_chip<'a>(
    label: String,
    message: Message,
    state: ThemeChip,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let control = style::control_size(font_size);

    let mut chip = button(text(label).size(control)).padding([
        style::CHIP_PADDING_EM[0] * control,
        style::CHIP_PADDING_EM[1] * control
    ]);

    if state.is_pressable() {
        chip = chip.on_press(message);
    }

    chip.style(move |theme: &Theme, _status| {
        let palette = theme.extended_palette();

        let (background, text_color) = match state {
            ThemeChip::Active => (palette.primary.base.color, palette.primary.base.text),
            ThemeChip::Idle => (palette.background.weak.color, palette.background.base.text),
            ThemeChip::Applying(spinner) => (
                palette.primary.base.color.scale_alpha(spinner.pulse()),
                palette.primary.base.text
            ),
            ThemeChip::Blocked => (
                palette.background.weak.color.scale_alpha(BLOCKED_ALPHA),
                palette.background.base.text.scale_alpha(BLOCKED_ALPHA)
            )
        };

        button::Style {
            background: Some(Background::Color(background.scale_alpha(opacity))),
            text_color,
            border: Border::default().rounded(style::corner_radius(font_size)),
            ..button::Style::default()
        }
    })
    .into()
}

/// Renders a note: a sentence a page states in place of a control it cannot
/// offer, such as a section that holds nothing yet.
///
/// Drawn smaller than a value so it reads as an aside rather than as something
/// the bar is reporting.
pub(super) fn note<'a>(
    label: impl text::IntoFragment<'a>,
    font_size: f32
) -> Element<'a, Message> {
    text(label).size(style::caption_size(font_size)).into()
}

/// Renders the heading of a section.
///
/// Every heading on every tab comes from here, so no page can invent a heading
/// of its own size or weight.
fn section_title<'a>(label: impl text::IntoFragment<'a>, font_size: f32) -> Element<'a, Message> {
    text(label)
        .size(style::section_title_size(font_size))
        .into()
}

/// Renders a titled section: the heading, then what it holds.
///
/// A page is a stack of these and nothing else, which is what makes the three
/// tabs read as one window.
pub(super) fn section<'a>(
    title: impl text::IntoFragment<'a>,
    content: Element<'a, Message>,
    font_size: f32
) -> Element<'a, Message> {
    Column::new()
        .push(section_title(title, font_size))
        .push(content)
        .spacing(style::caption_gap(font_size))
        .width(Length::Fill)
        .into()
}

/// Starts the column a page is stacked in.
///
/// Handed out ready-spaced and ready-padded so a page states what it holds and
/// never how far apart it holds it.
pub(super) fn page<'a>(font_size: f32) -> Column<'a, Message> {
    Column::new()
        .spacing(style::section_gap(font_size))
        .padding(style::page_padding(font_size))
        .width(Length::Fill)
}

/// Starts the column the rows of one section are stacked in.
pub(super) fn rows<'a>(font_size: f32) -> Column<'a, Message> {
    Column::new()
        .spacing(style::page_gap(font_size))
        .width(Length::Fill)
}

/// Starts the column a wrapping grid of chips is stacked in.
///
/// Chips sit closer together than rows do, in both directions, so a grid reads
/// as one block rather than as a stack of unrelated rows.
pub(super) fn grid<'a>(font_size: f32) -> Column<'a, Message> {
    Column::new()
        .spacing(style::group_gap(font_size))
        .width(Length::Fill)
}

/// Starts the row the controls of one setting sit in.
pub(super) fn controls<'a>(font_size: f32) -> Row<'a, Message> {
    Row::new()
        .spacing(style::row_gap(font_size))
        .align_y(Alignment::Center)
}

/// Starts the row items that belong together sit in, such as the chips of an
/// island or the buttons of one action.
pub(super) fn group<'a>(font_size: f32) -> Row<'a, Message> {
    Row::new()
        .spacing(style::group_gap(font_size))
        .align_y(Alignment::Center)
}

/// Renders a labelled row whose label is not known until it is drawn, such as
/// the number of an island.
///
/// Shares the label column with every other row on every other tab, so the
/// islands of the module page line up with the steppers of the appearance page.
pub(super) fn labelled_row<'a>(
    label: impl text::IntoFragment<'a>,
    content: Element<'a, Message>,
    font_size: f32
) -> Element<'a, Message> {
    Row::new()
        .push(
            text(label)
                .size(font_size)
                .width(Length::Fixed(style::label_width(font_size)))
        )
        .push(content)
        .spacing(style::row_gap(font_size))
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

/// Renders a card the detail of a picked entry lives in.
pub(super) fn card<'a>(
    content: Element<'a, Message>,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    container(content)
        .padding(style::card_padding(font_size))
        .width(Length::Fill)
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity * style::CARD_FILL_ALPHA)
            )),
            border: Border::default().rounded(style::corner_radius(font_size)),
            ..container::Style::default()
        })
        .into()
}

/// Renders an outlined box around `content`.
///
/// The outline carries the same padding and the same corner as a card, so an
/// island on the module page sits on the same grid as the card below it.
pub(super) fn outlined<'a>(
    content: Element<'a, Message>,
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    container(content)
        .padding(style::card_padding(font_size))
        .style(move |theme: &Theme| container::Style {
            border: Border {
                width:  style::BORDER_WIDTH,
                radius: style::corner_radius(font_size).into(),
                color:  theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color
                    .scale_alpha(opacity)
            },
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_that_can_be_switched_to_takes_a_press() {
        assert!(ThemeChip::Idle.is_pressable());
    }

    #[test]
    fn the_theme_in_force_takes_a_press() {
        assert!(ThemeChip::Active.is_pressable());
    }

    /// A press on the theme already being applied would be dropped by the
    /// module anyway; refusing it at the chip is what makes the refusal
    /// something the user can see before clicking rather than after.
    #[test]
    fn the_theme_being_applied_takes_no_press() {
        assert!(!ThemeChip::Applying(Spinner::default()).is_pressable());
    }

    #[test]
    fn a_theme_blocked_by_a_running_switch_takes_no_press() {
        assert!(!ThemeChip::Blocked.is_pressable());
    }

    #[test]
    fn a_blocked_chip_is_dimmed_but_not_erased() {
        assert!(BLOCKED_ALPHA > 0.0);
        assert!(BLOCKED_ALPHA < 1.0);
    }
}
