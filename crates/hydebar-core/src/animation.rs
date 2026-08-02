//! Frame-clock driven animation primitives.
//!
//! Animations are expressed as critically damped springs rather than fixed
//! duration tweens. A spring carries its own velocity, so retargeting mid
//! flight continues smoothly instead of snapping to a new interpolation curve.
//!
//! The animator never renders and never reads application state: callers feed
//! it a target, advance it once per frame with the elapsed time, and read the
//! current value while building the view.
//!
//! The spring and the motion tokens live in [`spring`], the stagger that
//! turns one travel into a front in [`wave`], and the per-item hover fades in
//! [`hover`].

mod hover;
mod spring;
mod wave;

pub use hover::HoverFades;
pub use spring::{GENTLE, SNAPPY, STANDARD, SWEEP, Spring};
pub use wave::sweep;
