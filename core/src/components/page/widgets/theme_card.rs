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
#[cfg_attr(coverage_nightly, coverage(off))]
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod paint_tests {
    #![allow(clippy::float_cmp)]

    use iced::Color;

    use super::*;

    const PAINT: (Color, Color, Color) = (
        Color::from_rgb(0.1, 0.1, 0.1),
        Color::from_rgb(0.9, 0.9, 0.9),
        Color::from_rgb(0.5, 0.0, 0.5)
    );

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {}

    fn applying() -> ThemeChip {
        ThemeChip::Applying(Spinner::default())
    }

    #[test]
    fn a_resting_window_has_travelled_its_whole_fade() {
        assert_eq!(fade_share(&Theme::Dark), Theme::Dark.palette().text.a);
    }

    #[test]
    fn an_unpainted_active_card_takes_the_accent_fill_and_no_ring() {
        let theme = Theme::Dark;
        let (background, text, ringed) = card_colors(&theme, None, ThemeChip::Active);
        let palette = theme.extended_palette();

        assert_eq!(background, palette.primary.base.color);
        assert_eq!(text, palette.primary.base.text);
        assert!(!ringed);
    }

    #[test]
    fn a_painted_active_card_keeps_its_own_colours_and_gains_a_ring() {
        let theme = Theme::Dark;
        let (background, text, ringed) = card_colors(&theme, Some(PAINT), ThemeChip::Active);

        assert_eq!(background, PAINT.0.scale_alpha(fade_share(&theme)));
        assert_eq!(text, PAINT.1.scale_alpha(fade_share(&theme)));
        assert!(ringed);
    }

    #[test]
    fn an_idle_and_an_inert_card_look_alike() {
        let theme = Theme::Dark;

        assert_eq!(
            card_colors(&theme, Some(PAINT), ThemeChip::Idle),
            card_colors(&theme, Some(PAINT), ThemeChip::Inert)
        );
        assert!(!card_colors(&theme, Some(PAINT), ThemeChip::Idle).2);
    }

    #[test]
    fn an_unpainted_idle_card_falls_back_to_the_weak_background() {
        let theme = Theme::Dark;
        let (background, text, _) = card_colors(&theme, None, ThemeChip::Idle);
        let palette = theme.extended_palette();

        assert_eq!(background, palette.background.weak.color);
        assert_eq!(text, palette.background.base.text);
    }

    #[test]
    fn a_card_being_applied_pulses_its_fill_and_keeps_its_ink() {
        let theme = Theme::Dark;
        let resting = card_colors(&theme, Some(PAINT), ThemeChip::Idle);
        let busy = card_colors(&theme, Some(PAINT), applying());

        assert_eq!(busy.0.a, resting.0.a * Spinner::default().pulse());
        assert_eq!(busy.1, resting.1);
        assert!(busy.2);
    }

    #[test]
    fn an_unpainted_card_being_applied_pulses_the_accent() {
        let theme = Theme::Dark;
        let busy = card_colors(&theme, None, applying());
        let palette = theme.extended_palette();

        assert_eq!(
            busy.0,
            palette.primary.base.color.scale_alpha(Spinner::default().pulse())
        );
        assert_eq!(busy.1, palette.primary.base.text);
        assert!(!busy.2);
    }

    #[test]
    fn a_blocked_card_fades_both_its_fill_and_its_ink() {
        let theme = Theme::Dark;
        let idle = card_colors(&theme, Some(PAINT), ThemeChip::Idle);
        let blocked = card_colors(&theme, Some(PAINT), ThemeChip::Blocked);

        assert_eq!(blocked.0.a, idle.0.a * BLOCKED_ALPHA);
        assert_eq!(blocked.1.a, idle.1.a * BLOCKED_ALPHA);
        assert!(!blocked.2);
    }

    #[test]
    fn a_condemned_card_keeps_its_colours_and_gains_a_ring() {
        let theme = Theme::Dark;
        let idle = card_colors(&theme, Some(PAINT), ThemeChip::Idle);
        let condemned = card_colors(&theme, Some(PAINT), ThemeChip::Condemned);

        assert_eq!((condemned.0, condemned.1), (idle.0, idle.1));
        assert!(condemned.2);
    }

    #[test]
    fn a_condemned_card_is_ringed_in_danger_whatever_it_is_painted_with() {
        let theme = Theme::Dark;

        assert_eq!(
            card_ring(&theme, Some(PAINT), ThemeChip::Condemned),
            theme.palette().danger
        );
        assert_eq!(
            card_ring(&theme, None, ThemeChip::Condemned),
            theme.palette().danger
        );
    }

    #[test]
    fn a_painted_card_is_ringed_in_its_own_accent() {
        let theme = Theme::Dark;

        assert_eq!(
            card_ring(&theme, Some(PAINT), ThemeChip::Active),
            PAINT.2.scale_alpha(fade_share(&theme))
        );
    }

    #[test]
    fn an_unpainted_card_is_ringed_in_the_bar_accent() {
        let theme = Theme::Dark;

        assert_eq!(
            card_ring(&theme, None, ThemeChip::Active),
            theme.extended_palette().primary.base.color
        );
    }

    #[test]
    fn the_dot_row_reserves_the_dot_and_the_gap_above_it() {
        assert_eq!(DOT_ROW_EM, DOT_EM + DOT_GAP_EM);
    }

    #[test]
    fn a_palette_row_fills_the_width_it_is_given() {
        let dots: Element<'_, Msg> =
            palette_dots(vec![Color::WHITE, Color::BLACK, Color::from_rgb(1.0, 0.0, 0.0)], 14.0);

        assert_eq!(dots.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn a_theme_that_announces_no_palette_still_draws_a_row() {
        let dots: Element<'_, Msg> = palette_dots(Vec::new(), 14.0);

        assert_eq!(dots.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn the_busy_strip_spans_the_card() {
        let strip: Element<'_, Msg> = busy_strip(Spinner::default(), 14.0);

        assert_eq!(strip.as_widget().size().width, Length::Fill);
    }

    #[test]
    fn the_busy_strip_is_drawn_at_every_phase_of_the_cycle() {
        let mut spinner = Spinner::default();

        for _ in 0..Spinner::cycle() {
            let strip: Element<'_, Msg> = busy_strip(spinner, 14.0);
            assert_eq!(strip.as_widget().size().width, Length::Fill);
            spinner.advance();
        }
    }
}
