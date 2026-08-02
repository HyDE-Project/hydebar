//! Menu visibility and fade animation state.
//!
//! The read-only queries and the per-frame tick live in [`visibility`], the
//! open and close walks in [`toggle`], and the press-armed dismissal in
//! [`dismiss`].

mod dismiss;
mod toggle;
mod visibility;
