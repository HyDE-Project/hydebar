//! A whole theme faded to a share of its alpha.
//!
//! Serves the surface of a menu mid-fade: the box background is the only
//! thing the menu style itself fades, and text, icons and buttons drawn from
//! the palette would otherwise stand fully opaque until the surface empties.
//! Fading the whole palette makes the entire window travel as one.

use iced::{
    Theme,
    theme::{Palette, palette}
};

/// The same theme with every palette colour faded to `share` of its alpha.
///
/// Serves the surface of a menu mid-fade: the box background is the only
/// thing the menu style itself fades, and text, icons and buttons drawn from
/// the palette would otherwise stand fully opaque until the surface empties.
/// Fading the whole palette makes the entire window travel as one.
#[must_use]
pub fn faded_theme(base: &Theme, share: f32) -> Theme {
    let share = share.clamp(0.0, 1.0);
    let faded = fade_extended(base.extended_palette(), share);

    Theme::custom_with_fn(
        "local-fading".to_string(),
        fade_palette(base.palette(), share),
        move |_| faded
    )
}

fn fade_palette(palette: Palette, share: f32) -> Palette {
    Palette {
        background: palette.background.scale_alpha(share),
        text:       palette.text.scale_alpha(share),
        primary:    palette.primary.scale_alpha(share),
        success:    palette.success.scale_alpha(share),
        warning:    palette.warning.scale_alpha(share),
        danger:     palette.danger.scale_alpha(share)
    }
}

fn fade_extended(extended: &palette::Extended, share: f32) -> palette::Extended {
    let pair = |pair: palette::Pair| palette::Pair {
        color: pair.color.scale_alpha(share),
        text:  pair.text.scale_alpha(share)
    };

    palette::Extended {
        background: palette::Background {
            base:      pair(extended.background.base),
            weakest:   pair(extended.background.weakest),
            weaker:    pair(extended.background.weaker),
            weak:      pair(extended.background.weak),
            neutral:   pair(extended.background.neutral),
            strong:    pair(extended.background.strong),
            stronger:  pair(extended.background.stronger),
            strongest: pair(extended.background.strongest)
        },
        primary:    palette::Primary {
            base:   pair(extended.primary.base),
            weak:   pair(extended.primary.weak),
            strong: pair(extended.primary.strong)
        },
        secondary:  palette::Secondary {
            base:   pair(extended.secondary.base),
            weak:   pair(extended.secondary.weak),
            strong: pair(extended.secondary.strong)
        },
        success:    palette::Success {
            base:   pair(extended.success.base),
            weak:   pair(extended.success.weak),
            strong: pair(extended.success.strong)
        },
        warning:    palette::Warning {
            base:   pair(extended.warning.base),
            weak:   pair(extended.warning.weak),
            strong: pair(extended.warning.strong)
        },
        danger:     palette::Danger {
            base:   pair(extended.danger.base),
            weak:   pair(extended.danger.weak),
            strong: pair(extended.danger.strong)
        },
        is_dark:    extended.is_dark
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::{config::Appearance, style::theme::hydebar_theme};

    #[test]
    fn a_faded_theme_scales_every_alpha_and_nothing_else() {
        let base = hydebar_theme(&Appearance::default());

        let faded = faded_theme(&base, 0.5);

        let text = faded.palette().text;
        assert_eq!(text.a, base.palette().text.a * 0.5);
        assert_eq!(text.r, base.palette().text.r);

        let weak = faded.extended_palette().background.weak;
        assert_eq!(
            weak.color.a,
            base.extended_palette().background.weak.color.a * 0.5
        );
        assert_eq!(
            weak.color.g,
            base.extended_palette().background.weak.color.g
        );
    }

    #[test]
    fn a_full_fade_share_leaves_the_theme_untouched() {
        let base = hydebar_theme(&Appearance::default());

        let same = faded_theme(&base, 1.0);

        assert_eq!(same.palette(), base.palette());
        assert_eq!(
            same.extended_palette().background.strong,
            base.extended_palette().background.strong
        );
    }
}
