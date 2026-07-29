//! Event sources the application listens to.

use std::sync::Arc;

use hydebar_core::config::{self, ConfigEvent};
use iced::{
    Subscription,
    event::{listen_with, wayland::Event as WaylandEvent},
    keyboard, window
};
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

impl App {
    /// Maps a watcher event onto the messages the reload path already handles.
    fn config_event(event: ConfigEvent) -> Message {
        match event {
            ConfigEvent::Applied(config) => Message::ConfigChanged(config),
            ConfigEvent::Degraded(degradation) => Message::ConfigDegraded(degradation)
        }
    }

    /// Follows the desktop theme so a HyDE theme switch repaints the bar.
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

    /// of interpolating on a polling timer.
    fn frame_subscription(&self) -> Subscription<Message> {
        if self.outputs.menu_is_animating() || self.appearance_transition.is_animating() {
            window::wayland_frames().map(Message::Frame)
        } else {
            Subscription::none()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            bus::subscription(self.bus_receiver.clone()).map(Message::BusFlushed),
            shutdown::subscription().map(Message::Shutdown),
            self.frame_subscription(),
            config::subscription(&self.config_path, Arc::clone(&self.config_manager))
                .map(Self::config_event),
            self.theme_subscription(),
            listen_with(|evt, _, _| match evt {
                iced::Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
                    WaylandEvent::Output(event, wl_output)
                )) => {
                    debug!("Wayland event: {event:?}");
                    Some(Message::OutputEvent((event, wl_output)))
                }
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
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
