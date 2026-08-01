//! The life of a tooltip: dwell, warmth and the fade either way.
//!
//! One machine owns every rule about when a hint may be on screen, so the
//! shell holds a single value and executes the commands it is handed:
//!
//! - a pointer merely passing through raises nothing — the hint waits out a
//!   dwell first;
//! - once the user is reading hints, neighbouring modules answer at once for a
//!   short warmth window instead of charging the dwell again;
//! - a hint enters and leaves on the same fade every window rides, and the
//!   surface is only emptied once the fade-out has landed.

use std::time::{Duration, Instant};

use hydebar_proto::config::ModuleName;
use iced::SurfaceId as Id;

use super::TooltipInfo;
use crate::animation::{GENTLE, SNAPPY, Spring};

/// How long the pointer rests on a module before its tooltip shows.
const DWELL: Duration = Duration::from_secs(1);

/// How long after a tooltip hides the next one still shows at once.
const WARMTH: Duration = Duration::from_millis(300);

/// What the shell must do for the machine, in its own vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintCommand {
    /// Put this hint on the tooltip surface.
    Show {
        /// Bar surface the module sits on.
        surface: Id,
        /// Module the hint belongs to.
        module:  ModuleName,
        /// The hint itself.
        info:    TooltipInfo
    },
    /// Take the hint off the tooltip surface.
    Hide {
        /// Bar surface the module sits on.
        surface: Id,
        /// Only hide a hint this module owns; [`None`] hides whatever shows.
        owner:   Option<ModuleName>
    }
}

/// A hint waiting out the rest of the pointer's dwell.
#[derive(Debug, Clone)]
struct Dwell {
    surface: Id,
    module:  ModuleName,
    info:    TooltipInfo,
    due:     Instant
}

/// The one tooltip lifecycle of the bar.
///
/// # Examples
///
/// ```
/// use hydebar_core::tooltip::Hints;
///
/// let hints = Hints::default();
/// assert_eq!(hints.presence(), 0.0);
/// assert!(!hints.needs_frames());
/// ```
#[derive(Debug)]
pub struct Hints {
    dwell:      Option<Dwell>,
    shown:      bool,
    warm_until: Option<Instant>,
    presence:   Spring,
    closing:    Option<(Id, Option<ModuleName>)>
}

impl Default for Hints {
    fn default() -> Self {
        Self {
            dwell:      None,
            shown:      false,
            warm_until: None,
            presence:   Spring::new(0.0).with_response(SNAPPY),
            closing:    None
        }
    }
}

impl Hints {
    /// How far the tooltip currently is between absent and fully shown.
    #[must_use]
    pub const fn presence(&self) -> f32 {
        self.presence.value().clamp(0.0, 1.0)
    }

    /// Whether the machine needs the frame clock to keep ticking.
    #[must_use]
    pub fn needs_frames(&self) -> bool {
        self.dwell.is_some() || self.presence.is_animating()
    }

    /// Follows the pointer entering or leaving a module.
    ///
    /// `hint` carries the module's tooltip when it publishes one for this
    /// move. With `animated` off, every fade collapses to a snap.
    pub fn observe(
        &mut self,
        surface: Id,
        module: ModuleName,
        entered: bool,
        hint: Option<TooltipInfo>,
        now: Instant,
        animated: bool
    ) -> Option<HintCommand> {
        match hint {
            Some(_) if !entered => {
                // the anchors never publish a hint on a leave; a machine that
                // trusted that silently would show a hint for a module the
                // pointer just left if they ever did
                self.hide(surface, Some(module), now, animated)
            }
            Some(info) => {
                let warm = self.shown || self.warm_until.is_some_and(|until| now < until);

                if warm {
                    self.dwell = None;

                    return Some(self.show(surface, module, info, animated));
                }

                self.dwell = Some(Dwell {
                    surface,
                    module,
                    info,
                    due: now + DWELL
                });

                None
            }
            None => {
                if self
                    .dwell
                    .as_ref()
                    .is_some_and(|dwell| !entered && dwell.module == module)
                {
                    self.dwell = None;
                }

                let owner = if entered { None } else { Some(module) };

                self.hide(surface, owner, now, animated)
            }
        }
    }

