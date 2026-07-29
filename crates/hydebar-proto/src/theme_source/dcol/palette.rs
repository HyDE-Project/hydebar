//! The wallbash palette and the `.dcol` grammar it is written in.

use super::super::color::Rgba;
use crate::shell_vars;

/// Number of dominant colours wallbash extracts from a wallpaper.
pub(super) const GROUPS: usize = 4;

/// Number of accent steps wallbash derives from each dominant colour.
pub(super) const STEPS: usize = 9;

/// Key the light/dark decision is recorded under.
const MODE_KEY: &str = "dcol_mode";

/// Which way round wallbash read the wallpaper.
///
/// Carried because it is what decides whether a palette has to be read
/// backwards for the theme in force, which is the one rule the bar cannot skip
/// without colouring itself differently from the rest of the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DcolMode {
    /// Light text on dark surfaces.
    Dark,
    /// Dark text on light surfaces.
    Light
}

impl DcolMode {
    /// Reads a mode from the word `dcol_mode` is assigned.
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None
        }
    }

    /// The mode a palette read backwards has.
    ///
    /// Wallbash calls this `dcol_invt` and substitutes it wherever a template
    /// asks for the mode, so an inverted palette has to report the opposite of
    /// what the file says or the two would disagree.
    #[must_use]
    pub fn inverted(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark
        }
    }

    /// The word HyDE writes this mode as.
    ///
    /// A theme states its preference as `prefer-dark`, so the comparison HyDE
    /// makes is a substring one against exactly this word.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light"
        }
    }
}

/// The colours wallbash extracted from a wallpaper.
///
/// This is the whole palette every HyDE consumer is coloured from, so the bar
/// reads it directly instead of waiting for a template to render a stylesheet
/// for it.
#[derive(Debug, Clone, PartialEq)]
pub struct DcolPalette {
    /// Whether the wallpaper read as dark or light.
    pub mode:    DcolMode,
    /// The four dominant colours, `dcol_pry1` to `dcol_pry4`.
    pub primary: [Rgba; GROUPS],
    /// The text colour picked for each dominant colour, `dcol_txt1` to
    /// `dcol_txt4`.
    pub text:    [Rgba; GROUPS],
    /// The nine-step accent ramp of each dominant colour, `dcol_<n>xa1` to
    /// `dcol_<n>xa9`, darkest first.
    pub accents: [[Rgba; STEPS]; GROUPS]
}

impl DcolPalette {
    /// Reads a palette out of the contents of a `.dcol` file.
    ///
    /// Returns [`None`] unless every colour is present: a half-read palette
    /// would paint some of the bar in the new theme and the rest in the old,
    /// which is worse than falling back to the previous source wholesale.
    ///
    /// The `_rgba` variants of each key are ignored on purpose. They carry a
    /// literal `\1` where the alpha belongs, because a shell `sed` fills it in
    /// later; the bar applies its own alpha in [`super::roles`] instead.
    #[must_use]
    pub fn parse(source: &str) -> Option<Self> {
        let mode = DcolMode::parse(&shell_vars::value_of(source, MODE_KEY)?)?;

        let mut primary = [Rgba::rgb(0, 0, 0); GROUPS];
        let mut text = [Rgba::rgb(0, 0, 0); GROUPS];
        let mut accents = [[Rgba::rgb(0, 0, 0); STEPS]; GROUPS];

        for group in 0..GROUPS {
            let number = group + 1;

            primary[group] = hex(source, &format!("dcol_pry{number}"))?;
            text[group] = hex(source, &format!("dcol_txt{number}"))?;

            for step in 0..STEPS {
                accents[group][step] = hex(source, &format!("dcol_{number}xa{}", step + 1))?;
            }
        }

        Some(Self {
            mode,
            primary,
            text,
            accents
        })
    }

