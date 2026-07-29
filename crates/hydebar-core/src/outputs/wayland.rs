use iced::{
    Task,
    platform_specific::{
        runtime::wayland::layer_surface::{IcedOutput, SctkLayerSurfaceSettings},
        shell::wayland::commands::layer_surface::{
            Anchor, KeyboardInteractivity, Layer, destroy_layer_surface, get_layer_surface
        }
    },
    window::Id
};
use wayland_client::protocol::wl_output::WlOutput;

use crate::{
    HEIGHT,
    config::{AppearanceStyle, BarLayer, Position}
};

/// Namespace of the surface the bar itself is drawn on.
///
/// It is what compositor rules are attached to, the blur behind the bar above
/// all, so it is stated once and read by everything that names it.
pub(crate) const MAIN_NAMESPACE: &str = "hydebar-main-layer";

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
pub(crate) const NOTIFICATIONS_WIDTH: u32 = 520;

/// Input region of a surface that only ever draws.
///
/// A layer surface that states no region at all takes pointer input over every
/// pixel it covers. The tooltip surface spans the whole screen the bar leaves
/// free, so that a hint can be drawn beside any module, and it rises to the
/// overlay for as long as one is shown: with the region unstated it takes every
/// press aimed at the desktop underneath it while the pointer merely rests on a
/// module. The notification surface does the same over the corner its popups
/// occupy.
///
/// An empty region states the opposite: the surface is painted and nothing
/// else, so presses keep reaching whatever sits below it.
fn draw_only() -> Option<Vec<iced::Rectangle>> {
    Some(Vec::new())
}

/// Maps the configured bar layer onto the compositor layer it is created on.
fn surface_layer(layer: BarLayer) -> Layer {
    match layer {
        BarLayer::Background => Layer::Background,
        BarLayer::Bottom => Layer::Bottom,
        BarLayer::Top => Layer::Top,
        BarLayer::Overlay => Layer::Overlay
    }
}

pub(crate) struct LayerSurfaceCreation<Message> {
    pub(crate) main_id:          Id,
    pub(crate) menu_id:          Id,
    pub(crate) tooltip_id:       Id,
    pub(crate) notifications_id: Id,
    pub(crate) task:             Task<Message>
}

/// Returns the height the bar layer-surface is created with, in physical
/// pixels.
///
/// `configured_height` is the height named by the configuration; left unset the
/// built-in [`HEIGHT`] is used instead. Either way the solid and gradient
/// styles trim the island margin and the result is scaled by `scale_factor`.
pub(crate) fn layer_height(
    style: AppearanceStyle,
    scale_factor: f64,
    configured_height: Option<f32>
) -> f64 {
    let base = configured_height.map_or(HEIGHT, f64::from);

    (base
        - match style {
            AppearanceStyle::Solid | AppearanceStyle::Gradient => 8.,
            AppearanceStyle::Islands => 0.
        })
        * scale_factor
}

/// Resolves the output a surface is created on.
fn on_output(wl_output: Option<WlOutput>) -> IcedOutput {
    wl_output.map_or(IcedOutput::Active, IcedOutput::Output)
}

/// Settings of the surface the bar itself is drawn on.
pub(crate) fn main_settings(
    id: Id,
    style: AppearanceStyle,
    wl_output: Option<WlOutput>,
    position: Position,
    menu_keyboard_focus: bool,
    scale_factor: f64,
    configured_height: Option<f32>,
    layer: BarLayer
) -> SctkLayerSurfaceSettings {
    let height = layer_height(style, scale_factor, configured_height);

    SctkLayerSurfaceSettings {
        id,
        namespace: MAIN_NAMESPACE.to_string(),
        size: Some((None, Some(height as u32))),
        layer: surface_layer(layer),
        keyboard_interactivity: if menu_keyboard_focus {
            KeyboardInteractivity::OnDemand
        } else {
            KeyboardInteractivity::None
        },
        exclusive_zone: height as i32,
        output: on_output(wl_output),
        anchor: match position {
            Position::Top => Anchor::TOP,
            Position::Bottom => Anchor::BOTTOM
        } | Anchor::LEFT
            | Anchor::RIGHT,
        ..Default::default()
    }
}

/// Settings of the surface the menus are drawn on.
///
/// It states no input region on purpose: the menu is dismissed by pressing
/// beside it, which only reaches the bar while the surface takes the press.
pub(crate) fn menu_settings(id: Id, wl_output: Option<WlOutput>) -> SctkLayerSurfaceSettings {
    SctkLayerSurfaceSettings {
        id,
        namespace: MENU_NAMESPACE.to_string(),
        size: Some((None, None)),
        layer: Layer::Background,
        keyboard_interactivity: KeyboardInteractivity::None,
        output: on_output(wl_output),
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        ..Default::default()
    }
}

/// Settings of the surface the tooltips are drawn on.
pub(crate) fn tooltip_settings(id: Id, wl_output: Option<WlOutput>) -> SctkLayerSurfaceSettings {
    SctkLayerSurfaceSettings {
        id,
        namespace: TOOLTIP_NAMESPACE.to_string(),
        size: Some((None, None)),
        layer: Layer::Background,
        keyboard_interactivity: KeyboardInteractivity::None,
        input_zone: draw_only(),
        output: on_output(wl_output),
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        ..Default::default()
    }
}

