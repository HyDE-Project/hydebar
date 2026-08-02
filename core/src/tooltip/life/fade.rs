//! The fade either way, and the wipe a landed fade-out owes.

use std::time::{Duration, Instant};

use hydebar_proto::config::ModuleName;
use iced::SurfaceId as Id;

use super::{HintCommand, Hints, WARMTH};
use crate::{
    animation::{GENTLE, SNAPPY},
    tooltip::TooltipInfo
};

impl Hints {
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
    pub(super) fn show(
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
    pub(super) fn hide(
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
