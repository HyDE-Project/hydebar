//! The module vocabulary of the bar, split by concern.
//!
//! The names the modules answer to live in [`name`], their two spellings in
//! [`spelling`], the configuration reader for them in [`de`], the three
//! position arrays in [`layout`], the bar placement in [`position`] and the
//! output targeting in [`outputs`].

mod de;
mod layout;
mod name;
mod outputs;
mod position;
mod spelling;

pub use layout::{ModuleDef, Modules};
pub use name::ModuleName;
pub use outputs::Outputs;
pub use position::{BarLayer, Position};
