//! The Catppuccin Macchiato preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Catppuccin Macchiato.
pub(super) fn catppuccin_macchiato() -> Appearance {
    Appearance {
        font_name:                None,
        font_size:                None,
        radius:                   None,
        height:                   None,
        side_padding:             None,
        follow_hyde:              true,
        auto_scale:               false,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        bar_opacity:              0.0,
        menu:                     MenuAppearance {
            opacity:  0.95,
            backdrop: 0.3
        },
        animations:               AnimationConfig::default(),
        greeting:                 true,
        background_color:         AppearanceColor::Simple(HexColor::rgb(36, 39, 58)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(198, 160, 246)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(138, 173, 244)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(166, 218, 149)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(237, 135, 150)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(202, 211, 245)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(138, 173, 244)),
            AppearanceColor::Simple(HexColor::rgb(198, 160, 246)),
            AppearanceColor::Simple(HexColor::rgb(245, 189, 230)),
            AppearanceColor::Simple(HexColor::rgb(245, 169, 127)),
            AppearanceColor::Simple(HexColor::rgb(238, 212, 159)),
            AppearanceColor::Simple(HexColor::rgb(166, 218, 149)),
            AppearanceColor::Simple(HexColor::rgb(139, 213, 202)),
            AppearanceColor::Simple(HexColor::rgb(145, 215, 227)),
            AppearanceColor::Simple(HexColor::rgb(125, 196, 228)),
            AppearanceColor::Simple(HexColor::rgb(183, 189, 248)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            238, 153, 160
        ))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
