//! OKLCH arithmetic for perceptually even colour blends.
//!
//! Mixing two saturated colours channel by channel in sRGB drags the
//! midpoint through a muddy, sunken middle: the straight line between two
//! vivid hues leaves the gamut's bright edge and passes through its grey
//! core. OKLCH states a colour as lightness, chroma and hue, so a blend can
//! travel the hue wheel the short way at a lightness that stays where the
//! eye expects it — the same space the `HyDE` palette is meant to move in.
//!
//! The conversions are Björn Ottosson's reference sRGB↔OKLab pair; the mix
//! follows the CSS Color 4 hue rules: an achromatic colour has no hue of its
//! own and borrows the other's, so fading from grey to red tints up instead
//! of wheeling around the circle. A mixed colour that left the sRGB box is
//! pulled back by reducing its chroma, which keeps the lightness and the
//! hue the eye locked onto.

use std::f32::consts::{PI, TAU};

use hex_color::HexColor;

/// Chroma below which a colour counts as achromatic.
///
/// Its hue is numerically meaningless at that point — a hue of a grey is
/// whatever the rounding left behind — so it must never steer a blend.
const ACHROMATIC_CHROMA: f32 = 0.0025;

/// How far the binary search may stay from the gamut edge it is hunting.
const GAMUT_TOLERANCE: f32 = 1.0 / 512.0;

/// Halvings of the chroma window enough to reach [`GAMUT_TOLERANCE`].
///
/// The search starts below one full turn of chroma, so this many halvings
/// land inside the tolerance for certain and the loop needs no float
/// comparison of its own.
const GAMUT_SEARCH_STEPS: usize = 20;

/// A colour as lightness, chroma and hue, the way the eye compares colours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Oklch {
    l: f32,
    c: f32,
    h: f32
}

impl Oklch {
    /// Reads an 8-bit sRGB colour into the perceptual space.
    pub(super) fn from_rgb(color: HexColor) -> Self {
        let (lab_l, lab_a, lab_b) = oklab_of(color);

        Self {
            l: lab_l,
            c: lab_a.hypot(lab_b),
            h: lab_b.atan2(lab_a)
        }
    }

    /// Returns the colour as 8-bit sRGB, reducing chroma until it fits.
    pub(super) fn to_rgb(self) -> HexColor {
        let mut low = 0.0_f32;
        let mut high = self.c;

        if self.fits_in_gamut(high) {
            return self.to_rgb_at(high);
        }

        for _ in 0..GAMUT_SEARCH_STEPS {
            let middle = f32::midpoint(low, high);

            if self.fits_in_gamut(middle) {
                low = middle;
            } else {
                high = middle;
            }
        }

        self.to_rgb_at(low)
    }

    /// Travels from this colour to `other` by the share `t`, taking the hue
    /// the short way round and letting an achromatic end borrow the other's
    /// hue.
    pub(super) fn mix(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let (hue_from, hue_to) = match (self.c >= ACHROMATIC_CHROMA, other.c >= ACHROMATIC_CHROMA)
        {
            (true, true) => (self.h, other.h),
            (true, false) => (self.h, self.h),
            (false, true) => (other.h, other.h),
            (false, false) => (0.0, 0.0)
        };
        let hue = shortest_hue_delta(hue_from, hue_to).mul_add(t, hue_from);

        Self {
            l: (other.l - self.l).mul_add(t, self.l),
            c: (other.c - self.c).mul_add(t, self.c),
            h: normalise_hue(hue)
        }
    }

    /// Whether the colour at this chroma (and this lightness and hue) fits
    /// inside the sRGB box.
    fn fits_in_gamut(self, chroma: f32) -> bool {
        let (red, green, blue) =
            linear_srgb_of(self.l, chroma * self.h.cos(), chroma * self.h.sin());

        (-GAMUT_TOLERANCE..=1.0 + GAMUT_TOLERANCE).contains(&red)
            && (-GAMUT_TOLERANCE..=1.0 + GAMUT_TOLERANCE).contains(&green)
            && (-GAMUT_TOLERANCE..=1.0 + GAMUT_TOLERANCE).contains(&blue)
    }

