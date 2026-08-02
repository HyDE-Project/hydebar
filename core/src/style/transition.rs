//! Cross-fade between two appearance snapshots.
//!
//! The transition state — the two snapshots, the spring between them and the
//! appearance shown this frame — lives in [`state`]; the arithmetic that
//! mixes two appearances lives in [`blend`].

mod blend;
mod state;

pub use state::AppearanceTransition;
