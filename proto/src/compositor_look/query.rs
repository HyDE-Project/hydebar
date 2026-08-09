//! The one batch the look is read with, and the fields it fills.

use super::{
    CompositorLook,
    answers::{Answer, parse}
};
use crate::compositor_ipc;

/// Options the look is made of, asked for in one go.
///
/// Written as a table because the batch and the reading have to name the same
/// options: a field filled from an option nobody asked for stays unset
/// forever, and that is the kind of mistake a list read twice makes.
const OPTIONS: [&str; 10] = [
    "decoration:rounding",
    "general:gaps_out",
    "general:gaps_in",
    "animations:enabled",
    "decoration:blur:enabled",
    "general:border_size",
    "general:col.active_border",
    "decoration:shadow:enabled",
    "decoration:shadow:range",
    "decoration:shadow:color"
];

/// Asks the compositor for every part of the look, in one round trip.
pub(super) fn read() -> CompositorLook {
    let commands = OPTIONS
        .iter()
        .map(|option| format!("j/getoption {option}"))
        .collect::<Vec<_>>();

    compositor_ipc::batch(commands.iter().map(String::as_str))
        .map(|response| fill(&parse(&response)))
        .unwrap_or_default()
}

/// Fills the look from the answers a batch wrote back.
fn fill(answers: &[Answer]) -> CompositorLook {
    let of = |option: &str| answers.iter().find(|answer| answer.option == option);

    CompositorLook {
        rounding:     of("decoration:rounding").and_then(Answer::number),
        gaps_out:     of("general:gaps_out").and_then(Answer::gap),
        gaps_in:      of("general:gaps_in").and_then(Answer::gap),
        animations:   of("animations:enabled").and_then(|answer| answer.bool),
        blur:         of("decoration:blur:enabled").and_then(|answer| answer.bool),
        border_width: of("general:border_size").and_then(Answer::number),
        border_color: of("general:col.active_border").and_then(Answer::leading_color),
        shadow:       of("decoration:shadow:enabled").and_then(|answer| answer.bool),
        shadow_range: of("decoration:shadow:range").and_then(Answer::number),
        shadow_color: of("decoration:shadow:color").and_then(Answer::leading_color)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    const RESPONSE: &str = concat!(
        "{\"option\": \"decoration:rounding\", \"int\": 3 }\n\n\n",
        "{\"option\": \"general:gaps_out\", \"css\": \"8 8 8 8\" }\n\n\n",
        "{\"option\": \"general:gaps_in\", \"css\": \"3 3 3 3\" }\n\n\n",
        "{\"option\": \"animations:enabled\", \"bool\": true }\n\n\n",
        "{\"option\": \"decoration:blur:enabled\", \"bool\": true }\n\n\n",
        "{\"option\": \"general:border_size\", \"int\": 2 }\n\n\n",
        "{\"option\": \"general:col.active_border\", \"gradient\": \"f2e0c8a4 45deg\" }\n\n\n",
        "{\"option\": \"decoration:shadow:enabled\", \"bool\": false }\n\n\n",
        "{\"option\": \"decoration:shadow:range\", \"int\": 4 }\n\n\n",
        "{\"option\": \"decoration:shadow:color\", \"gradient\": \"ee1a1a1a 0deg\" }"
    );

    #[test]
    fn every_option_asked_for_lands_in_its_own_field() {
        let look = fill(&parse(RESPONSE));

        assert_eq!(look.rounding, Some(3.0));
        assert_eq!(look.gaps_out, Some(8.0));
        assert_eq!(look.gaps_in, Some(3.0));
        assert_eq!(look.animations, Some(true));
        assert_eq!(look.blur, Some(true));
        assert_eq!(look.border_width, Some(2.0));
        assert_eq!(look.shadow, Some(false));
        assert_eq!(look.shadow_range, Some(4.0));
        assert!(look.border_color.is_some());
        assert!(look.shadow_color.is_some());
    }

    #[test]
    fn the_batch_asks_for_exactly_what_the_reading_looks_up() {
        let look = fill(&parse(RESPONSE));
        let asked: Vec<&str> = OPTIONS.to_vec();

        assert_eq!(asked.len(), parse(RESPONSE).len());
        for answer in parse(RESPONSE) {
            assert!(
                asked.contains(&answer.option.as_str()),
                "{} is answered but never asked for",
                answer.option
            );
        }
        assert_ne!(look, CompositorLook::default());
    }

    #[test]
    fn an_option_the_compositor_never_answered_is_left_unset() {
        let look = fill(&parse("{\"option\": \"decoration:rounding\", \"int\": 3 }"));

        assert_eq!(look.rounding, Some(3.0));
        assert_eq!(look.gaps_in, None);
        assert_eq!(look.border_color, None);
    }

    #[test]
    fn a_compositor_that_answers_nothing_leaves_the_look_untouched() {
        assert_eq!(fill(&parse("")), CompositorLook::default());
    }
}