    /// The show whose dwell `now` has served, if one was waiting.
    pub fn served(&mut self, now: Instant, animated: bool) -> Option<HintCommand> {
        if self.dwell.as_ref().is_none_or(|dwell| now < dwell.due) {
            return None;
        }

        let dwell = self.dwell.take()?;

        Some(self.show(dwell.surface, dwell.module, dwell.info, animated))
    }

    /// Advances the fade and reports it, with the hide a landed fade-out owes.
    pub fn advance(&mut self, elapsed: Duration) -> (bool, Option<HintCommand>) {
        let fading = self.presence.advance(elapsed);

        if !fading
            && self.presence.value() <= f32::EPSILON
            && let Some((surface, owner)) = self.closing.take()
        {
            return (
                false,
                Some(HintCommand::Hide {
                    surface,
                    owner
                })
            );
        }

        (fading, None)
    }

    /// Drops every hint at once, for the moment a menu takes the screen.
    ///
    /// The warmth goes too: opening a menu is a change of activity, and the
    /// first hint after it must wait out the dwell like any first hint.
    pub fn dismiss(&mut self) {
        self.dwell = None;
        self.shown = false;
        self.warm_until = None;
        self.closing = None;
        self.presence.snap_to(0.0);
    }

    /// Starts showing a hint, fading it in unless fades are off.
    ///
    /// The entrance is gentle on purpose — a hint is passive information, and
    /// arriving softly is what tells it apart from something demanding
    /// attention. The exit in [`Hints::hide`] is quicker: a hint on its way
    /// out is in the way.
    fn show(
        &mut self,
        surface: Id,
        module: ModuleName,
        info: TooltipInfo,
        animated: bool
    ) -> HintCommand {
        self.shown = true;
        self.closing = None;

        if animated {
            self.presence.set_response(GENTLE);
            self.presence.set_target(1.0);
        } else {
            self.presence.snap_to(1.0);
        }

        HintCommand::Show {
            surface,
            module,
            info
        }
    }

    /// Starts hiding, deferring the surface wipe until the fade lands.
    fn hide(
        &mut self,
        surface: Id,
        owner: Option<ModuleName>,
        now: Instant,
        animated: bool
    ) -> Option<HintCommand> {
        if self.shown {
            self.shown = false;
            self.warm_until = Some(now + WARMTH);
        }

        if animated && self.presence.value() > f32::EPSILON {
            self.closing = Some((surface, owner));
            self.presence.set_response(SNAPPY);
            self.presence.set_target(0.0);

            return None;
        }

        self.presence.snap_to(0.0);
        self.closing = None;

        Some(HintCommand::Hide {
            surface,
            owner
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::Point;

    use super::*;
    use crate::position_button::ButtonUIRef;

    fn info(text: &str) -> TooltipInfo {
        TooltipInfo {
            text:   text.to_owned(),
            anchor: ButtonUIRef {
                position: Point::new(10.0, 10.0),
                viewport: (1920.0, 40.0)
            }
        }
    }

    fn surface() -> Id {
        Id::unique()
    }

    fn drain(hints: &mut Hints) -> Option<HintCommand> {
        let mut hidden = None;
        let mut frames = 0;

        loop {
            let (fading, command) = hints.advance(Duration::from_millis(8));

            if let Some(command) = command {
                hidden = Some(command);
            }

            if !fading && !hints.presence.is_animating() && hints.closing.is_none() {
                break;
            }

            frames += 1;
            assert!(frames < 1000, "tooltip fade failed to settle");
        }

        hidden
    }

    #[test]
    fn a_pointer_passing_through_raises_nothing() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let on_enter = hints.observe(id, ModuleName::Clock, true, Some(info("hi")), now, true);

        assert_eq!(on_enter, None, "the hint waits out the dwell");
        assert!(hints.needs_frames());
        assert_eq!(
            hints.served(now + Duration::from_millis(300), true),
            None,
            "the dwell is not served early"
        );

        let _ = hints.observe(id, ModuleName::Clock, false, None, now + WARMTH, true);

        assert_eq!(hints.served(now + DWELL, true), None, "leaving cancels it");
    }

    #[test]
    fn a_dwell_served_shows_the_hint() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let _ = hints.observe(id, ModuleName::Clock, true, Some(info("hi")), now, true);
        let served = hints.served(now + DWELL, true);

        assert!(matches!(served, Some(HintCommand::Show { .. })));
        assert!(hints.presence.is_animating(), "the hint fades in");
    }

    #[test]
    fn a_neighbour_shows_at_once_while_the_hints_are_warm() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let _ = hints.observe(id, ModuleName::Clock, true, Some(info("a")), now, true);
        let _ = hints.served(now + DWELL, true);

        let left = now + DWELL + Duration::from_millis(50);
        let _ = hints.observe(id, ModuleName::Clock, false, None, left, true);

        let entered = left + Duration::from_millis(100);
        let neighbour = hints.observe(
            id,
            ModuleName::Battery,
            true,
            Some(info("b")),
            entered,
            true
        );

        assert!(
            matches!(neighbour, Some(HintCommand::Show { .. })),
            "warmth spares the neighbour the dwell"
        );
    }

