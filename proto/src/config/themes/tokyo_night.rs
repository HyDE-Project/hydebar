//! The Tokyo Night preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Tokyo Night.
pub(super) fn tokyo_night() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(26, 27, 38)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(187, 154, 247)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(122, 162, 247)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(158, 206, 106)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(247, 118, 142)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(192, 202, 245)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(122, 162, 247)),
            AppearanceColor::Simple(HexColor::rgb(187, 154, 247)),
            AppearanceColor::Simple(HexColor::rgb(255, 117, 127)),
            AppearanceColor::Simple(HexColor::rgb(255, 158, 100)),
            AppearanceColor::Simple(HexColor::rgb(224, 175, 104)),
            AppearanceColor::Simple(HexColor::rgb(158, 206, 106)),
            AppearanceColor::Simple(HexColor::rgb(115, 218, 202)),
            AppearanceColor::Simple(HexColor::rgb(125, 207, 255)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            247, 118, 142
        ))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