/// Settings of the surface the notification popups are drawn on.
pub(crate) fn notifications_settings(
    id: Id,
    wl_output: Option<WlOutput>,
    position: Position
) -> SctkLayerSurfaceSettings {
    SctkLayerSurfaceSettings {
        id,
        namespace: NOTIFICATIONS_NAMESPACE.to_string(),
        size: Some((Some(NOTIFICATIONS_WIDTH), None)),
        // Parked out of the way until a popup arrives. The compositor stacks a
        // surface that changes layer above everything already on that layer, so
        // a surface that sat on the overlay from the start would end up below a
        // menu raised there later. Rising only when there is something to show
        // is what keeps the popups above whatever the bar raised before them.
        layer: Layer::Background,
        keyboard_interactivity: KeyboardInteractivity::None,
        input_zone: draw_only(),
        output: on_output(wl_output),
        anchor: match position {
            Position::Top => Anchor::TOP,
            Position::Bottom => Anchor::BOTTOM
        } | Anchor::RIGHT,
        ..Default::default()
    }
}

/// Creates every surface an output is drawn on.
///
/// The blur is asked for first: the compositor reads its layer rules when a
/// surface is mapped, so a rule stated afterwards would only reach the surface
/// the next time the bar is started.
pub(crate) fn create_layer_surfaces<Message: 'static>(
    style: AppearanceStyle,
    wl_output: Option<WlOutput>,
    position: Position,
    menu_keyboard_focus: bool,
    scale_factor: f64,
    configured_height: Option<f32>,
    layer: BarLayer
) -> LayerSurfaceCreation<Message> {
    super::blur::request();

    let main_id = Id::unique();
    let menu_id = Id::unique();
    let tooltip_id = Id::unique();
    let notifications_id = Id::unique();

    let main_task = get_layer_surface(main_settings(
        main_id,
        style,
        wl_output.clone(),
        position,
        menu_keyboard_focus,
        scale_factor,
        configured_height,
        layer
    ));
    let menu_task = get_layer_surface(menu_settings(menu_id, wl_output.clone()));
    let tooltip_task = get_layer_surface(tooltip_settings(tooltip_id, wl_output.clone()));
    let notifications_task = get_layer_surface(notifications_settings(
        notifications_id,
        wl_output,
        position
    ));

    LayerSurfaceCreation {
        main_id,
        menu_id,
        tooltip_id,
        notifications_id,
        task: Task::batch(vec![main_task, menu_task, tooltip_task, notifications_task])
    }
}

pub(crate) fn destroy_layer_surfaces<Message: 'static>(
    main_id: Id,
    menu_id: Id,
    tooltip_id: Id,
    notifications_id: Id
) -> Task<Message> {
    Task::batch(vec![
        destroy_layer_surface(main_id),
        destroy_layer_surface(menu_id),
        destroy_layer_surface(tooltip_id),
        destroy_layer_surface(notifications_id),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tooltip_surface_takes_no_pointer_input() {
        // it spans the screen and rises above the windows while a hint is up,
        // so an unstated region would take every press meant for the bar
        assert_eq!(
            tooltip_settings(Id::unique(), None).input_zone,
            Some(Vec::new())
        );
    }

    #[test]
    fn the_notification_surface_takes_no_pointer_input() {
        assert_eq!(
            notifications_settings(Id::unique(), None, Position::Top).input_zone,
            Some(Vec::new())
        );
    }

    #[test]
    fn the_menu_surface_keeps_taking_pointer_input() {
        // pressing beside an open menu is what closes it
        assert!(menu_settings(Id::unique(), None).input_zone.is_none());
    }

    #[test]
    fn the_bar_surface_keeps_taking_pointer_input() {
        let settings = main_settings(
            Id::unique(),
            AppearanceStyle::Islands,
            None,
            Position::Top,
            false,
            1.0,
            None,
            BarLayer::Top
        );

        assert!(settings.input_zone.is_none());
    }

    #[test]
    fn maps_every_configured_layer_onto_its_compositor_counterpart() {
        // a bar meant to be blurred has to leave the levels the blur source is
        // composited from
        assert!(matches!(
            surface_layer(BarLayer::Background),
            Layer::Background
        ));
        assert!(matches!(surface_layer(BarLayer::Bottom), Layer::Bottom));
        assert!(matches!(surface_layer(BarLayer::Top), Layer::Top));
        assert!(matches!(surface_layer(BarLayer::Overlay), Layer::Overlay));
    }

    #[test]
    fn an_unset_height_keeps_the_built_in_one() {
        assert_eq!(layer_height(AppearanceStyle::Islands, 1.0, None), HEIGHT);
        assert_eq!(layer_height(AppearanceStyle::Solid, 1.0, None), HEIGHT - 8.);
        assert_eq!(
            layer_height(AppearanceStyle::Gradient, 1.0, None),
            HEIGHT - 8.
        );
        assert_eq!(
            layer_height(AppearanceStyle::Islands, 1.5, None),
            HEIGHT * 1.5
        );
    }

    #[test]
    fn a_configured_height_replaces_the_built_in_one() {
        assert_eq!(
            layer_height(AppearanceStyle::Islands, 1.0, Some(38.0)),
            38.0
        );
        assert_eq!(layer_height(AppearanceStyle::Solid, 1.0, Some(38.0)), 30.0);
        assert_eq!(
            layer_height(AppearanceStyle::Gradient, 1.0, Some(38.0)),
            30.0
        );
    }

    #[test]
    fn a_configured_height_is_still_scaled() {
        assert_eq!(
            layer_height(AppearanceStyle::Islands, 2.0, Some(38.0)),
            76.0
        );
        assert_eq!(layer_height(AppearanceStyle::Solid, 0.5, Some(38.0)), 15.0);
    }
}
