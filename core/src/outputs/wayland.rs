//! The layer surfaces one screen is drawn on, and what each of them is.
//!
//! Two rooms. The namespaces live here, because every compositor rule the bar
//! states keys on one of them and a rule that named a surface by anything else
//! would be a rule about nothing. [`settings`] is what each surface is asked
//! for — its level, its anchors, the band it reserves — and this file is the
//! creating and the destroying of the set a screen needs.

mod settings;

use iced::{
    OutputId, SurfaceId as Id, Task, destroy_layer_surface, new_layer_surface, set_input_region
};
pub use settings::{
    desk_settings, layer_height, main_settings, menu_settings, notifications_settings,
    tooltip_settings
};

use crate::config::{AppearanceStyle, BarLayer, Position};

/// Namespace of the surface the bar itself is drawn on.
///
/// It is what compositor rules are attached to, the blur behind the bar above
/// all, so it is stated once and read by everything that names it.
pub const MAIN_NAMESPACE: &str = "hydebar-main-layer";

/// Namespace of the surface the tooltips are drawn on.
///
/// It is deliberately not the namespace of the bar and its menus: compositor
/// rules attached to those, a blur behind the menu backdrop above all, would
/// otherwise fire for every hover.
const TOOLTIP_NAMESPACE: &str = "hydebar-tooltip-layer";

/// Namespace of the full screen surface the menus are drawn on.
///
/// Kept apart from the bar for the same reason as the tooltips: a compositor
/// blur rule matching the bar would otherwise blur the whole desktop as soon
/// as a menu surface covers it.
const MENU_NAMESPACE: &str = "hydebar-menu-layer";

/// Namespace of the surface the desk is drawn on.
///
/// Kept apart from the menus for the same reason as the tooltips: the desk
/// covers the whole wallpaper for as long as the screen is bare, and a blur
/// rule written for the menu backdrop must not fire because a workspace was
/// cleared.
pub const DESK_NAMESPACE: &str = "hydebar-desk-layer";

/// Namespace of the surface the notification popups are drawn on.
///
/// Kept apart from the menus for the same reason as the tooltips: a compositor
/// rule attached to a menu must not fire because a notification arrived.
const NOTIFICATIONS_NAMESPACE: &str = "hydebar-notifications-layer";

/// Width of the strip the popups live on, in physical pixels.
///
/// Deliberately narrow and anchored to one corner rather than covering the
/// screen: a full screen surface on a layer above the desktop swallows every
/// click meant for the windows underneath it.
pub const NOTIFICATIONS_WIDTH: u32 = 520;

/// Strips a surface of pointer input, leaving it a drawing and nothing else.
///
/// A layer surface that states no region takes pointer input over every pixel
/// it covers. The tooltip surface spans the whole screen the bar leaves free,
/// so that a hint can be drawn beside any module, and it rises to the overlay
/// for as long as one is shown: left as it is it would take every press aimed
/// at the desktop underneath it while the pointer merely rests on a module.
/// The notification surface does the same over the corner its popups occupy.
///
/// The empty region is stated *after* creation because the settings carry no
/// region of their own.
fn draw_only<Message: 'static>(id: Id) -> Task<Message> {
    set_input_region(id, Some(Vec::new()))
}

/// The surfaces one screen was given, and the task that creates them.
pub struct LayerSurfaceCreation<Message> {
    pub(crate) main_id:          Id,
    pub(crate) menu_id:          Id,
    pub(crate) tooltip_id:       Id,
    pub(crate) desk_id:          Id,
    pub(crate) notifications_id: Id,
    pub(crate) task:             Task<Message>
}

/// Creates every surface an output is drawn on.
///
/// The blur is asked for first: the compositor reads its layer rules when a
/// surface is mapped, so a rule stated afterwards would only reach the surface
/// the next time the bar is started.
pub fn create_layer_surfaces<Message: 'static>(
    style: AppearanceStyle,
    output: Option<OutputId>,
    position: Position,
    menu_keyboard_focus: bool,
    scale_factor: f64,
    configured_height: Option<f32>,
    layer: BarLayer
) -> LayerSurfaceCreation<Message> {
    super::restate_blur();

    let (main_id, main_task) = new_layer_surface(main_settings(
        style,
        output,
        position,
        menu_keyboard_focus,
        scale_factor,
        configured_height,
        layer
    ));
    let (menu_id, menu_task) = new_layer_surface(menu_settings(output));
    let (tooltip_id, tooltip_task) = new_layer_surface(tooltip_settings(output));
    let (desk_id, desk_task) = new_layer_surface(desk_settings(output, layer));
    let (notifications_id, notifications_task) =
        new_layer_surface(notifications_settings(output, position));

    LayerSurfaceCreation {
        main_id,
        menu_id,
        tooltip_id,
        desk_id,
        notifications_id,
        task: Task::batch(vec![
            main_task,
            menu_task,
            tooltip_task,
            draw_only(tooltip_id),
            desk_task,
            draw_only(desk_id),
            notifications_task,
            draw_only(notifications_id),
        ])
    }
}

pub fn destroy_layer_surfaces<Message: 'static>(
    main_id: Id,
    menu_id: Id,
    tooltip_id: Id,
    desk_id: Id,
    notifications_id: Id
) -> Task<Message> {
    Task::batch(vec![
        destroy_layer_surface(main_id),
        destroy_layer_surface(menu_id),
        destroy_layer_surface(tooltip_id),
        destroy_layer_surface(desk_id),
        destroy_layer_surface(notifications_id),
    ])
}
