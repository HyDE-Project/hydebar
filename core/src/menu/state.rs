//! Lifetime and fade of a single menu surface.

use iced::{OutputId, SurfaceId as Id};

use super::kind::MenuType;
use crate::{animation::Spring, position_button::ButtonUIRef};

mod fade;
mod lifecycle;

/// Opacity distance below which the fade counts as arrived.
///
/// The default spring precision is tuned for values whose last thousandths
/// still matter. An opacity's do not — yet the spring walked them anyway,
/// and for that whole asymptotic crawl a large dark panel stayed readable
/// over a bright wallpaper as a ghost of the closed window. Settling at the
/// first invisible alpha drops the surface the moment the eye is done with
/// it, without touching the visible part of either fade.
const SETTLE_PRECISION: f32 = 0.02;

/// One menu surface, and what it currently holds.
#[derive(Clone, Debug)]
pub struct Menu {
    /// The surface the compositor knows it by.
    pub id:        Id,
    /// What is open on it, and the button it was opened from.
    pub menu_info: Option<(MenuType, ButtonUIRef)>,
    /// Output the menu surface stands on, for building its replacement.
    output:        Option<OutputId>,
    opacity:       Spring,
    /// Opacity the menu opens up to, kept so the travelled share can be told
    /// while fading in.
    full_opacity:  f32,
    /// Whether the press in flight has to take this menu down once it
    /// completes.
    dismiss_armed: bool
}

impl Menu {
    /// A menu surface holding nothing yet.
    #[must_use]
    pub const fn new(id: Id, output: Option<OutputId>) -> Self {
        Self {
            id,
            menu_info: None,
            output,
            opacity: Spring::new(0.0).with_precision(SETTLE_PRECISION),
            full_opacity: 0.0,
            dismiss_armed: false
        }
    }

    /// Returns whether the menu is open as far as the user is concerned.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.menu_info.is_some()
    }
}