    /// Encodes the colour at this chroma, assuming it already fits.
    fn to_rgb_at(self, chroma: f32) -> HexColor {
        let (red, green, blue) =
            linear_srgb_of(self.l, chroma * self.h.cos(), chroma * self.h.sin());

        HexColor::rgb(encode(red), encode(green), encode(blue))
    }
}

/// The shortest signed way from one hue to another, in radians.
fn shortest_hue_delta(from: f32, to: f32) -> f32 {
    normalise_hue(to - from)
}

/// Wraps a hue into `[-PI, PI)`.
fn normalise_hue(hue: f32) -> f32 {
    let wrapped = hue % TAU;

    if wrapped > PI {
        wrapped - TAU
    } else if wrapped < -PI {
        wrapped + TAU
    } else {
        wrapped
    }
}

fn cube(v: f32) -> f32 {
    v * v * v
}

fn srgb_to_linear(channel: f32) -> f32 {
    if channel >= 0.04045 {
        ((channel + 0.055) / 1.055).powf(2.4)
    } else {
        channel / 12.92
    }
}

fn linear_to_srgb(channel: f32) -> f32 {
    if channel >= 0.003_130_8 {
        1.055f32.mul_add(channel.powf(1.0 / 2.4), -0.055)
    } else {
        12.92 * channel
    }
}

/// The `OKLab` coordinates of an 8-bit sRGB colour.
fn oklab_of(color: HexColor) -> (f32, f32, f32) {
    let red = srgb_to_linear(f32::from(color.r) / 255.0);
    let green = srgb_to_linear(f32::from(color.g) / 255.0);
    let blue = srgb_to_linear(f32::from(color.b) / 255.0);

    let long =
        (0.412_221_5f32.mul_add(red, 0.536_332_5f32.mul_add(green, 0.051_446 * blue))).cbrt();
    let medium =
        (0.211_903_5f32.mul_add(red, 0.680_699_5f32.mul_add(green, 0.107_397 * blue))).cbrt();
    let short =
        (0.088_302_5f32.mul_add(red, 0.281_718_8f32.mul_add(green, 0.629_978_7 * blue))).cbrt();

    (
        0.210_454_3f32.mul_add(long, 0.793_617_8f32.mul_add(medium, -0.004_072 * short)),
        1.977_998_5f32.mul_add(long, (-2.428_592_2f32).mul_add(medium, 0.450_593_7 * short)),
        0.025_904f32.mul_add(long, 0.782_771_8f32.mul_add(medium, -0.808_675_8 * short))
    )
}

/// The linear sRGB channels an `OKLab` colour drives, before the encoding
/// curve and the clamping into bytes.
fn linear_srgb_of(lab_l: f32, lab_a: f32, lab_b: f32) -> (f32, f32, f32) {
    let long = cube(0.396_337_8f32.mul_add(lab_a, 0.215_803_8f32.mul_add(lab_b, lab_l)));
    let medium = cube((-0.105_561_3f32).mul_add(lab_a, (-0.063_854_2f32).mul_add(lab_b, lab_l)));
    let short = cube((-0.089_484_2f32).mul_add(lab_a, (-1.291_485_5f32).mul_add(lab_b, lab_l)));

    let red = 4.076_741_5f32.mul_add(long, (-3.307_711_5f32).mul_add(medium, 0.230_969_9 * short));
    let green =
        (-1.268_438f32).mul_add(long, 2.609_757_4f32.mul_add(medium, -0.341_319_4 * short));
    let blue =
        (-0.004_196_1f32).mul_add(long, (-0.703_418_6f32).mul_add(medium, 1.707_614_7 * short));

    (red, green, blue)
}

