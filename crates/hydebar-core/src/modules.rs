/// Core module declarations - Business logic only, no GUI!
use std::borrow::Cow;

use masterror::AppError;

use crate::{attention::PollSchedule, menu::MenuType};

pub mod app_launcher;
pub mod battery;
pub mod calendar;
pub mod clipboard;
pub mod clock;
pub mod control_center;
pub mod cpu;
pub mod cpu_temp;
pub mod custom_module;
pub mod gpu_temp;
pub mod hyde_menu;
pub mod idle_inhibitor;
pub mod keyboard_layout;
pub mod keyboard_submap;
pub mod media_player;
pub mod memory;
pub mod notifications;
pub mod privacy;
pub mod screenshot;
pub mod settings;
pub mod system_info;
pub mod themes;
pub mod tray;
pub mod updates;
pub mod wallpaper;
pub mod weather;
pub mod window_title;
pub mod workspaces;

/// Action to perform when a module is pressed
#[derive(Debug, Clone)]
pub enum OnModulePress<M> {
    Action(Box<M>),
    ToggleMenu(MenuType)
}

/// Module registration and operation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    Registration { reason: Cow<'static, str> }
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
    pub fn registration(reason: impl Into<Cow<'static, str>>) -> Self {
        Self::Registration {
            reason: reason.into()
        }
    }
}

/// Behaviour shared by all UI modules rendered inside the bar.
///
/// NOTE: This trait is being phased out in favor of clean architecture.
/// New modules should follow the Battery pattern: separate data/logic (core)
/// from rendering (gui).
pub trait Module<Message> {
    type ViewData<'a>;
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

    fn view(
        &self,
        data: Self::ViewData<'_>
    ) -> Option<(
        iced::Element<'static, Message>,
        Option<OnModulePress<Message>>
    )> {
        let _ = data;
        None
    }

    fn subscription(&self) -> Option<iced::Subscription<Message>> {
        None
    }
}
