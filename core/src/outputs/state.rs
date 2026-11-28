mod keyboard;
mod lifecycle;
mod lookup;
mod menus;
mod resize;
mod sync;
mod tooltips;

use iced::{OutputId, SurfaceId as Id, Task};

use super::wayland::{LayerSurfaceCreation, create_layer_surfaces};
use crate::{
    config::{AppearanceStyle, ModuleName, Position},
    frame::PanelWidths,
    menu::{Menu, MenuType},
    position_button::ButtonUIRef,
    tooltip::TooltipInfo
};

#[derive(Debug, Clone)]
struct ShellInfo {
    id:               Id,
    position:         Position,
    style:            AppearanceStyle,
    menu:             Menu,
    scale_factor:     f64,
    /// Bar height the surface was created with, as named by the configuration.
    height:           Option<f32>,
    /// Surface the tooltips of this output are drawn on.
    tooltip_id:       Id,
    /// Tooltip that surface is showing, if any, and the module it belongs to.
    ///
    /// The owner is kept because enter and leave arrive in widget order, not
    /// in pointer order: moving towards the start of the bar delivers the next
    /// module's enter before the previous module's leave, and a leave that
    /// hid whatever is showing would take the fresh tooltip down with it.
    tooltip:          Option<(ModuleName, TooltipInfo)>,
    /// Surface the desk of this output is drawn on.
    desk_id:          Id,
    /// Surface the ring of this output is painted on.
    ///
    /// The ring is the fullscreen, click-through paint of the frame; it is
    /// absent while the frame is switched off or while no real screen owns
    /// this group yet, which is why the identifier is optional.
    frame_id:         Option<Id>,
    /// Whether the frame was switched on when this group was last synced.
    ///
    /// The stored switch is what the next sync reads before it decides the
    /// ring surface has to be created or destroyed, so a config flip never
    /// leaves a stale fullscreen surface behind.
    frame_enabled:    bool,
    /// Reservation widths the panels were last sized to, in layer-shell
    /// pixels.
    ///
    /// A flipped edge width is a `set_size` away from real; comparing the
    /// stored numbers against the freshly computed ones is how the sync
    /// resizes exactly the panels that moved.
    reservations:     PanelWidths,
    /// Surface the left panel of this output is drawn on.
    left_panel_id:    Option<Id>,
    /// Surface the right panel of this output is drawn on.
    right_panel_id:   Option<Id>,
    /// Surface the bottom panel of this output is drawn on.
    bottom_panel_id:  Option<Id>,
    /// Surface the top panel of this output is drawn on.
    top_panel_id:     Option<Id>,
    /// Surface the notification popups of this output are drawn on.
    notifications_id: Id
}

impl ShellInfo {
    /// Reports whether `id` names one of the surfaces of this output.
    fn owns(&self, id: Id) -> bool {
        self.id == id
            || self.menu.id == id
            || self.tooltip_id == id
            || self.desk_id == id
            || self.frame_id == Some(id)
            || self.left_panel_id == Some(id)
            || self.right_panel_id == Some(id)
            || self.bottom_panel_id == Some(id)
            || self.top_panel_id == Some(id)
    }
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
pub struct Outputs(Vec<(Option<String>, Option<ShellInfo>, Option<OutputId>)>);

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
    Menu(Option<&'a (MenuType, ButtonUIRef)>),
    /// The identifier refers to the surface the tooltips are drawn on.
    ///
    /// What it shows is read back with [`Outputs::tooltip`].
    Tooltip,
    /// The identifier refers to the surface the desk is drawn on.
    ///
    /// What screen it stands on is read back with [`Outputs::screen_of`].
    Desk,
    /// The identifier refers to the surface the ring of the frame is painted
    /// on.
    ///
    /// What screen it stands on is read back with [`Outputs::screen_of`].
    Frame,
    /// The identifier refers to the surface one panel of the frame is drawn
    /// on.
    ///
    /// Which edge it stands on is read back with [`Outputs::panel_edge_of`].
    Panel,
    /// The identifier refers to the surface the notification popups are drawn
    /// on.
    Notifications
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
    #[must_use]
    pub fn new<Message: 'static>(
        style: AppearanceStyle,
        position: Position,
        config: &crate::config::Config
    ) -> (Self, Task<Message>) {
        let LayerSurfaceCreation {
            main_id,
            menu_id,
            tooltip_id,
            desk_id,
            notifications_id,
            task
        } = create_layer_surfaces(
            style,
            None,
            position,
            config.menu_keyboard_focus,
            config.appearance.scale_factor,
            config.appearance.height_px(),
            config.layer
        );

        (
            Self(vec![(
                None,
                Some(ShellInfo {
                    id: main_id,
                    menu: Menu::new(menu_id, None),
                    position,
                    style,
                    scale_factor: config.appearance.scale_factor,
                    height: config.appearance.height_px(),
                    tooltip_id,
                    tooltip: None,
                    desk_id,
                    notifications_id,
                    frame_id: None,
                    frame_enabled: false,
                    reservations: PanelWidths::default(),
                    left_panel_id: None,
                    right_panel_id: None,
                    bottom_panel_id: None,
                    top_panel_id: None
                }),
                None
            )]),
            task
        )
    }
}
