//! The Tokyo Night Light preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Tokyo Night Light.
pub(super) fn tokyo_night_light() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(213, 214, 219)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(121, 94, 172)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(52, 108, 197)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(51, 153, 51)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(185, 29, 71)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(60, 62, 73)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(52, 108, 197)),
            AppearanceColor::Simple(HexColor::rgb(121, 94, 172)),
            AppearanceColor::Simple(HexColor::rgb(185, 29, 71)),
            AppearanceColor::Simple(HexColor::rgb(166, 88, 24)),
            AppearanceColor::Simple(HexColor::rgb(143, 94, 21)),
            AppearanceColor::Simple(HexColor::rgb(51, 153, 51)),
            AppearanceColor::Simple(HexColor::rgb(15, 155, 142)),
            AppearanceColor::Simple(HexColor::rgb(29, 130, 183)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(185, 29, 71))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
