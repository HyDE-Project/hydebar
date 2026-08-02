//! The arithmetic of mixing two appearance snapshots.
//!
//! Colours and opacities interpolate channel by channel; discrete settings
//! such as the layout style or the scale factor take the target's value
//! outright, because the compositor side of the shell has to be reconfigured
//! for them anyway.

use hex_color::HexColor;

use crate::config::{Appearance, AppearanceColor, MenuAppearance};

/// Interpolates the animatable parts of an appearance.
pub(super) fn blend_appearance(from: &Appearance, to: &Appearance, t: f32) -> Appearance {
    Appearance {
        font_name:                to.font_name.clone(),
        font_size:                to.font_size,
        radius:                   to.radius,
        height:                   to.height,
        side_padding:             to.side_padding,
        follow_hyde:              to.follow_hyde,
        auto_scale:               to.auto_scale,
        scale_factor:             to.scale_factor,
        style:                    to.style,
        opacity:                  blend_f32(from.opacity, to.opacity, t),
        bar_opacity:              blend_f32(from.bar_opacity, to.bar_opacity, t),
        menu:                     MenuAppearance {
            opacity:  blend_f32(from.menu.opacity, to.menu.opacity, t),
            backdrop: blend_f32(from.menu.backdrop, to.menu.backdrop, t)
        },
        animations:               to.animations.clone(),
        greeting:                 to.greeting,
        background_color:         blend_color(from.background_color, to.background_color, t),
        primary_color:            blend_color(from.primary_color, to.primary_color, t),
        secondary_color:          blend_color(from.secondary_color, to.secondary_color, t),
        success_color:            blend_color(from.success_color, to.success_color, t),
        danger_color:             blend_color(from.danger_color, to.danger_color, t),
        warning_color:            blend_color(from.warning_color, to.warning_color, t),
        text_color:               blend_color(from.text_color, to.text_color, t),
        workspace_colors:         blend_colors(&from.workspace_colors, &to.workspace_colors, t),
        special_workspace_colors: blend_optional_colors(
            from.special_workspace_colors.as_deref(),
            to.special_workspace_colors.as_deref(),
            t
        ),
        island_borders:           to.island_borders,
        window_border:            to.window_border,
        window_shadow:            to.window_shadow
    }
}

fn blend_f32(from: f32, to: f32, t: f32) -> f32 {
    (to - from).mul_add(t, from)
}

fn blend_u8(from: u8, to: u8, t: f32) -> u8 {
    let blended = (f32::from(to) - f32::from(from)).mul_add(t, f32::from(from));

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is rounded and clamped into the u8 range"
    )]
    let channel = blended.round().clamp(0.0, 255.0) as u8;

    channel
}

fn blend_hex(from: HexColor, to: HexColor, t: f32) -> HexColor {
    HexColor::rgba(
        blend_u8(from.r, to.r, t),
        blend_u8(from.g, to.g, t),
        blend_u8(from.b, to.b, t),
        blend_u8(from.a, to.a, t)
    )
}

/// Interpolates two palette entries.
///
/// The target decides which optional shades exist; a shade missing on one side
/// falls back to that side's base colour so the blend never jumps.
fn blend_color(from: AppearanceColor, to: AppearanceColor, t: f32) -> AppearanceColor {
    let (from_base, from_strong, from_weak, from_text) = parts(from);
    let (to_base, to_strong, to_weak, to_text) = parts(to);

    let base = blend_hex(from_base, to_base, t);

    match to {
        AppearanceColor::Simple(_) if from_strong.is_none() && from_weak.is_none() => {
            AppearanceColor::Simple(base)
        }
        _ => AppearanceColor::Complete {
            base,
            strong: blend_optional(from_strong, to_strong, from_base, to_base, t),
            weak: blend_optional(from_weak, to_weak, from_base, to_base, t),
            text: blend_optional(from_text, to_text, from_base, to_base, t)
        }
    }
}

const fn parts(
    color: AppearanceColor
) -> (
    HexColor,
    Option<HexColor>,
    Option<HexColor>,
    Option<HexColor>
) {
    match color {
        AppearanceColor::Simple(base) => (base, None, None, None),
        AppearanceColor::Complete {
            base,
            strong,
            weak,
            text
        } => (base, strong, weak, text)
    }
}

fn blend_optional(
    from: Option<HexColor>,
    to: Option<HexColor>,
    from_base: HexColor,
    to_base: HexColor,
    t: f32
) -> Option<HexColor> {
    match (from, to) {
        (None, None) => None,
        (Some(from), Some(to)) => Some(blend_hex(from, to, t)),
        (Some(from), None) => Some(blend_hex(from, to_base, t)),
        (None, Some(to)) => Some(blend_hex(from_base, to, t))
    }
}

fn blend_colors(from: &[AppearanceColor], to: &[AppearanceColor], t: f32) -> Vec<AppearanceColor> {
    to.iter()
        .enumerate()
        .map(|(index, target)| {
            from.get(index)
                .map_or(*target, |source| blend_color(*source, *target, t))
        })
        .collect()
}

fn blend_optional_colors(
    from: Option<&[AppearanceColor]>,
    to: Option<&[AppearanceColor]>,
    t: f32
) -> Option<Vec<AppearanceColor>> {
    let to = to?;

    Some(blend_colors(from.unwrap_or(&[]), to, t))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(value: u8) -> HexColor {
        HexColor::rgb(value, value, value)
    }

    #[test]
    fn optional_shades_blend_from_the_base_colour() {
        let from = AppearanceColor::Simple(hex(0));
        let to = AppearanceColor::Complete {
            base:   hex(100),
            strong: Some(hex(200)),
            weak:   None,
            text:   None
        };

        let blended = blend_color(from, to, 0.5);

        match blended {
            AppearanceColor::Complete {
                strong, ..
            } => assert_eq!(strong, Some(hex(100))),
            other @ AppearanceColor::Simple(_) => {
                panic!("expected a complete colour, got {other:?}")
            }
        }
    }
}
