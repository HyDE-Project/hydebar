/// Core module declarations - Business logic only, no GUI!
use std::borrow::Cow;

use masterror::AppError;

use crate::{attention::PollSchedule, menu::MenuType};

pub mod bar;
/// Bar entry stepping through the layouts the desktop ships.
pub mod bar_layout;
pub mod battery;
/// The calendar the clock opens.
pub mod calendar;
pub mod clock;
/// A bar entry whose whole behaviour is running one command.
pub mod command_button;
/// Bar entry gathering the quick settings, and the services behind them.
pub mod control_center;
pub mod cpu;
pub mod cpu_temp;
pub mod custom_module;
pub mod desk;
pub mod gpu_temp;
/// The desktop's own buttons, drawn by a plain function.
pub mod hyde_button;
pub mod hyde_menu;
pub mod idle_inhibitor;
pub mod keyboard_layout;
/// Bar entry naming the compositor submap the keyboard is in.
pub mod keyboard_submap;
pub mod media_player;
pub mod memory;
pub mod notifications;
pub mod privacy;
pub mod screenshot;
/// Bar entry opening the window the bar is configured from.
pub mod settings;
/// Bar entry reading the machine, and the sample every readout shares.
pub mod system_info;
pub mod taskbar;
pub mod themes;
pub mod tray;
/// Bar entry counting what is waiting to be installed.
pub mod updates;
pub mod wallpaper;
pub mod weather;
pub mod window_title;
pub mod workspaces;

/// Action to perform when a module is pressed
#[derive(Debug, Clone)]
pub enum OnModulePress<M> {
    /// Send this message.
    Action(Box<M>),
    /// Open or close this menu.
    ToggleMenu(MenuType)
}

/// Module registration and operation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    /// The module could not start the work it owns.
    Registration {
        /// What went wrong, in the module's own words.
        reason: Cow<'static, str>
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registration {
                reason
            } => write!(f, "Module registration failed: {reason}")
        }
    }
}

impl std::error::Error for ModuleError {}

impl From<ModuleError> for AppError {
    fn from(err: ModuleError) -> Self {
        match err {
            ModuleError::Registration {
                ..
            } => Self::validation(err.to_string())
        }
    }
}

impl ModuleError {
    /// A module that could not start its work, and why.
    #[must_use]
    pub fn registration(reason: impl Into<Cow<'static, str>>) -> Self {
        Self::Registration {
            reason: reason.into()
        }
    }
}

/// The background work a bar module owns, and how the bar drives it.
///
/// Drawing is not here: a module renders through a method of its own that the
/// bar's dispatch calls with the data it holds. What is left is what only the
/// bar can decide — when to start the work, when to give it back, and how
/// often to take a sample.
pub trait Module<Message> {
    /// What the module needs in order to start its work.
    type RegistrationData<'a>;

    /// Starts the module's background work, if it owns any.
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleError`] when the module fails to start its listeners
    /// or background tasks; the default implementation never fails.
    fn register(
        &mut self,
        ctx: &crate::module_context::ModuleContext,
        data: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        let _ = (ctx, data);
        Ok(())
    }

    /// Releases the background work [`register`] started.
    ///
    /// The bar calls this instead of `register` for every module the layout
    /// stopped drawing. Without it a module dropped from the configuration on a
    /// hot reload would keep its poller and its listeners alive for the rest of
    /// the session, burning wakeups for a readout nobody can see.
    ///
    /// The default is a no-op, which is correct for the modules that own no
    /// task at all.
    ///
    /// [`register`]: Module::register
    fn deregister(&mut self) {}

    /// The two cadences the module is willing to be sampled at.
    ///
    /// A module that declares a schedule owns no timer: the bar keeps one clock
    /// for every module at rest and one for the module being attended, and
    /// calls [`poll`] when either of them comes due. The default is [`None`],
    /// which is right for a module fed by a compositor or bus listener rather
    /// than by a readout somebody has to go and take.
    ///
    /// [`poll`]: Module::poll
    fn poll_schedule(&self) -> Option<PollSchedule> {
        None
    }

    /// Takes one sample now.
    ///
    /// Called from the clock the module's [`poll_schedule`] put it on, never
    /// more often than the cadence it declared. A module that publishes a
    /// reading identical to the one already on screen should drop it instead:
    /// every event reaching the bar rebuilds every surface it draws.
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleError`] when taking or publishing the sample fails;
    /// the default implementation never fails.
    ///
    /// [`poll_schedule`]: Module::poll_schedule
    fn poll(&mut self, ctx: &crate::module_context::ModuleContext) -> Result<(), ModuleError> {
        let _ = ctx;
        Ok(())
    }

    /// The stream the module produces on its own, if it produces one.
    ///
    /// The default produces none: nearly every module publishes through the
    /// event bus instead, which costs no wakeup while nothing happens.
    fn subscription(&self) -> Option<iced::Subscription<Message>> {
        None
    }
}
