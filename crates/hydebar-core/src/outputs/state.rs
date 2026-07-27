mod keyboard;
mod lifecycle;
mod lookup;
mod menus;
mod sync;

#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests;

use iced::{Task, window::Id};
use wayland_client::protocol::wl_output::WlOutput;

use super::wayland::{LayerSurfaceCreation, create_layer_surfaces};
use crate::{
    config::{AppearanceStyle, Position},
    menu::{Menu, MenuType},
    position_button::ButtonUIRef
};

#[derive(Debug, Clone)]
struct ShellInfo {
    id:           Id,
    position:     Position,
    style:        AppearanceStyle,
    menu:         Menu,
    scale_factor: f64
}

/// Collection of Wayland outputs currently tracked by the bar.
///
/// Instances manage Wayland layer-surfaces for both the main bar surface and
/// the associated menu surface per monitor. All operations return [`Task`]
/// objects that must be executed by the caller to coordinate with the
/// compositor.
///
/// # Examples
///
/// ```
/// # use hydebar_core::outputs::Outputs;
/// # use hydebar_core::config::Config;
/// let config = Config::default();
/// let (outputs, _task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
/// assert!(!outputs.menu_is_open());
/// ```
#[derive(Debug, Clone)]
pub struct Outputs(Vec<(Option<String>, Option<ShellInfo>, Option<WlOutput>)>);

/// Result of looking up a Wayland surface identifier.
///
/// The lookup differentiates between the main bar surface and the menu surface
/// so that event handlers can update the appropriate component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HasOutput<'a> {
    /// The identifier refers to the main bar surface.
    Main,
    /// The identifier refers to the menu surface along with its optional
    /// metadata about the menu currently shown.
    Menu(Option<&'a (MenuType, ButtonUIRef)>)
}

impl Outputs {
    /// Construct a new collection with a fallback surface that is active even
    /// before the compositor reports specific monitors.
    ///
    /// The returned [`Task`] must be spawned so that the fallback layer-surface
    /// is created. Once actual monitors appear, [`Outputs::add`] replaces this
    /// fallback entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::outputs::Outputs;
    /// # use hydebar_core::config::Config;
    /// let config = Config::default();
    /// let (outputs, task) = Outputs::new::<()>(config.appearance.style, config.position, &config);
    /// assert!(!outputs.menu_is_open());
    /// # let _ = task;
    /// ```
    pub fn new<Message: 'static>(
        style: AppearanceStyle,
        position: Position,
        config: &crate::config::Config
    ) -> (Self, Task<Message>) {
        let LayerSurfaceCreation {
            main_id,
            menu_id,
            task
        } = create_layer_surfaces(
            style,
            None,
            position,
            config.menu_keyboard_focus,
            config.appearance.scale_factor,
            config.layer
        );

        (
            Self(vec![(
                None,
                Some(ShellInfo {
                    id: main_id,
                    menu: Menu::new(menu_id),
                    position,
                    style,
                    scale_factor: config.appearance.scale_factor
                }),
                None
            )]),
            task
        )
    }

    #[cfg(test)]
    fn iter_internal(
        &self
    ) -> impl Iterator<Item = &(Option<String>, Option<ShellInfo>, Option<WlOutput>)> {
        self.0.iter()
    }
}
