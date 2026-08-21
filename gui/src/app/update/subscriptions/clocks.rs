//! The clocks of the bar: polling, spinners and the frame tick.

use std::time::Instant;

use hydebar_core::modules::themes;
use iced::Subscription;

use super::super::super::state::{App, Message};

/// Which of the two clocks of the bar a tick came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Clock {
    /// Serves every pollable module the layout draws.
    Rest,
    /// Serves the one module the user is looking at.
    Attended,
    /// Serves the wallpaper entry's loading indicator.
    WallpaperSpinner,
    /// Serves the layout entry's loading indicator.
    BarLayoutSpinner
}

impl App {
    /// Keeps the indicator of a running desktop change moving.
    ///
    /// A `HyDE` theme switch takes seconds and the bar has nothing to do for
    /// any of them, so nothing would otherwise make it redraw: the
    /// indicator would be drawn once on the press and then stand still,
    /// which is exactly what a hung bar looks like. The tick runs only
    /// while a switch is running, and on the indicator's own cadence rather
    /// than on the frame clock, so a wait costs a handful of redraws a
    /// second instead of one per refresh.
    pub(super) fn switch_subscription(&self) -> Subscription<Message> {
        if self.themes.is_waiting() {
            iced::time::every(themes::FRAME_INTERVAL)
                .map(|_| Message::Themes(themes::Message::Tick))
        } else {
            Subscription::none()
        }
    }

    /// Tick of the wallpaper entry's loading indicator.
    ///
    /// Runs only while a listing is being read, on the indicator's own
    /// cadence, and under its own clock identity so it never collapses
    /// into the theme switch tick that shares the period.
    pub(super) fn wallpaper_loading_subscription(&self) -> Subscription<Message> {
        if self.wallpaper.is_loading() {
            Self::clock(Clock::WallpaperSpinner, themes::FRAME_INTERVAL)
                .map(|_| Message::Wallpaper(hydebar_core::modules::wallpaper::Message::Tick))
        } else {
            Subscription::none()
        }
    }

    /// Tick of the layout entry's loading indicator.
    pub(super) fn bar_layout_loading_subscription(&self) -> Subscription<Message> {
        if self.bar_layout.is_loading() {
            Self::clock(Clock::BarLayoutSpinner, themes::FRAME_INTERVAL)
                .map(|_| Message::BarLayout(hydebar_core::modules::bar_layout::Message::Tick))
        } else {
            Subscription::none()
        }
    }

    /// The clock refreshing what every module keeps on the bar.
    ///
    /// One clock for the whole bar rather than one per module, and none at all
    /// while the layout draws nothing that can be polled.
    pub(super) fn rest_clock(&self) -> Subscription<Message> {
        self.attention
            .rest_period()
            .map_or_else(Subscription::none, |period| {
                Self::clock(Clock::Rest, period).map(|_| Message::PollAtRest)
            })
    }

    /// The clock refreshing the module the user is looking at.
    ///
    /// It exists only while something is attended, so a bar nobody is touching
    /// carries no fast clock at all rather than one ticking on an empty roster.
    pub(super) fn attended_clock(&self) -> Subscription<Message> {
        self.attention
            .attended_period()
            .map_or_else(Subscription::none, |period| {
                Self::clock(Clock::Attended, period).map(|_| Message::PollAttended)
            })
    }

    /// A clock ticking every `period`, told apart from the other one by
    /// `which`.
    ///
    /// The runtime keys a subscription on what it was built from, so two clocks
    /// left to their bare periods would collapse into one the moment those
    /// periods matched, and only one of them would ever tick.
    fn clock(which: Clock, period: std::time::Duration) -> Subscription<(Clock, Instant)> {
        iced::time::every(period).with(which)
    }

    /// of interpolating on a polling timer.
    pub(super) fn frame_subscription(&self) -> Subscription<Message> {
        if self.outputs.menu_is_animating()
            || self.appearance_transition.is_animating()
            || self.hover.is_animating()
            || self.desk_fades.is_animating()
            || self.desk_blooms.is_animating()
            || self.desk_blooms_are_due()
            || self.entrance.is_animating()
            || self.relayout.is_animating()
            || self.greeting.is_animating()
            || self.greeting.target() > 0.0
            || self.hints.needs_frames()
            || self.clock.is_fading()
            || self.updates.is_fading()
            || self.keyboard_layout.is_fading()
            || self.keyboard_submap.is_fading()
            || self.battery.is_fading()
        {
            iced::time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::Frame(std::time::Instant::now()))
        } else {
            Subscription::none()
        }
    }
}
