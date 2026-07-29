//! Where the user is looking, and the two clocks that follow it.
//!
//! There is one pointer, so there is one attention: either nothing is attended
//! or exactly one module is. Resting the pointer on a module and opening its
//! menu are the same state, because they are the same act of looking at it, and
//! the bar keeps a single value for both.
//!
//! Two clocks serve the whole bar rather than one clock per module. The slow
//! one refreshes what the modules keep on the bar; the fast one runs only while
//! a module is attended and refreshes that module alone. Taking the attention
//! away stops the fast clock outright.
//!
//! A module declares the two cadences it is willing to be sampled at through
//! [`PollSchedule`] and owns no timer of its own, so the cost of an idle bar is
//! two wakeups rather than one per readout.

use std::time::{Duration, Instant};

use hydebar_proto::config::ModuleName;

/// The two cadences a module declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollSchedule {
    /// Cadence while nothing is attended.
    ///
    /// [`None`] marks a readout the bar never draws on its own — the list of
    /// nearby networks only the menu shows, for one. Such a module costs
    /// nothing at all while nobody looks at it.
    pub at_rest:  Option<Duration>,
    /// Cadence while the user attends the module.
    pub attended: Duration
}

impl PollSchedule {
    /// Schedule of a readout the bar keeps current, and refreshes faster while
    /// it is attended.
    #[must_use]
    pub const fn new(at_rest: Duration, attended: Duration) -> Self {
        Self {
            at_rest: Some(at_rest),
            attended
        }
    }

    /// Schedule of a readout that only exists while the module is attended.
    #[must_use]
    pub const fn only_when_attended(attended: Duration) -> Self {
        Self {
            at_rest: None,
            attended
        }
    }
}

/// A pollable module the layout draws.
#[derive(Debug)]
struct Placed {
    /// Layout entry the schedule belongs to.
    name:     ModuleName,
    /// Cadences the module declared.
    schedule: PollSchedule,
    /// When the module was last sampled, at either cadence.
    last:     Option<Instant>
}

/// The attention of the bar and the roster the two clocks serve.
///
/// # Examples
///
/// ```
/// use std::time::{Duration, Instant};
///
/// use hydebar_core::attention::{Attention, PollSchedule};
/// use hydebar_proto::config::ModuleName;
///
/// let mut attention = Attention::default();
/// attention.place([(
///     ModuleName::Network,
///     PollSchedule::only_when_attended(Duration::from_secs(2))
/// )]);
///
/// assert_eq!(attention.attended_period(), None);
///
/// attention.look_at(Some(ModuleName::Network));
/// assert_eq!(attention.attended_period(), Some(Duration::from_secs(2)));
/// ```
#[derive(Debug, Default)]
pub struct Attention {
    /// The one module the pointer rests on, or whose menu is open.
    focus:  Option<ModuleName>,
    /// Pollable modules the layout draws, with the instant each was sampled.
    placed: Vec<Placed>
}

impl Attention {
    /// Replaces the roster with the pollable modules the layout now draws.
    ///
    /// A module that survives the reload keeps the instant it was last sampled
    /// at, so reloading a configuration does not restart every cadence from
    /// zero. A module the layout dropped leaves the roster and stops being
    /// polled, which is the whole point of gating on placement: nothing that
    /// nobody can see is worth a wakeup.
    pub fn place<I>(&mut self, modules: I)
    where
        I: IntoIterator<Item = (ModuleName, PollSchedule)>
    {
        let previous = std::mem::take(&mut self.placed);

        self.placed = modules
            .into_iter()
            .map(|(name, schedule)| {
                let last = previous
                    .iter()
                    .find(|placed| placed.name == name)
                    .and_then(|placed| placed.last);

                Placed {
                    name,
                    schedule,
                    last
                }
            })
            .collect();
    }

    /// The module the user is looking at.
    #[must_use]
    pub fn focus(&self) -> Option<&ModuleName> {
        self.focus.as_ref()
    }

    /// Reports whether `name` is the module the user is looking at.
    #[must_use]
    pub fn is_on(&self, name: &ModuleName) -> bool {
        self.focus.as_ref() == Some(name)
    }

    /// Moves the attention onto `module`, or releases it when given [`None`].
    ///
    /// Returns whether the attention moved, so a caller can restate the clocks
    /// only when there is something to restate.
    pub fn look_at(&mut self, module: Option<ModuleName>) -> bool {
        if self.focus == module {
            return false;
        }

        self.focus = module;

        true
    }

