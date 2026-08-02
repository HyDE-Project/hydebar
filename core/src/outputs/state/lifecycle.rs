//! Creation and removal of per output surfaces.
//!
//! A monitor arriving is handled in [`add`], a monitor departing — and the
//! full teardown on the way out — in [`remove`].

mod add;
mod remove;
