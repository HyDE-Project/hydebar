//! How the machine follows the pointer: dwell, warmth and the odd leave.

use std::time::Instant;

use hydebar_proto::config::ModuleName;
use iced::SurfaceId as Id;

use super::{DWELL, Dwell, HintCommand, Hints};
use crate::tooltip::TooltipInfo;

impl Hints {
    /// Follows the pointer entering or leaving a module.
    ///
    /// `hint` carries the module's tooltip when it publishes one for this
    /// move. With `animated` off, every fade collapses to a snap.
    ///
    /// A hint arriving on a *leave* hides instead of showing: the anchors
    /// never publish one on a leave, and a machine that trusted that
    /// silently would show a hint for a module the pointer just left if
    /// they ever did.
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
            Some(_) if !entered => self.hide(surface, Some(module), now, animated),
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
}