    /// Returns the palette read backwards.
    ///
    /// Wallbash offers every template two renderings of the same colours and
    /// picks between them per theme; the inverted one swaps dominant colour `i`
    /// for `5 - i`, leaving each accent ramp in its own order, and reports the
    /// opposite mode. Reproduced exactly so a theme that asks to be inverted
    /// looks the same in the bar as in everything else on the desktop.
    #[must_use]
    pub fn inverted(&self) -> Self {
        let mut primary = self.primary;
        let mut text = self.text;
        let mut accents = self.accents;

        for group in 0..GROUPS {
            let mirrored = GROUPS - 1 - group;

            primary[group] = self.primary[mirrored];
            text[group] = self.text[mirrored];
            accents[group] = self.accents[mirrored];
        }

        Self {
            mode: self.mode.inverted(),
            primary,
            text,
            accents
        }
    }
}

/// Reads one colour, written as six hexadecimal digits without a `#`.
fn hex(source: &str, key: &str) -> Option<Rgba> {
    let value = shell_vars::value_of(source, key)?;

    super::super::color::parse_color(&format!("#{value}"))
}

#[cfg(test)]
mod tests {
    use super::{super::fixtures::WALL_DCOL, *};

    #[test]
    fn a_complete_palette_is_read_from_a_dcol_file() {
        let palette = DcolPalette::parse(WALL_DCOL).expect("palette");

        assert_eq!(palette.mode, DcolMode::Dark);
        assert_eq!(palette.primary[0], Rgba::rgb(0x48, 0x38, 0x28));
        assert_eq!(palette.primary[3], Rgba::rgb(0x04, 0x03, 0x02));
        assert_eq!(palette.text[0], Rgba::rgb(0xFF, 0xFF, 0xFF));
        assert_eq!(palette.accents[0][7], Rgba::rgb(0xF0, 0xCD, 0xAA));
        assert_eq!(palette.accents[3][8], Rgba::rgb(0x49, 0x38, 0x27));
    }

    #[test]
    fn a_palette_missing_a_colour_is_refused_whole() {
        let truncated: String = WALL_DCOL
            .lines()
            .filter(|line| !line.starts_with("dcol_4xa9"))
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(DcolPalette::parse(&truncated), None);
    }

    #[test]
    fn a_palette_without_a_mode_is_refused() {
        assert_eq!(DcolPalette::parse("dcol_pry1=\"483828\""), None);
        assert_eq!(DcolPalette::parse(""), None);
    }

    #[test]
    fn an_unreadable_colour_is_refused() {
        let broken = WALL_DCOL.replace("dcol_pry1=\"483828\"", "dcol_pry1=\"not-a-colour\"");

        assert_eq!(DcolPalette::parse(&broken), None);
    }

    #[test]
    fn inverting_mirrors_the_dominant_colours_and_flips_the_mode() {
        let palette = DcolPalette::parse(WALL_DCOL).expect("palette");
        let inverted = palette.inverted();

        assert_eq!(inverted.mode, DcolMode::Light);
        assert_eq!(inverted.primary[0], palette.primary[3]);
        assert_eq!(inverted.primary[3], palette.primary[0]);
        assert_eq!(inverted.text[1], palette.text[2]);
        assert_eq!(inverted.accents[0], palette.accents[3]);
    }

    #[test]
    fn inverting_twice_returns_the_palette_unchanged() {
        let palette = DcolPalette::parse(WALL_DCOL).expect("palette");

        assert_eq!(palette.inverted().inverted(), palette);
    }

    #[test]
    fn an_accent_ramp_keeps_its_own_order_when_mirrored() {
        let palette = DcolPalette::parse(WALL_DCOL).expect("palette");

        assert_eq!(palette.inverted().accents[0][0], palette.accents[3][0]);
        assert_eq!(palette.inverted().accents[0][8], palette.accents[3][8]);
    }

    #[test]
    fn the_inverted_mode_is_the_opposite_word() {
        assert_eq!(DcolMode::Dark.inverted(), DcolMode::Light);
        assert_eq!(DcolMode::Light.inverted(), DcolMode::Dark);
        assert_eq!(DcolMode::Dark.word(), "dark");
        assert_eq!(DcolMode::Light.word(), "light");
    }
}
