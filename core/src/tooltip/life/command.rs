//! The vocabulary the machine speaks to the shell, and the pending dwell.

use std::time::Instant;

use hydebar_proto::config::ModuleName;
use iced::SurfaceId as Id;

use crate::tooltip::TooltipInfo;

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
pub(super) struct Dwell {
    pub(super) surface: Id,
    pub(super) module:  ModuleName,
    pub(super) info:    TooltipInfo,
    pub(super) due:     Instant
}
