//! Tooltips rendered beside the bar instead of inside it.
//!
//! The bar is a layer-surface exactly as tall as the bar itself, so anything an
//! in-surface overlay draws is clipped to that strip and ends up painted over
//! the module it belongs to. Waybar places its tooltips outside the bar, and
//! the only way to do the same on a layer-shell bar is to draw them on another
//! surface.
//!
//! Every output therefore owns a surface of its own for them, the way it owns
//! one for its menus. That surface asks for no exclusive zone, so the
//! compositor lays it out beside the exclusive zone of the bar: the edge it
//! starts at, its top under a top bar and its bottom over a bottom one,
//! already sits clear of the bar. The bar publishes the hover, the surface
//! draws the hint anchored to the module the pointer rests on.
//!
//! The widget publishing the hover lives in [`anchor`], the request it turns
//! into in [`info`] and the content of the tooltip surface in [`wrapper`].

mod anchor;
mod info;
mod life;
mod wrapper;

pub use anchor::{TooltipAnchor, tooltip_anchor};
pub use info::TooltipInfo;
pub use life::{HintCommand, Hints};
pub use wrapper::{TOOLTIP_GAP_EM, tooltip_wrapper};
