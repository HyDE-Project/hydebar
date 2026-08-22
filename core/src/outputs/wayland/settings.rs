//! What each of a screen's surfaces asks the compositor for.

use iced::{Anchor, KeyboardInteractivity, Layer, LayerShellSettings, OutputId};

use super::{
    DESK_NAMESPACE, MAIN_NAMESPACE, MENU_NAMESPACE, NOTIFICATIONS_NAMESPACE, NOTIFICATIONS_WIDTH,
    TOOLTIP_NAMESPACE
};
use crate::{
    HEIGHT,
    config::{AppearanceStyle, BarLayer, Position}
};

const fn surface_layer(layer: BarLayer) -> Layer {
    match layer {
        BarLayer::Background => Layer::Background,
        BarLayer::Bottom => Layer::Bottom,
        BarLayer::Top => Layer::Top,
        BarLayer::Overlay => Layer::Overlay
    }
}

pub fn layer_height(
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

/// Settings of the surface the bar itself is drawn on.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the layer height is a small positive pixel count"
)]
pub fn main_settings(
    style: AppearanceStyle,
    output: Option<OutputId>,
    position: Position,
    menu_keyboard_focus: bool,
    scale_factor: f64,
    configured_height: Option<f32>,
    layer: BarLayer
) -> LayerShellSettings {
    let height = layer_height(style, scale_factor, configured_height);

    LayerShellSettings {
        namespace: MAIN_NAMESPACE.to_string(),
        size: Some((0, height as u32)),
        layer: surface_layer(layer),
        keyboard_interactivity: if menu_keyboard_focus {
            KeyboardInteractivity::OnDemand
        } else {
            KeyboardInteractivity::None
        },
        exclusive_zone: height as i32,
        output,
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
/// It keeps its pointer input on purpose: the menu is dismissed by pressing
/// beside it, which only reaches the bar while the surface takes the press.
pub fn menu_settings(output: Option<OutputId>) -> LayerShellSettings {
    LayerShellSettings {
        namespace: MENU_NAMESPACE.to_string(),
        size: Some((0, 0)),
        layer: Layer::Background,
        keyboard_interactivity: KeyboardInteractivity::None,
        output,
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        ..Default::default()
    }
}

/// Settings of the surface the tooltips are drawn on.
///
/// Created input-free through [`draw_only`], stated as a follow-up task.
pub fn tooltip_settings(output: Option<OutputId>) -> LayerShellSettings {
    LayerShellSettings {
        namespace: TOOLTIP_NAMESPACE.to_string(),
        size: Some((0, 0)),
        layer: Layer::Background,
        keyboard_interactivity: KeyboardInteractivity::None,
        output,
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        ..Default::default()
    }
}

/// Settings of the surface the desk is drawn on.
///
/// Created input-free through [`draw_only`], on the same level as the strip
/// it unfolds out of. The two are one thing in two shapes, and a compositor
/// blurs a layer surface by blurring whatever it finds behind it: a canvas
/// laid below the strip had every module blurred and dimmed for the frames it
/// spent crossing the strip's own band, so a block set off sharp, went to mush
/// on the way out of the bar and came back sharp underneath it. Sharing the
/// strip's level, and ordered to be drawn after it, is what leaves a departing
/// block in front of the background it is leaving rather than behind it.
///
/// Its exclusive zone is stated as -1, which asks the compositor to lay the
/// surface out over the whole screen rather than over what the bar's own
/// strip leaves free. The strip's band has to belong to the canvas too: the
/// modules leave the strip by travelling out of it, and a canvas that began
/// below the strip would have them jump the height of the bar before they
/// started moving.
pub fn desk_settings(output: Option<OutputId>, layer: BarLayer) -> LayerShellSettings {
    LayerShellSettings {
        namespace: DESK_NAMESPACE.to_string(),
        size: Some((0, 0)),
        layer: surface_layer(layer),
        keyboard_interactivity: KeyboardInteractivity::None,
        output,
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        exclusive_zone: -1,
        ..Default::default()
    }
}

/// Settings of the surface the notification popups are drawn on.
///
/// Created input-free through [`draw_only`], stated as a follow-up task.
/// Parked out of the way until a popup arrives: the compositor stacks a
/// surface that changes layer above everything already on that layer, so a
/// surface that sat on the overlay from the start would end up below a menu
/// raised there later. Rising only when there is something to show is what
/// keeps the popups above whatever the bar raised before them.
pub fn notifications_settings(output: Option<OutputId>, position: Position) -> LayerShellSettings {
    LayerShellSettings {
        namespace: NOTIFICATIONS_NAMESPACE.to_string(),
        size: Some((NOTIFICATIONS_WIDTH, 1)),
        layer: Layer::Background,
        keyboard_interactivity: KeyboardInteractivity::None,
        output,
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the built-in height is a small positive pixel count"
    )]
    fn the_bar_surface_reserves_its_strip() {
        let settings = main_settings(
            AppearanceStyle::Islands,
            None,
            Position::Top,
            false,
            1.0,
            None,
            BarLayer::Top
        );

        assert_eq!(settings.exclusive_zone, HEIGHT as i32);
        assert_eq!(settings.size, Some((0, HEIGHT as u32)));
    }

    #[test]
    fn every_surface_carries_its_own_namespace() {
        // compositor rules key on the namespace; a shared one would blur the
        // whole desktop the moment a menu covers it
        assert_eq!(menu_settings(None).namespace, MENU_NAMESPACE);
        assert_eq!(tooltip_settings(None).namespace, TOOLTIP_NAMESPACE);
        assert_eq!(desk_settings(None, BarLayer::Top).namespace, DESK_NAMESPACE);
        assert_eq!(
            notifications_settings(None, Position::Top).namespace,
            NOTIFICATIONS_NAMESPACE
        );
    }

    #[test]
    fn the_desk_shares_the_strips_level_and_covers_the_whole_screen() {
        // a canvas laid out below the strip would make every module jump the
        // height of the bar before it started travelling, and one on a level
        // under the strip would have the strip's own blur eat every module
        // crossing its band
        for layer in [BarLayer::Background, BarLayer::Bottom, BarLayer::Top] {
            let settings = desk_settings(None, layer);

            assert_eq!(settings.layer, surface_layer(layer));
            assert_eq!(settings.exclusive_zone, -1);
        }
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
