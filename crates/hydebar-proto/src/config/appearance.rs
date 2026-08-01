//! Appearance configuration of the bar, split by concern.
//!
//! The palette lives in [`color`], the layout metrics derived from the themed
//! font size in [`metrics`], the overlay of the `HyDE` Project theme in [`hyde`]
//! and the configuration struct tying them together in [`settings`].

mod animation;
mod color;
mod compositor;
mod hyde;
mod menu;
mod metrics;
mod settings;
mod style;

pub use animation::AnimationConfig;
pub use color::AppearanceColor;
pub use menu::MenuAppearance;
pub use metrics::{
    BAR_PADDING_EM, DEFAULT_FONT_SIZE, DEFAULT_RADIUS, GROUP_GAP_EM, ICON_LABEL_GAP_EM,
    MODULE_GAP_EM, MODULE_SIDE_PADDING_EM, MODULE_VERTICAL_PADDING_EM, WORKSPACE_ACTIVE_MARGIN_EM,
    WORKSPACE_ACTIVE_PADDING_EM, WORKSPACE_GAP_EM, WORKSPACE_GLYPH_ADVANCE_EM,
    WORKSPACE_MIN_HEIGHT_EM, WORKSPACE_MIN_WIDTH_EM, WORKSPACE_PADDING_EM
};
pub use settings::{Appearance, WindowBorder, WindowShadow};
pub use style::AppearanceStyle;