    #[test]
    fn warmth_expires_and_the_dwell_returns() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let _ = hints.observe(id, ModuleName::Clock, true, Some(info("a")), now, true);
        let _ = hints.served(now + DWELL, true);
        let _ = hints.observe(id, ModuleName::Clock, false, None, now + DWELL, true);

        let cold = now + DWELL + WARMTH + Duration::from_millis(1);
        let later = hints.observe(id, ModuleName::Battery, true, Some(info("b")), cold, true);

        assert_eq!(later, None, "cold hints wait out the dwell again");
    }

    #[test]
    fn the_surface_is_only_wiped_once_the_fade_out_lands() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let _ = hints.observe(id, ModuleName::Clock, true, Some(info("a")), now, true);
        let _ = hints.served(now + DWELL, true);
        drain(&mut hints);

        let hide = hints.observe(id, ModuleName::Clock, false, None, now + DWELL, true);

        assert_eq!(hide, None, "the content stays while the fade travels");
        assert!(hints.presence.is_animating());

        let hidden = drain(&mut hints);

        assert!(matches!(hidden, Some(HintCommand::Hide { .. })));
        assert_eq!(hints.presence(), 0.0);
    }

    #[test]
    fn disabled_fades_show_and_hide_at_once() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let _ = hints.observe(id, ModuleName::Clock, true, Some(info("a")), now, false);
        let shown = hints.served(now + DWELL, false);
        assert!(matches!(shown, Some(HintCommand::Show { .. })));
        assert_eq!(hints.presence(), 1.0);

        let hidden = hints.observe(id, ModuleName::Clock, false, None, now + DWELL, false);

        assert!(matches!(hidden, Some(HintCommand::Hide { .. })));
        assert_eq!(hints.presence(), 0.0);
    }

    #[test]
    fn an_opening_menu_drops_everything_at_once() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let _ = hints.observe(id, ModuleName::Clock, true, Some(info("a")), now, true);
        let _ = hints.served(now + DWELL, true);

        hints.dismiss();

        assert_eq!(hints.presence(), 0.0);
        assert!(!hints.needs_frames());
    }

    #[test]
    fn a_menu_takes_the_warmth_with_it() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let _ = hints.observe(id, ModuleName::Clock, true, Some(info("a")), now, true);
        let _ = hints.served(now + DWELL, true);
        let _ = hints.observe(id, ModuleName::Clock, false, None, now + DWELL, true);

        hints.dismiss();

        let after = hints.observe(
            id,
            ModuleName::Battery,
            true,
            Some(info("b")),
            now + DWELL + Duration::from_millis(50),
            true
        );

        assert_eq!(after, None, "the first hint after a menu waits again");
    }

    #[test]
    fn a_leave_carrying_a_hint_hides_instead_of_showing() {
        let mut hints = Hints::default();
        let now = Instant::now();
        let id = surface();

        let odd = hints.observe(id, ModuleName::Clock, false, Some(info("a")), now, false);

        assert!(matches!(odd, Some(HintCommand::Hide { .. })));
    }
}
