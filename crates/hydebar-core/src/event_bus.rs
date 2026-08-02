//! The queue every module event rides on its way to the UI.
//!
//! The events themselves, the coalescing rules and the snapshot replacement
//! live in [`queue`]; the bus handle, its sender and its receiver in
//! [`endpoints`].

mod endpoints;
mod queue;

pub use endpoints::{EventBus, EventReceiver, EventSender};
pub use queue::{BusEvent, ModuleEvent};
