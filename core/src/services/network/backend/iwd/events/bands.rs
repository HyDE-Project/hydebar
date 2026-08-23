//! The bands the signal agent reports in, as the bar's percentages.

/// Maps the bucket the signal agent reports onto a percentage.
///
/// The agent is registered with three thresholds, so iwd answers with the
/// index of the band the signal fell into — not a percentage. Rendering the
/// index directly showed the weakest icon whatever the actual signal.
pub(super) const fn strength_of_level(level: i16) -> u8 {
    match level {
        0 => 100,
        1 => 75,
        2 => 50,
        _ => 25
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::strength_of_level;

    #[test]
    fn the_strongest_band_maps_to_a_full_signal() {
        assert_eq!(strength_of_level(0), 100);
    }

    #[test]
    fn the_second_band_maps_to_three_quarters() {
        assert_eq!(strength_of_level(1), 75);
    }

    #[test]
    fn the_third_band_maps_to_half() {
        assert_eq!(strength_of_level(2), 50);
    }

    #[test]
    fn any_other_band_maps_to_a_quarter() {
        assert_eq!(strength_of_level(3), 25);
        assert_eq!(strength_of_level(-1), 25);
        assert_eq!(strength_of_level(i16::MAX), 25);
        assert_eq!(strength_of_level(i16::MIN), 25);
    }
}
