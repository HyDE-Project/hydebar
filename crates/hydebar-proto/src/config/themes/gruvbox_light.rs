//! The Gruvbox Light preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Gruvbox Light.
pub(super) fn gruvbox_light() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(251, 241, 199)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(157, 0, 6)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(7, 102, 120)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(121, 116, 14)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(204, 36, 29)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(60, 56, 54)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(7, 102, 120)),
            AppearanceColor::Simple(HexColor::rgb(157, 0, 6)),
            AppearanceColor::Simple(HexColor::rgb(143, 63, 113)),
            AppearanceColor::Simple(HexColor::rgb(175, 58, 3)),
            AppearanceColor::Simple(HexColor::rgb(181, 118, 20)),
            AppearanceColor::Simple(HexColor::rgb(121, 116, 14)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(204, 36, 29))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
