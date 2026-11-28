//! The screens the bar stands on: what each one measures, what it has been
//! told, and the row of the strip it earned from the compositor.
//!
//! Every number here arrives from outside — a monitor report, a scale call,
//! a compositor answer — and the bar only repeats what it was told. Absent
//! means the answer has not come yet, which the surfaces read as a default,
//! never as a guess.

use std::collections::HashMap;

use hydebar_core::outputs::AutoMetrics;
use hydebar_proto::config::AppearanceStyle;

/// What the screens have reported, and what the bar stated back to them.
pub struct Screen {
    /// Sizes the screen calls for, once an output has reported itself.
    pub(crate) auto_metrics:         Option<AutoMetrics>,
    /// Logical height of the screen the bar stands on, once reported.
    pub(crate) height:               Option<f32>,
    /// Logical width of the screen the bar stands on, once reported.
    ///
    /// What the islands are sent beyond when they fly back onto the strip:
    /// off screen has to be off *this* screen, not a guessed distance.
    pub(crate) width:                Option<f32>,
    /// Logical sizes each measured monitor calls for, by its name.
    ///
    /// Every output rings itself in its own numbers: a 4K panel and a 1080p
    /// one report different sizes, and whichever report arrived last is what
    /// the desk and the islands travel with, but the frame of a screen is a
    /// shape of that screen and no other.
    pub(crate) sizes:                HashMap<String, (f32, f32)>,
    /// The origin the strip stands at, screen by screen, as last measured.
    ///
    /// The canvas covers the screen from its very top and the strip does not
    /// have to: a layer surface is never told its own place, so the position
    /// the modules leave is asked of the compositor rather than assumed.
    /// Absent until the first answer, which reads as a strip at the edge.
    pub(crate) strip_positions:      HashMap<String, (f32, f32)>,
    /// Factor the screen calls for, folded into every configuration loaded.
    pub(crate) magnification:        f32,
    /// The layer metrics last stated to the compositor.
    ///
    /// Style, scale bits and height bits: while they stand still, a reload
    /// does not re-state the size and exclusive zone of every bar surface.
    pub(crate) stated_layer_metrics: Option<(AppearanceStyle, u64, Option<u32>)>
}