/// Clamps a linear channel and encodes it back into 8-bit sRGB.
fn encode(linear: f32) -> u8 {
    let encoded = linear_to_srgb(linear.clamp(0.0, 1.0)) * 255.0;

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the encoded channel is clamped into the u8 range"
    )]
    let byte = encoded.round().clamp(0.0, 255.0) as u8;

    byte
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> HexColor {
        HexColor::rgb(r, g, b)
    }

    #[test]
    fn a_round_trip_through_the_perceptual_space_keeps_the_colour() {
        for color in [
            rgb(30, 30, 46),
            rgb(203, 166, 247),
            rgb(255, 0, 0),
            rgb(0, 255, 0),
            rgb(0, 0, 255),
            rgb(255, 255, 255),
            rgb(0, 0, 0)
        ] {
            let back = Oklch::from_rgb(color).to_rgb();

            assert!(
                back.r.abs_diff(color.r) <= 1
                    && back.g.abs_diff(color.g) <= 1
                    && back.b.abs_diff(color.b) <= 1,
                "{color:?} came back as {back:?}"
            );
        }
    }

    #[test]
    fn the_ends_of_a_mix_are_the_colours_it_mixes() {
        let from = Oklch::from_rgb(rgb(30, 30, 46));
        let to = Oklch::from_rgb(rgb(203, 166, 247));

        let start = from.mix(to, 0.0).to_rgb();
        let end = from.mix(to, 1.0).to_rgb();

        assert_eq!((start.r, start.g, start.b), (30, 30, 46));
        assert_eq!((end.r, end.g, end.b), (203, 166, 247));
    }

    #[test]
    fn a_mix_keeps_the_lightness_the_eye_expects() {
        let dark = Oklch::from_rgb(rgb(30, 30, 46));
        let light = Oklch::from_rgb(rgb(227, 227, 240));
        let middle = dark.mix(light, 0.5);

        let expected = f32::midpoint(dark.l, light.l);

        assert!((middle.l - expected).abs() < 1e-4);
    }

    #[test]
    fn the_hue_travels_the_short_way_round() {
        let just_left = normalise_hue(175_f32.to_radians());
        let just_right = normalise_hue(-175_f32.to_radians());

        let delta = shortest_hue_delta(just_left, just_right);

        assert!(
            delta.abs() < (40_f32).to_radians(),
            "ten degrees across the wrap must not become three hundred fifty"
        );
    }

    #[test]
    fn an_achromatic_end_borrows_the_hue_instead_of_wheeling() {
        let grey = Oklch::from_rgb(rgb(128, 128, 128));
        let red = Oklch::from_rgb(rgb(220, 38, 38));

        let halfway = grey.mix(red, 0.5);

        assert!(
            (halfway.h - red.h).abs() < 1e-3,
            "a grey tinting up keeps the red's hue"
        );
    }

    #[test]
    fn two_greys_mix_to_a_grey_without_a_hue_opinion() {
        let dark = Oklch::from_rgb(rgb(40, 40, 40));
        let light = Oklch::from_rgb(rgb(200, 200, 200));

        let halfway = dark.mix(light, 0.5);

        assert!(halfway.c < ACHROMATIC_CHROMA);
    }

    #[test]
    fn an_out_of_gamut_colour_is_pulled_back_by_losing_chroma() {
        let impossible = Oklch {
            l: 0.5,
            c: 0.9,
            h: 0.0
        };

        let fitted = impossible.to_rgb();

        assert!(
            impossible.c > ACHROMATIC_CHROMA,
            "the test colour must actually be out of gamut"
        );
        assert!(
            Oklch::from_rgb(fitted).c < impossible.c,
            "the fitted colour gave up chroma to fit"
        );
    }

    #[test]
    fn hue_normalisation_wraps_both_ways() {
        use std::f32::consts::FRAC_PI_2;

        assert!(normalise_hue(TAU + 0.1) < PI);
        assert!(normalise_hue(-TAU - 0.1) > -PI);
        assert!((normalise_hue(FRAC_PI_2) - FRAC_PI_2).abs() < 1e-6);
    }
}
