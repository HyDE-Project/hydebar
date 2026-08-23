//! How each view data shape takes itself out of the bar context.
//!
//! One implementation per shape rather than per module: eight entries want
//! nothing but the icon theme, three want nothing at all, and the rest want a
//! section of the configuration each. A module that reuses a shape another
//! already stated costs nothing here.

use iced::SurfaceId as Id;

use super::{BarContext, FromBarContext};
use crate::{
    components::icons::IconTheme,
    config::{
        Appearance, ClockModuleConfig, CustomModuleDef, KeyboardLayoutModuleConfig,
        MediaPlayerModuleConfig, SystemModuleConfig, UpdatesModuleConfig, WindowTitleConfig,
        WorkspacesModuleConfig
    },
    outputs::Outputs
};

impl FromBarContext<'_> for () {
    fn from_bar_context(_: &BarContext<'_>) -> Option<Self> {
        Some(())
    }
}

impl FromBarContext<'_> for f32 {
    fn from_bar_context(ctx: &BarContext<'_>) -> Option<Self> {
        Some(ctx.opacity)
    }
}

impl FromBarContext<'_> for (Id, f32) {
    fn from_bar_context(ctx: &BarContext<'_>) -> Option<Self> {
        Some((ctx.surface, ctx.opacity))
    }
}

impl<'a> FromBarContext<'a> for &'a IconTheme {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some(ctx.icons)
    }
}

impl<'a> FromBarContext<'a> for &'a ClockModuleConfig {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some(&ctx.config.clock)
    }
}

impl<'a> FromBarContext<'a> for &'a KeyboardLayoutModuleConfig {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some(&ctx.config.keyboard_layout)
    }
}

impl<'a> FromBarContext<'a> for (&'a WindowTitleConfig, bool) {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some((&ctx.config.window_title, ctx.attended))
    }
}

impl<'a> FromBarContext<'a> for (&'a Option<String>, &'a IconTheme) {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some((ctx.command, ctx.icons))
    }
}

impl<'a> FromBarContext<'a> for (&'a Option<UpdatesModuleConfig>, &'a IconTheme) {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some((&ctx.config.updates, ctx.icons))
    }
}

impl<'a> FromBarContext<'a> for (&'a MediaPlayerModuleConfig, &'a IconTheme) {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some((&ctx.config.media_player, ctx.icons))
    }
}

impl<'a> FromBarContext<'a> for (&'a SystemModuleConfig, &'a Appearance, &'a IconTheme) {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some((&ctx.config.system, ctx.appearance, ctx.icons))
    }
}

impl<'a> FromBarContext<'a> for (&'a CustomModuleDef, &'a Appearance, &'a IconTheme) {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some((ctx.custom?, ctx.appearance, ctx.icons))
    }
}

impl<'a> FromBarContext<'a> for (&'a Outputs, Id, &'a WorkspacesModuleConfig, &'a Appearance) {
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self> {
        Some((
            ctx.outputs,
            ctx.surface,
            &ctx.config.workspaces,
            ctx.appearance
        ))
    }
}
