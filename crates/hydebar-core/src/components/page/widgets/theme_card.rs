//! What a theme card knows about itself: its states, its paint and the
//! small shapes — palette dots and the busy strip — drawn from them.

use iced::{
    Background, Border, Element, Length, Theme,
    widget::{Row, Space, container}
};

use crate::modules::themes::Spinner;

/// Share of its colour a control that cannot be pressed right now is drawn at.
///
/// Low enough to read as unavailable at a glance, high enough that the label
/// stays legible: a chip the user cannot press still has to say which theme it
/// stands for, or the grid turns into a row of blanks while a switch runs.
const BLOCKED_ALPHA: f32 = 0.35;

/// The colours a theme chip is painted in, taken from the theme it stands for.
///
/// Carried whole rather than as a background alone: a background from one
/// palette under a text colour from another is exactly the unreadable pairing
/// the swatch exists to avoid.
#[derive(Debug, Clone, PartialEq)]
pub struct ChipPaint {
    /// Surface of the chip.
    pub(crate) background: iced::Color,
    /// Text on that surface.
    pub(crate) text:       iced::Color,
    /// Accent the active theme is ringed with.
    pub(crate) accent:     iced::Color,
    /// The theme's own colours, drawn as a row of dots under the name.
    ///
    /// One surface colour cannot tell two dark themes apart; the dots draw
    /// the palette itself, the way any theme picker worth its name does.
    /// However many the source honestly knows: four from a wallpaper
    /// swatch, two from the catalogue's announcement.
    pub(crate) palette:    Vec<iced::Color>
}

/// Diameter of one palette dot, in multiples of the control text size.
const DOT_EM: f32 = 0.55;

/// Gap between two palette dots, in multiples of the control text size.
pub(super) const DOT_GAP_EM: f32 = 0.35;

/// Room the dot row adds under the name, in multiples of the control size.
///
/// The dot itself plus the spacing the column keeps above it; the menu height
/// estimate reads this so a grid of painted chips is measured as tall as it
/// draws.
pub const DOT_ROW_EM: f32 = DOT_EM + DOT_GAP_EM;

/// Share of the menu fade in force, read off the palette the widget is drawn
/// with.
///
/// The menu wrapper animates a window by fading its whole palette, so a colour
/// taken from the palette travels on its own. A colour taken from a theme's
/// file does not — and the palette's text alpha, one whenever the window
/// rests, is where the travelled share can be read back so those colours
/// follow the same fade.
fn fade_share(theme: &Theme) -> f32 {
    theme.palette().text.a
}

