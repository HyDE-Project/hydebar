//! Where the bar's attention stands: what the keyboard holds, what the
//! pointer warmed, what is unfolded on screen, and the greeting the bar
//! shows while it comes up.
//!
//! This is the part of the state a person drives directly — keys, pointer,
//! the menus they leave open — as opposed to what the machine reports.

use std::time::{Duration, Instant};

use hydebar_core::{
    animation::{HoverFades, Spring},
    attention::Attention,
    config::ModuleName,
    notifications_popup,
    tooltip::Hints
};
use iced::SurfaceId as Id;

/// How long the greeting stays before it fades on its own.
pub(in crate::app) const GREETING_LIFETIME: Duration = Duration::from_secs(3);

/// What the bar is paying attention to, and what is unfolded around it.
pub struct Focus {
    /// Whether the bar currently holds the keyboard.
    pub navigation_mode:           bool,
    /// Which entry the keyboard selection rests on, while it holds it.
    pub focused_module_index:      Option<usize>,
    /// The one module the user is looking at, and the clocks that follow it.
    pub attention:                 Attention,
    /// Fade of the hover highlight of every module the pointer touched.
    pub hover:                     HoverFades<ModuleName>,
    /// The one tooltip lifecycle: dwell, warmth and the fade either way.
    pub hints:                     Hints,
    /// Notifications currently shown as popups.
    pub notification_popups:       Vec<notifications_popup::Popup>,
    /// Presence of the greeting shown mid-screen while the bar comes up.
    pub greeting:                  Spring,
    /// The greeting line, composed once when the greeting is armed.
    pub greeting_line:             String,
    /// The menu surfaces the greeting has raised, each exactly once.
    pub(crate) greeting_raised:    Vec<Id>,
    /// The frame instant past which the greeting lets itself out.
    pub(crate) greeting_deadline:  Option<Instant>,
    /// The wallpaper press waiting for its pictures before its window opens.
    pub(crate) wallpaper_pending:  Option<(Id, hydebar_core::position_button::ButtonUIRef)>,
    /// The layout press waiting for its roster before its window opens.
    pub(crate) bar_layout_pending: Option<(Id, hydebar_core::position_button::ButtonUIRef)>
}
