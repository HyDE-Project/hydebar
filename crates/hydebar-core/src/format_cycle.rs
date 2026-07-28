//! Cyclic cursor over the alternative formats a module can render.
//!
//! A module declares a primary format plus an ordered list of alternatives.
//! Pressing the module advances the cursor, wrapping back to the primary
//! format after the last alternative, mirroring waybar's `format-alt`.
//!
//! A module without alternatives owns a cursor that never leaves the primary
//! format, so its rendering is unaffected.

/// Index of the format a module currently renders.
///
/// Index `0` selects the primary format and index `n` selects the `n - 1`th
/// alternative. The cursor tolerates a shrinking list of alternatives, which
/// happens when the configuration is reloaded while an alternative is active.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FormatCycle {
    index: usize
}

impl FormatCycle {
    /// Creates a cursor resting on the primary format.
    pub const fn new() -> Self {
        Self {
            index: 0
        }
    }

    /// Index of the active format, primary format included.
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Advances to the next format, wrapping after the last alternative.
    ///
    /// A module without alternatives stays on its primary format.
    pub fn advance<T>(&mut self, alternatives: &[T]) {
        self.index = match alternatives.len() {
            0 => 0,
            len => (self.index + 1) % (len + 1)
        };
    }

    /// Resolves the format the cursor selects.
    ///
    /// A cursor pointing past the end of a shrunken list of alternatives wraps
    /// instead of falling back to the primary format alone.
    pub fn resolve<'a, T>(&self, primary: &'a T, alternatives: &'a [T]) -> &'a T {
        match self.index % (alternatives.len() + 1) {
            0 => primary,
            position => &alternatives[position - 1]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_cursor_rests_on_the_primary_format() {
        let cycle = FormatCycle::new();

        assert_eq!(cycle.index(), 0);
        assert_eq!(cycle.resolve(&"primary", &["alt"]), &"primary");
    }

    #[test]
    fn advancing_walks_the_alternatives_in_order() {
        let alternatives = ["first", "second"];
        let mut cycle = FormatCycle::new();

        cycle.advance(&alternatives);
        assert_eq!(cycle.resolve(&"primary", &alternatives), &"first");

        cycle.advance(&alternatives);
        assert_eq!(cycle.resolve(&"primary", &alternatives), &"second");
    }

    #[test]
    fn advancing_past_the_last_alternative_wraps_around() {
        let alternatives = ["first", "second"];
        let mut cycle = FormatCycle::new();

        for _ in 0..3 {
            cycle.advance(&alternatives);
        }

        assert_eq!(cycle.index(), 0);
        assert_eq!(cycle.resolve(&"primary", &alternatives), &"primary");
    }

    #[test]
    fn a_module_without_alternatives_never_changes_format() {
        let alternatives: [&str; 0] = [];
        let mut cycle = FormatCycle::new();

        for _ in 0..5 {
            cycle.advance(&alternatives);
            assert_eq!(cycle.index(), 0);
            assert_eq!(cycle.resolve(&"primary", &alternatives), &"primary");
        }
    }

    #[test]
    fn a_cursor_past_a_shrunken_list_wraps_instead_of_panicking() {
        let mut cycle = FormatCycle::new();
        cycle.advance(&["first", "second"]);
        cycle.advance(&["first", "second"]);

        assert_eq!(cycle.index(), 2);
        assert_eq!(cycle.resolve(&"primary", &["only"]), &"primary");
    }
}