/// Renders the palette of a theme as a row of coloured dots.
pub(super) fn palette_dots<'a, M: 'a>(
    palette: Vec<iced::Color>,
    control: f32
) -> Element<'a, M> {
    let dot = DOT_EM * control;
    let mut row = Row::new()
        .spacing(DOT_GAP_EM * control)
        .align_y(iced::Alignment::Center);

    for colour in palette {
        row = row.push(container(Space::new().width(dot).height(dot)).style(
            move |theme: &Theme| container::Style {
                background: Some(Background::Color(colour.scale_alpha(fade_share(theme)))),
                border: Border::default().rounded(dot / 2.0),
                ..container::Style::default()
            }
        ));
    }

    container(row)
        .width(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
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
pub enum ThemeChip {
    /// The theme the desktop is on.
    Active,
    /// A theme the desktop can be switched to.
    Idle,
    /// The theme the running switch is on its way to.
    Applying(Spinner),
    /// A theme that cannot be pressed while another switch runs.
    Blocked,
    /// A theme whose removal waits for one confirming press.
    Condemned,
    /// A card that answers no press of its own.
    ///
    /// The gallery wears it: applying and removing belong to installed
    /// themes, installing belongs to the download button alone, so the
    /// card body itself has nothing to say to a click.
    Inert
}

impl ThemeChip {
    /// Whether a press on this chip is allowed to start a switch.
    ///
    /// A chip that cannot start one carries no press handler at all, so the
    /// refusal is something the pointer meets rather than something the module
    /// has to log after the fact.
    /// Whether the chip answers a press.
    ///
    /// A blocked chip stays pressable on purpose: a press mid-switch is
    /// queued to run next rather than silently thrown away, so the module
    /// has to hear about it. Only the chip already being applied stays deaf
    /// — pressing it could mean nothing new.
    pub(crate) const fn is_pressable(self) -> bool {
        matches!(
            self,
            Self::Active | Self::Idle | Self::Condemned | Self::Blocked
        )
    }
}

/// Surface, ink and whether a ring is due, for a card in `state`.
pub(super) fn card_colors(
    theme: &Theme,
    paint_colors: Option<(iced::Color, iced::Color, iced::Color)>,
    state: ThemeChip
) -> (iced::Color, iced::Color, bool) {
    let palette = theme.extended_palette();

    let (base, text_colour) = match paint_colors {
        Some((background, text, _)) => {
            let share = fade_share(theme);

            (background.scale_alpha(share), text.scale_alpha(share))
        }
        None => (palette.background.weak.color, palette.background.base.text)
    };

    match state {
        ThemeChip::Active => match paint_colors {
            Some(_) => (base, text_colour, true),
            None => (palette.primary.base.color, palette.primary.base.text, false)
        },
        ThemeChip::Idle | ThemeChip::Inert => (base, text_colour, false),
        ThemeChip::Applying(spinner) => match paint_colors {
            Some(_) => (base.scale_alpha(spinner.pulse()), text_colour, true),
            None => (
                palette.primary.base.color.scale_alpha(spinner.pulse()),
                palette.primary.base.text,
                false
            )
        },
        ThemeChip::Blocked => (
            base.scale_alpha(BLOCKED_ALPHA),
            text_colour.scale_alpha(BLOCKED_ALPHA),
            false
        ),
        ThemeChip::Condemned => (base, text_colour, true)
    }
}

/// The colour a card's ring is drawn in.
pub(super) fn card_ring(
    theme: &Theme,
    paint_colors: Option<(iced::Color, iced::Color, iced::Color)>,
    state: ThemeChip
) -> iced::Color {
    if state == ThemeChip::Condemned {
        return theme.palette().danger;
    }

    match paint_colors {
        Some((_, _, accent)) => accent.scale_alpha(fade_share(theme)),
        None => theme.extended_palette().primary.base.color
    }
}

/// A strip sweeping under a chip that is being worked on.
///
/// Indeterminate on purpose: the desktop's own importer publishes no
/// percentages, and a bar with invented numbers would be lying. The sweep
/// rides the same spinner phase the glyph indicator uses, so one clock
/// serves both.
pub(super) fn busy_strip<'a, M: 'a>(spinner: Spinner, control: f32) -> Element<'a, M> {
    let phase = spinner.pulse();
    let height = DOT_EM * control * 0.5;

    container(Space::new().width(Length::Fill).height(height))
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                    .add_stop(0.0, theme.palette().primary.scale_alpha(0.15))
                    .add_stop(phase.clamp(0.05, 0.95), theme.palette().primary)
                    .add_stop(1.0, theme.palette().primary.scale_alpha(0.15))
            ))),
            border: Border::default().rounded(height / 2.0),
            ..container::Style::default()
        })
        .width(Length::Fill)
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
    fn a_blocked_chip_still_hears_the_press_for_the_queue() {
        assert!(ThemeChip::Blocked.is_pressable());
    }

    #[test]
    #[expect(
        clippy::assertions_on_constants,
        reason = "the test pins the constant inside its documented range"
    )]
    fn a_blocked_chip_is_dimmed_but_not_erased() {
        assert!(BLOCKED_ALPHA > 0.0);
        assert!(BLOCKED_ALPHA < 1.0);
    }
}
