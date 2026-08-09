//! The answers the compositor gives when asked about one of its options.
//!
//! Asked in JSON, every answer names the option it belongs to and carries the
//! value under a key that says what kind it is — a whole number, a fraction, a
//! flag, four stylesheet-style sides, or a gradient. Reading them back by name
//! rather than by position is what lets one batch stand in for ten questions:
//! an option the compositor has never heard of simply leaves its field unset
//! instead of shifting every answer after it.

use serde::Deserialize;

/// One answer, as the compositor writes it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct Answer {
    /// Option the answer belongs to.
    pub option:   String,
    /// Value of an option counted in whole units.
    #[serde(default)]
    pub int:      Option<f32>,
    /// Value of an option measured in fractions.
    #[serde(default)]
    pub float:    Option<f32>,
    /// Value of an option that is on or off.
    #[serde(default)]
    pub bool:     Option<bool>,
    /// Value of an option written the way a stylesheet writes four sides.
    #[serde(default)]
    pub css:      Option<String>,
    /// Value of an option painted as a gradient.
    #[serde(default)]
    pub gradient: Option<String>
}

impl Answer {
    /// The answer read as a number, whichever way it was counted.
    pub(super) fn number(&self) -> Option<f32> {
        self.int.or(self.float)
    }

    /// The first of the four sides an answer states.
    ///
    /// The compositor states gaps the way a stylesheet does, four sides at
    /// once; the bar only ever needs one number, and the sides are equal in
    /// every configuration that looks deliberate.
    pub(super) fn gap(&self) -> Option<f32> {
        self.css.as_ref()?.split_whitespace().next()?.parse().ok()
    }

    /// The leading colour of the gradient an answer states, as RGBA in unit
    /// range.
    ///
    /// The compositor paints a border as a gradient; one border colour is all
    /// an island needs, and the leading stop is the one the eye reads at the
    /// corner the gradient starts from. The spelling is AARRGGBB.
    pub(super) fn leading_color(&self) -> Option<[f32; 4]> {
        let token = self.gradient.as_ref()?.split_whitespace().next()?;

        if token.len() != 8 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        let channel = |from: usize| {
            u8::from_str_radix(&token[from..from + 2], 16)
                .map(|byte| f32::from(byte) / 255.0)
                .ok()
        };

        Some([channel(2)?, channel(4)?, channel(6)?, channel(0)?])
    }
}

/// Reads every answer out of what one batch wrote back.
///
/// The compositor separates answers with blank lines and writes each JSON
/// answer on a line of its own, so a line that does not parse — the plain
/// `no such option` it answers with when asked for something it does not know
/// — is skipped rather than ending the reading.
pub(super) fn parse(response: &str) -> Vec<Answer> {
    response
        .lines()
        .filter_map(|line| serde_json::from_str::<Answer>(line.trim()).ok())
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    const RESPONSE: &str = concat!(
        "{\"option\": \"decoration:rounding\", \"int\": 3, \"set\": true }\n",
        "\n\n",
        "no such option\n",
        "\n\n",
        "{\"option\": \"general:gaps_in\", \"css\": \"3 3 3 3\", \"set\": true }\n",
        "\n\n",
        "{\"option\": \"animations:enabled\", \"bool\": true, \"set\": false }"
    );

    #[test]
    fn every_answer_the_batch_wrote_back_is_read() {
        let answers = parse(RESPONSE);

        assert_eq!(answers.len(), 3);
        assert_eq!(answers[0].option, "decoration:rounding");
        assert_eq!(answers[2].option, "animations:enabled");
    }

    #[test]
    fn an_unknown_option_costs_the_answers_after_it_nothing() {
        let answers = parse(RESPONSE);

        assert_eq!(answers[1].gap(), Some(3.0));
        assert_eq!(answers[2].bool, Some(true));
    }

    #[test]
    fn a_number_is_read_whichever_way_it_was_counted() {
        assert_eq!(parse(RESPONSE)[0].number(), Some(3.0));
        assert_eq!(
            parse("{\"option\": \"o\", \"float\": 0.9 }")[0].number(),
            Some(0.9)
        );
    }

    #[test]
    fn a_gradient_gives_up_its_leading_stop() {
        let answers = parse("{\"option\": \"o\", \"gradient\": \"f2e0c8a4 f2524129 45deg\" }");

        let color = answers[0].leading_color().expect("a leading colour");
        assert_eq!(color[3], 242.0 / 255.0);
        assert_eq!(color[0], 224.0 / 255.0);
    }

    #[test]
    fn a_gradient_that_is_not_one_is_refused() {
        assert_eq!(
            parse("{\"option\": \"o\", \"gradient\": \"zzzzzzzz 0deg\" }")[0].leading_color(),
            None
        );
        assert_eq!(
            parse("{\"option\": \"o\", \"gradient\": \"f2e0c8 0deg\" }")[0].leading_color(),
            None
        );
    }

    #[test]
    fn an_answer_of_another_kind_yields_nothing() {
        let answers = parse("{\"option\": \"o\", \"bool\": true }");

        assert_eq!(answers[0].number(), None);
        assert_eq!(answers[0].gap(), None);
        assert_eq!(answers[0].leading_color(), None);
    }
}
