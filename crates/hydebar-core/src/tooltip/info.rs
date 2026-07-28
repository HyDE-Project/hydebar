//! The request a module publishes when the pointer rests on it.

use crate::position_button::ButtonUIRef;

/// Tooltip a module asks the tooltip surface to show.
#[derive(Debug, Clone, PartialEq)]
pub struct TooltipInfo {
    /// Text to render.
    pub text:   String,
    /// On-screen placement of the module the tooltip belongs to.
    pub anchor: ButtonUIRef
}
