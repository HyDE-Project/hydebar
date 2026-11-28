//! The module vocabulary of the bar, split by concern.
//!
//! The names the modules answer to live in [`name`] — variants, roster and
//! both spellings grown from one declarative list — the configuration reader
//! for them in [`de`], the three position arrays in [`layout`], the bar
//! placement in [`position`] and the output targeting in [`outputs`].

mod de;
mod layout;
mod name;
mod outputs;
mod position;

pub use layout::{ModuleDef, Modules};
pub use name::ModuleName;
pub use outputs::Outputs;
pub use position::{BarLayer, Position};