    /// Follows the pointer entering `module`, or leaving it.
    ///
    /// Leaving releases the attention only while no menu holds it. Reaching a
    /// menu means taking the pointer off the module that opened it, and the
    /// menu is the very thing the user opened to look at, so the two are one
    /// state rather than two that cancel each other out.
    ///
    /// Returns whether the attention moved.
    pub fn follow_pointer(
        &mut self,
        module: ModuleName,
        entered: bool,
        menu_is_open: bool
    ) -> bool {
        if entered {
            return self.look_at(Some(module));
        }

        if menu_is_open || !self.is_on(&module) {
            return false;
        }

        self.look_at(None)
    }

    /// Period of the clock serving the modules at rest.
    ///
    /// The shortest cadence any placed module asked for: one clock cannot beat
    /// slower than the most impatient readout on the bar, and a module wanting
    /// less is served every few ticks instead of every one. [`None`] stops the
    /// clock, which is what a layout without a single pollable module deserves.
    #[must_use]
    pub fn rest_period(&self) -> Option<Duration> {
        self.placed
            .iter()
            .filter_map(|placed| placed.schedule.at_rest)
            .min()
    }

    /// Period of the clock serving the attended module.
    ///
    /// [`None`] whenever nothing is attended, or whenever what is attended is
    /// not a module the bar can poll, so the fast clock never ticks for nobody.
    #[must_use]
    pub fn attended_period(&self) -> Option<Duration> {
        let focus = self.focus.as_ref()?;

        self.placed
            .iter()
            .find(|placed| &placed.name == focus)
            .map(|placed| placed.schedule.attended)
    }

    /// Modules the slow clock owes a sample at `now`.
    ///
    /// The attended module is left out: the fast clock already serves it, and
    /// sampling it twice on the same cadence would only double the cost of
    /// looking at something.
    pub fn due_at_rest(&mut self, now: Instant) -> Vec<ModuleName> {
        let focus = self.focus.clone();
        let mut due = Vec::new();

        for placed in &mut self.placed {
            if Some(&placed.name) == focus.as_ref() {
                continue;
            }

            let Some(period) = placed.schedule.at_rest else {
                continue;
            };

            if placed
                .last
                .is_some_and(|last| now.saturating_duration_since(last) < period)
            {
                continue;
            }

            placed.last = Some(now);
            due.push(placed.name.clone());
        }

        due
    }

