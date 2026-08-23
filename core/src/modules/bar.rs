//! One shape every bar entry answers in, so the bar dispatches once.
//!
//! Each module states the data its view wants as an associated type, which is
//! what a module needs and nothing more — and what makes the trait impossible
//! to hold behind a reference. The bar therefore used to name every module in
//! seven places: to draw it, to subscribe to it, to poll it, to hint at it.
//!
//! [`BarContext`] is everything any entry may want, gathered once per frame.
//! A view data shape says how to take itself out of that context through
//! [`FromBarContext`], and a blanket implementation turns every module that
//! does into a [`BarModule`] — the object-safe shape the bar can hold, so the
//! seven lists become one.

use iced::{Element, Subscription, SurfaceId as Id};

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext,
    attention::PollSchedule,
    components::icons::IconTheme,
    config::{Appearance, Config, CustomModuleDef},
    outputs::Outputs
};

/// Everything an entry may want in order to draw itself.
///
/// Gathered once for the entry being drawn rather than per module: the surface
/// and the opacity belong to the section it stands in, the appearance is the
/// one a running transition currently holds, and the custom definition is the
/// module's own only while a custom module is what is being drawn.
#[derive(Debug, Clone, Copy)]
pub struct BarContext<'a> {
    /// The configuration in force.
    pub config:     &'a Config,
    /// The appearance the bar is drawn with this frame.
    pub appearance: &'a Appearance,
    /// The icon theme glyphs are taken from.
    pub icons:      &'a IconTheme,
    /// The screens the bar stands on.
    pub outputs:    &'a Outputs,
    /// The surface the entry is being drawn onto.
    pub surface:    Id,
    /// How opaque the island holding the entry is.
    pub opacity:    f32,
    /// Whether the pointer or an open menu is on this entry.
    pub attended:   bool,
    /// The definition of the custom module being drawn, if one is.
    pub custom:     Option<&'a CustomModuleDef>,
    /// The command the entry being drawn runs, where its kind runs one.
    ///
    /// The launcher and the clipboard are the same module drawn twice, each
    /// configured with a command of its own, so which command is in force is
    /// a property of the entry rather than of the frame.
    pub command:    &'a Option<String>
}

mod shapes;

/// A view data shape that can take itself out of a [`BarContext`].
///
/// Answering [`None`] is how a shape says the context does not hold what it
/// needs — which happens for exactly one shape, a custom module drawn without
/// its definition.
pub trait FromBarContext<'a>: Sized {
    /// Takes this shape out of the context, if the context holds it.
    fn from_bar_context(ctx: &BarContext<'a>) -> Option<Self>;
}

/// The shape the bar holds every entry in.
///
/// Object-safe on purpose: the bar keeps its modules as named fields, and this
/// is what lets one lookup hand any of them back as the same kind of thing.
pub trait BarModule<M> {
    /// Draws the entry, and names what pressing it does.
    fn view(
        &self,
        ctx: &BarContext<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>;

    /// The stream of messages the entry produces on its own, if it has one.
    fn subscription(&self) -> Option<Subscription<M>>;

    /// The cadence the entry wants sampling at, if it is sampled at all.
    fn poll_schedule(&self) -> Option<PollSchedule>;

    /// Takes one sample.
    ///
    /// # Errors
    ///
    /// Returns the module's own failure.
    fn poll(&mut self, ctx: &ModuleContext) -> Result<(), ModuleError>;
}

impl<T, M> BarModule<M> for T
where
    T: Module<M>,
    for<'a> T::ViewData<'a>: FromBarContext<'a>
{
    fn view(
        &self,
        ctx: &BarContext<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        Module::view(self, T::ViewData::from_bar_context(ctx)?)
    }

    fn subscription(&self) -> Option<Subscription<M>> {
        Module::subscription(self)
    }

    fn poll_schedule(&self) -> Option<PollSchedule> {
        Module::poll_schedule(self)
    }

    fn poll(&mut self, ctx: &ModuleContext) -> Result<(), ModuleError> {
        Module::poll(self, ctx)
    }
}
