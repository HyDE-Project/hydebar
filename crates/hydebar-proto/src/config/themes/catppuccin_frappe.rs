//! The Catppuccin Frappe preset palette.

use hex_color::HexColor;

use crate::config::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance
};

/// The bar dressed in Catppuccin Frappe.
pub(super) fn catppuccin_frappe() -> Appearance {
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
        background_color:         AppearanceColor::Simple(HexColor::rgb(48, 52, 70)),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(202, 158, 230)),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(140, 170, 238)),
        success_color:            AppearanceColor::Simple(HexColor::rgb(166, 209, 137)),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(231, 130, 132)),
        warning_color:            AppearanceColor::Simple(HexColor::rgb(250, 179, 135)),
        text_color:               AppearanceColor::Simple(HexColor::rgb(198, 208, 245)),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(140, 170, 238)),
            AppearanceColor::Simple(HexColor::rgb(202, 158, 230)),
            AppearanceColor::Simple(HexColor::rgb(244, 184, 228)),
            AppearanceColor::Simple(HexColor::rgb(239, 159, 118)),
            AppearanceColor::Simple(HexColor::rgb(229, 200, 144)),
            AppearanceColor::Simple(HexColor::rgb(166, 209, 137)),
            AppearanceColor::Simple(HexColor::rgb(129, 200, 190)),
            AppearanceColor::Simple(HexColor::rgb(153, 209, 219)),
            AppearanceColor::Simple(HexColor::rgb(133, 193, 220)),
            AppearanceColor::Simple(HexColor::rgb(186, 187, 241)),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            234, 153, 156
        ))]),
        island_borders:           false,
        window_border:            None,
        window_shadow:            None
    }
}
