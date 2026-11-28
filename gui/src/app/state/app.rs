//! The application state every surface renders from.

use std::sync::Arc;

use hydebar_core::{ModuleContext, components::icons::IconTheme, outputs::Outputs};
use hydebar_proto::config::Config;

use super::{entries::Modules, focus::Focus, motion::Motion, runtime::Runtime, screen::Screen};

/// Everything the bar is, at this moment, on every screen it stands on.
pub struct App {
    /// The machinery the binary booted the bar with.
    pub(crate) runtime:           Runtime,
    /// The clocks the bar is currently riding.
    pub(crate) motion:            Motion,
    /// What the bar is paying attention to, and what is unfolded around it.
    pub(crate) focus:             Focus,
    /// Theme built from the appearance in force, rebuilt only when it moves.
    ///
    /// Building a theme runs the whole palette derivation; the renderer asks
    /// for the theme on every loop turn, and an idle bar must answer with a
    /// reference-count bump, not five colour cascades.
    pub(crate) theme_cache:       iced::Theme,
    pub(crate) module_context:    ModuleContext,
    pub(crate) icons:             IconTheme,
    /// What the screens have reported about themselves.
    pub(crate) screen:            Screen,
    /// Every module the bar carries.
    pub(crate) modules:           Modules,
    /// What the machine has been doing for the last few minutes.
    ///
    /// One reading is a number and a run of them is a shape; the canvas draws
    /// the shape beside the number, and nothing else in the bar keeps one.
    pub(crate) history:           super::history::History,
    /// The wallpaper in force, decoded ready to draw, and where it was read.
    ///
    /// Held rather than read where it is drawn: the canvas is built on every
    /// frame of an unfolding, and decoding a picture per frame is a picture
    /// decoded a hundred times for one that was wanted once. The path beside
    /// it is what tells one wallpaper from the next without decoding again.
    pub(crate) wallpaper_preview: Option<(std::path::PathBuf, iced::widget::image::Handle)>,
    /// How the desktop looks, as the pictures it keeps of itself.
    ///
    /// The themes installed here and the wallpapers of the theme in force,
    /// each already centred on the one in force and each already decoded.
    /// Held for the same reason the wallpaper is: the canvas is rebuilt on
    /// every frame of an unfolding and reads no files while it draws.
    pub(crate) looks:             hydebar_core::modules::desk::looks::Looks,
    /// The configuration in force.
    pub config:                   Arc<Config>,
    /// Whether the desktop asked to reduce motion, mirrored from the portal.
    ///
    /// Held beside the configuration rather than folded into it: the setting is
    /// the desktop's, not the user's file, and a configuration reload must not
    /// lose what the portal last said. The bar draws its motion through
    /// [`App::animations_enabled`], which respects both.
    pub(crate) reduced_motion:    bool,
    /// The screens the bar stands on, and the surfaces it holds on them.
    pub outputs:                  Outputs,
    /// The last configuration as the file spelled it, before adoption.
    ///
    /// The cheap gate against reload bursts: a reload whose raw text matches
    /// is finished before the adoption clone and the compositor questions it
    /// asks.
    pub(crate) raw_config:        Option<Arc<Config>>,
    /// Faded and swept themes derived this frame, by quantised key.
    ///
    /// One palette blend serves every island and menu that lands on the
    /// same sixty-fourth of the fade; cleared each frame, so the map holds
    /// a handful of entries and never staleness.
    pub(crate) derived_themes:
        std::cell::RefCell<std::collections::HashMap<(u32, u32), iced::Theme>>
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("config_path", &self.runtime.config_path)
            .field("navigation_mode", &self.focus.navigation_mode)
            .field("focused_module_index", &self.focus.focused_module_index)
            .field("magnification", &self.screen.magnification)
            .field("screen_height", &self.screen.height)
            .finish_non_exhaustive()
    }
}
