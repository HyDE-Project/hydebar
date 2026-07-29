//! Resizing the bar surfaces after the appearance changed.
//!
//! The height of a layer surface is fixed when it is created. A bar that lets
//! its height be changed, by the settings window or by the screen it landed on,
//! has to tell the compositor about it, otherwise the new height only reaches
//! the drawing and the strip the bar occupies stays as it was.

use iced::{
    Task,
    platform_specific::shell::wayland::commands::layer_surface::{set_exclusive_zone, set_size}
};

use super::Outputs;
use crate::{
    config::AppearanceStyle,
    outputs::wayland::{NOTIFICATIONS_WIDTH, layer_height}
};

impl Outputs {
    /// Re-states the height of every notification surface.
    ///
    /// Grown to what the popups need and shrunk back once they are gone, so the
    /// strip never covers more of the screen than it is drawing on.
    pub fn resize_notifications<Message: 'static>(&mut self, height: u32) -> Task<Message> {
        let tasks = self
            .notification_ids()
            .map(|id| set_size(id, Some(NOTIFICATIONS_WIDTH), Some(height.max(1))))
            .collect::<Vec<_>>();

        Task::batch(tasks)
    }

    /// Re-states the height of every bar surface.
    ///
    /// Both the size and the exclusive zone are sent: the first decides how
    /// tall the bar is drawn, the second how much room the windows below it
    /// leave free, and letting them drift apart is what leaves a gap or an
    /// overlap along the edge of the screen.
    pub fn resize<Message: 'static>(
        &mut self,
        style: AppearanceStyle,
        scale_factor: f64,
        configured_height: Option<f32>
    ) -> Task<Message> {
        let height = layer_height(style, scale_factor, configured_height);
        let rounded = height.round().max(1.0);

        let tasks = self
            .main_ids()
            .flat_map(|id| {
                [
                    set_size(id, None, Some(rounded as u32)),
                    set_exclusive_zone(id, rounded as i32)
                ]
            })
            .collect::<Vec<_>>();

        Task::batch(tasks)
    }
}
