//! The Catppuccin Mocha preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Catppuccin Mocha.
pub(super) fn catppuccin_mocha() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(30, 30, 46)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(203, 166, 247)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(137, 180, 250)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(166, 227, 161)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(243, 139, 168)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(205, 214, 244)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(137, 180, 250)),
            AppearanceColor::Simple(HexColor::rgb(203, 166, 247)),
            AppearanceColor::Simple(HexColor::rgb(245, 194, 231)),
            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
            AppearanceColor::Simple(HexColor::rgb(249, 226, 175)),
            AppearanceColor::Simple(HexColor::rgb(166, 227, 161)),
            AppearanceColor::Simple(HexColor::rgb(148, 226, 213)),
            AppearanceColor::Simple(HexColor::rgb(137, 220, 235)),
            AppearanceColor::Simple(HexColor::rgb(116, 199, 236)),
            AppearanceColor::Simple(HexColor::rgb(180, 190, 254)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            235, 160, 172
        ))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
