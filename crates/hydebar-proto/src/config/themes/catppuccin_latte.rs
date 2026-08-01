//! The Catppuccin Latte preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Catppuccin Latte.
pub(super) fn catppuccin_latte() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(239, 241, 245)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(136, 57, 239)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(30, 102, 245)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(64, 160, 43)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(210, 15, 57)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(76, 79, 105)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(30, 102, 245)),
            AppearanceColor::Simple(HexColor::rgb(136, 57, 239)),
            AppearanceColor::Simple(HexColor::rgb(234, 118, 203)),
            AppearanceColor::Simple(HexColor::rgb(254, 100, 11)),
            AppearanceColor::Simple(HexColor::rgb(223, 142, 29)),
            AppearanceColor::Simple(HexColor::rgb(64, 160, 43)),
            AppearanceColor::Simple(HexColor::rgb(4, 165, 159)),
            AppearanceColor::Simple(HexColor::rgb(23, 146, 153)),
            AppearanceColor::Simple(HexColor::rgb(4, 165, 229)),
            AppearanceColor::Simple(HexColor::rgb(114, 135, 253)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(230, 69, 83))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
