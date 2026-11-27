//! The clocks the bar moves on: the theme's own crossing, the islands'
//! entrance and rearrangement, the desk's unfoldings, and the seat book the
//! flip animations read.
//!
//! None of this is about what the bar shows; all of it is about how it gets
//! there. Each clock runs on its own schedule, which is why they are kept
//! beside one another rather than folded into the surfaces they drive.

use std::{cell::RefCell, collections::HashMap, time::Instant};

use hydebar_core::{
    animation::{Spring, Unfold},
    components::flip::FlipMemo,
    style::{AppearanceTransition, SweepStyle}
};

/// Every clock the bar is currently riding.
pub struct Motion {
    /// The frame instant the last frame was drawn at, for frame deltas.
    pub(crate) last_frame: Option<Instant>,
    /// Theme built from the appearance in force, rebuilt only when it moves.
    pub(crate) appearance_transition: AppearanceTransition,
    /// Signature the current or incoming theme crosses the bar with.
    pub sweep: SweepStyle,
    /// Birth of the bar: the islands ride in on the theme's own wave once.
    pub entrance: Spring,
    /// Travel of the blocks gliding to a rearranged layout's places.
    pub relayout: Spring,
    /// The book of seats every module records its place in, per frame.
    pub flip: RefCell<FlipMemo>,
    /// Unfolding of the desk, one clock per screen it may unfold on.
    ///
    /// A clock apiece because the screens answer for themselves: one monitor
    /// may be folding back under a window that just mapped while the other is
    /// still unfolding over a workspace that was cleared.
    ///
    /// Even, not sprung: the blocks leave one after another and every one of
    /// them deserves the same slice of the travel. A spring driving the whole
    /// sequence spends its tail on the last blocks, where it barely moves, and
    /// they read as stalling. The easing belongs to each block's own slice.
    pub desk_clocks: HashMap<Option<String>, Unfold>,
    /// How far the strip's own background has come back from a folded canvas.
    ///
    /// A clock of its own rather than a reading off the islands' spring: the
    /// spring is all but home in its first tenth of a second, and a
    /// background painted on that schedule is back before the eye has it. The
    /// islands ride the spring and the background runs ahead of them on this,
    /// which is the order the two are wanted in.
    pub desk_returning: Unfold
}
