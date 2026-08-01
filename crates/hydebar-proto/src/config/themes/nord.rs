//! The Nord preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Nord.
pub(super) fn nord() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(46, 52, 64)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(136, 192, 208)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(129, 161, 193)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(163, 190, 140)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(191, 97, 106)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(236, 239, 244)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(129, 161, 193)),
            AppearanceColor::Simple(HexColor::rgb(136, 192, 208)),
            AppearanceColor::Simple(HexColor::rgb(143, 188, 187)),
            AppearanceColor::Simple(HexColor::rgb(163, 190, 140)),
            AppearanceColor::Simple(HexColor::rgb(235, 203, 139)),
            AppearanceColor::Simple(HexColor::rgb(208, 135, 112)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(191, 97, 106))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
