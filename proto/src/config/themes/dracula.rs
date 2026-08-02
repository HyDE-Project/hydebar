//! The Dracula preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Dracula.
pub(super) fn dracula() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(40, 42, 54)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(189, 147, 249)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(139, 233, 253)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(80, 250, 123)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(255, 85, 85)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(248, 248, 242)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(139, 233, 253)),
            AppearanceColor::Simple(HexColor::rgb(189, 147, 249)),
            AppearanceColor::Simple(HexColor::rgb(255, 121, 198)),
            AppearanceColor::Simple(HexColor::rgb(255, 184, 108)),
            AppearanceColor::Simple(HexColor::rgb(241, 250, 140)),
            AppearanceColor::Simple(HexColor::rgb(80, 250, 123)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(255, 85, 85))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
