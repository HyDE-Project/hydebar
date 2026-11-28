//! Reads the desktop performs off the drawing thread.

use hydebar_core::outputs::scaling::screen_geometry;
use iced::Task;

use crate::app::state::Message;

/// Reads the wallpaper in force off the drawing thread.
///
/// Decoding even a thumbnail costs more than a frame is worth, and the answer
/// is wanted once per wallpaper rather than once per repaint.
pub fn read_wallpaper() -> Task<Message> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(hydebar_core::modules::wallpaper::current::current)
                .await
                .unwrap_or_default()
        },
        Message::WallpaperRead
    )
}

/// Reads the pictures the desktop keeps of its own look, off the drawing
/// thread.
///
/// Hashing wallpapers and decoding a row of crops costs far more than a frame,
/// and the answer is wanted once per change of look rather than once per
/// repaint.
pub fn read_looks() -> Task<Message> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(hydebar_core::modules::desk::looks::looks)
                .await
                .unwrap_or_default()
        },
        |looks| Message::LooksRead(Box::new(looks))
    )
}

/// Asks the compositor for the screen's geometry off the drawing thread.
///
/// The question travels the compositor's own socket and the compositor may
/// answer slowly; the drawing thread must never stand waiting on it, so the
/// answer comes back as a message of its own.
pub fn measure_screen(name: String) -> Task<Message> {
    Task::perform(
        async move {
            let answer = screen_geometry(&name).await;

            (name, answer)
        },
        |(name, geometry)| Message::ScreenMeasured {
            name,
            geometry
        }
    )
}
