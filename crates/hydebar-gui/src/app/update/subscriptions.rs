//! Event sources the application listens to.

use std::{sync::Arc, time::Instant};

use hydebar_core::{
    config::{self, ConfigEvent},
    modules::themes
};
use iced::{Subscription, event::listen_with, keyboard};
use log::debug;

use super::super::{
    bus, shutdown,
    state::{App, Message}
};

/// How often the popups on screen are checked for expiry.
///
/// Only ticks while something is on screen, so an idle bar keeps costing
/// nothing: the frame clock the rest of the bar rides on stops when nothing
/// animates, and a popup would otherwise never be taken down.
const POPUP_TICK_MS: u64 = 250;

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
    /// Maps a watcher event onto the messages the reload path already handles.
    fn config_event(event: ConfigEvent) -> Message {
        match event {
            ConfigEvent::Applied(config) => Message::ConfigChanged(config),
            ConfigEvent::Degraded(degradation) => Message::ConfigDegraded(degradation)
        }
    }

    /// Follows the desktop theme so a `HyDE` theme switch repaints the bar.
    ///
    /// Kept separate from the configuration watcher because the theme lives in
    /// files the bar does not own, and because a configuration that pins its
    /// own colours must not be disturbed by a theme switch at all.
    fn theme_subscription(&self) -> Subscription<Message> {
        config::theme_subscription(
            &self.config_path,
            Arc::clone(&self.config_manager),
            self.config.appearance.follow_hyde
        )
        .map(Self::config_event)
    }

    /// Keeps the indicator of a running desktop change moving.
    ///
    /// A `HyDE` theme switch takes seconds and the bar has nothing to do for
    /// any of them, so nothing would otherwise make it redraw: the
    /// indicator would be drawn once on the press and then stand still,
    /// which is exactly what a hung bar looks like. The tick runs only
    /// while a switch is running, and on the indicator's own cadence rather
    /// than on the frame clock, so a wait costs a handful of redraws a
    /// second instead of one per refresh.
    fn switch_subscription(&self) -> Subscription<Message> {
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
    fn wallpaper_loading_subscription(&self) -> Subscription<Message> {
        if self.wallpaper.is_loading() {
            Self::clock(Clock::WallpaperSpinner, themes::FRAME_INTERVAL)
                .map(|_| Message::Wallpaper(hydebar_core::modules::wallpaper::Message::Tick))
        } else {
            Subscription::none()
        }
    }

    /// Tick of the layout entry's loading indicator.
    fn bar_layout_loading_subscription(&self) -> Subscription<Message> {
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
    fn rest_clock(&self) -> Subscription<Message> {
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
    fn attended_clock(&self) -> Subscription<Message> {
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
    fn frame_subscription(&self) -> Subscription<Message> {
        if self.outputs.menu_is_animating()
            || self.appearance_transition.is_animating()
            || self.hover.is_animating()
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

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            bus::subscription(self.bus_receiver.clone()).map(Message::BusFlushed),
            shutdown::subscription().map(Message::Shutdown),
            self.frame_subscription(),
            self.wallpaper_loading_subscription(),
            self.bar_layout_loading_subscription(),
            self.rest_clock(),
            self.attended_clock(),
            self.switch_subscription(),
            config::subscription(&self.config_path, Arc::clone(&self.config_manager))
                .map(Self::config_event),
            self.theme_subscription(),
            iced::output_events().map(Message::OutputEvent),
            listen_with(|evt, _, _| match evt {
                iced_core::Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) => {
                    debug!("Keyboard event received: {key:?}, modifiers: {modifiers:?}");

                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
                        debug!("ESC key pressed");
                        return Some(Message::DeactivateNavigationMode);
                    }

                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Enter)) {
                        debug!("Enter pressed");
                        return Some(Message::ActivateFocusedModule);
                    }

                    if let keyboard::Key::Character(ref ch) = key {
                        let ch_str = ch.as_str();

                        if modifiers.logo() && ch_str == "b" {
                            debug!("Super+b detected, activating navigation mode");
                            return Some(Message::ActivateNavigationMode);
                        }

                        if ch_str == "k" {
                            debug!("Navigate up: k");
                            return Some(Message::NavigateUp);
                        } else if ch_str == "j" {
                            debug!("Navigate down: j");
                            return Some(Message::NavigateDown);
                        } else if ch_str == "h" {
                            debug!("Navigate left: h");
                            return Some(Message::NavigateLeft);
                        } else if ch_str == "l" {
                            debug!("Navigate right: l");
                            return Some(Message::NavigateRight);
                        }
                    }

                    None
                }
                _ => None
            }),
        ];

        subscriptions.extend(self.modules_subscriptions(&self.config.modules.left));
        subscriptions.extend(self.modules_subscriptions(&self.config.modules.center));
        subscriptions.extend(self.modules_subscriptions(&self.config.modules.right));

        if self.config.notifications.source.owns_the_bus() {
            use hydebar_core::modules::Module;

            if let Some(notifications) = Module::<Message>::subscription(&self.notifications) {
                subscriptions.push(notifications);
            }
        }

        if hydebar_core::notifications_popup::needs_expiry_clock(&self.notification_popups) {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_millis(POPUP_TICK_MS))
                    .map(|_| Message::ExpirePopups)
            );
        }

        Subscription::batch(subscriptions)
    }
}
