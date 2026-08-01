//! The Gruvbox Dark preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Gruvbox Dark.
pub(super) fn gruvbox_dark() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(40, 40, 40)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(211, 134, 155)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(131, 165, 152)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(184, 187, 38)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(251, 73, 52)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(235, 219, 178)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(131, 165, 152)),
            AppearanceColor::Simple(HexColor::rgb(211, 134, 155)),
            AppearanceColor::Simple(HexColor::rgb(177, 98, 134)),
            AppearanceColor::Simple(HexColor::rgb(254, 128, 25)),
            AppearanceColor::Simple(HexColor::rgb(250, 189, 47)),
            AppearanceColor::Simple(HexColor::rgb(184, 187, 38)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(251, 73, 52))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