    /// The attended module when the fast clock owes it a sample at `now`.
    pub fn due_attended(&mut self, now: Instant) -> Option<ModuleName> {
        let focus = self.focus.clone()?;
        let placed = self.placed.iter_mut().find(|placed| placed.name == focus)?;

        if placed
            .last
            .is_some_and(|last| now.saturating_duration_since(last) < placed.schedule.attended)
        {
            return None;
        }

        placed.last = Some(now);

        Some(focus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REST: Duration = Duration::from_secs(10);
    const FAST: Duration = Duration::from_secs(1);

    fn attention() -> Attention {
        let mut attention = Attention::default();
        attention.place([
            (ModuleName::SystemInfo, PollSchedule::new(REST, FAST)),
            (
                ModuleName::Network,
                PollSchedule::only_when_attended(Duration::from_secs(2))
            )
        ]);

        attention
    }

    #[test]
    fn attention_moves_from_one_module_to_another() {
        let mut attention = attention();

        assert!(attention.look_at(Some(ModuleName::SystemInfo)));
        assert!(attention.is_on(&ModuleName::SystemInfo));

        assert!(attention.look_at(Some(ModuleName::Network)));
        assert!(attention.is_on(&ModuleName::Network));
        assert!(
            !attention.is_on(&ModuleName::SystemInfo),
            "one pointer means one attended module at a time"
        );
    }

    #[test]
    fn looking_at_the_same_module_twice_changes_nothing() {
        let mut attention = attention();

        assert!(attention.look_at(Some(ModuleName::SystemInfo)));
        assert!(!attention.look_at(Some(ModuleName::SystemInfo)));
    }

    #[test]
    fn attention_is_released() {
        let mut attention = attention();
        attention.look_at(Some(ModuleName::SystemInfo));

        assert!(attention.look_at(None));
        assert_eq!(attention.focus(), None);
    }

    #[test]
    fn the_pointer_entering_a_module_attends_it() {
        let mut attention = attention();

        attention.follow_pointer(ModuleName::Network, true, false);

        assert!(attention.is_on(&ModuleName::Network));
    }

    #[test]
    fn the_pointer_leaving_releases_the_attention() {
        let mut attention = attention();
        attention.follow_pointer(ModuleName::Network, true, false);

        attention.follow_pointer(ModuleName::Network, false, false);

        assert_eq!(attention.focus(), None);
    }

    #[test]
    fn an_open_menu_holds_the_attention_after_the_pointer_leaves() {
        let mut attention = attention();
        attention.follow_pointer(ModuleName::Network, true, false);

        attention.follow_pointer(ModuleName::Network, false, true);

        assert!(
            attention.is_on(&ModuleName::Network),
            "reaching a menu means leaving the module that opened it"
        );
    }

    #[test]
    fn leaving_a_module_that_is_not_attended_changes_nothing() {
        let mut attention = attention();
        attention.look_at(Some(ModuleName::Network));

        assert!(!attention.follow_pointer(ModuleName::SystemInfo, false, false));
        assert!(attention.is_on(&ModuleName::Network));
    }

    #[test]
    fn the_fast_clock_does_not_tick_without_attention() {
        let mut attention = attention();
        let now = Instant::now();

        assert_eq!(attention.attended_period(), None);
        assert_eq!(attention.due_attended(now), None);
        assert_eq!(attention.due_attended(now + REST * 100), None);
    }

    #[test]
    fn releasing_the_attention_stops_the_fast_clock() {
        let mut attention = attention();
        attention.look_at(Some(ModuleName::SystemInfo));

        assert_eq!(attention.attended_period(), Some(FAST));

        attention.look_at(None);

        assert_eq!(attention.attended_period(), None);
    }

    #[test]
    fn the_fast_clock_stays_still_for_a_module_the_bar_cannot_poll() {
        let mut attention = attention();
        attention.look_at(Some(ModuleName::Clock));

        assert_eq!(attention.attended_period(), None);
        assert_eq!(attention.due_attended(Instant::now()), None);
    }

    #[test]
    fn the_two_cadences_do_not_get_mixed_up() {
        let mut attention = attention();
        let start = Instant::now();

        assert_eq!(attention.rest_period(), Some(REST));

        attention.look_at(Some(ModuleName::SystemInfo));
        assert_eq!(attention.attended_period(), Some(FAST));

        assert_eq!(attention.due_attended(start), Some(ModuleName::SystemInfo));
        assert_eq!(
            attention.due_attended(start + FAST / 2),
            None,
            "the attended module keeps its own cadence"
        );
        assert_eq!(
            attention.due_attended(start + FAST),
            Some(ModuleName::SystemInfo)
        );
    }

    #[test]
    fn a_module_polled_only_under_attention_costs_nothing_at_rest() {
        let mut attention = attention();
        let start = Instant::now();

        assert_eq!(attention.due_at_rest(start), vec![ModuleName::SystemInfo]);
        assert_eq!(
            attention.due_at_rest(start + REST * 10),
            vec![ModuleName::SystemInfo],
            "a schedule without a resting cadence is never due on the slow clock"
        );
    }

    #[test]
    fn the_slow_clock_leaves_the_attended_module_to_the_fast_one() {
        let mut attention = attention();
        let start = Instant::now();
        attention.look_at(Some(ModuleName::SystemInfo));

        assert!(attention.due_at_rest(start).is_empty());
    }

    #[test]
    fn the_slow_clock_keeps_the_resting_cadence() {
        let mut attention = attention();
        let start = Instant::now();

        assert_eq!(attention.due_at_rest(start), vec![ModuleName::SystemInfo]);
        assert!(attention.due_at_rest(start + REST / 2).is_empty());
        assert_eq!(
            attention.due_at_rest(start + REST),
            vec![ModuleName::SystemInfo]
        );
    }

    #[test]
    fn a_module_the_layout_dropped_is_never_polled() {
        let mut attention = attention();
        attention.place([(
            ModuleName::Network,
            PollSchedule::only_when_attended(Duration::from_secs(2))
        )]);

        assert!(attention.due_at_rest(Instant::now()).is_empty());
        assert_eq!(attention.rest_period(), None);
    }

    #[test]
    fn a_reload_does_not_restart_the_cadence_of_a_module_that_stayed() {
        let mut attention = attention();
        let start = Instant::now();

        assert_eq!(attention.due_at_rest(start), vec![ModuleName::SystemInfo]);

        attention.place([(ModuleName::SystemInfo, PollSchedule::new(REST, FAST))]);

        assert!(
            attention.due_at_rest(start + REST / 2).is_empty(),
            "a module that survived the reload keeps the instant it was sampled at"
        );
    }
}
