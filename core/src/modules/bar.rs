//! One shape every bar entry answers in, so the bar dispatches once.
//!
//! A module states what starting its work needs as an associated type, which
//! is what that module needs and nothing more — and what makes the trait
//! impossible to hold behind a reference. The bar therefore used to name
//! every module in a list of its own for each question it asks: to subscribe
//! to it, to schedule it, to sample it.
//!
//! This is the object-safe half, which asks none of that: a blanket
//! implementation turns every module into a [`BarModule`], the shape the bar
//! can hold, and the lists become one lookup.

use iced::Subscription;

use super::{Module, ModuleError};
use crate::{ModuleContext, attention::PollSchedule};

/// The shape the bar holds every entry in.
///
/// Object-safe on purpose: the bar keeps its modules as named fields, and this
/// is what lets one lookup hand any of them back as the same kind of thing.
pub trait BarModule<M> {
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
    T: Module<M>
{
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
