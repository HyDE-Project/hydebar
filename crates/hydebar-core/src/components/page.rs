//! The shapes, sizes and measurements every panel page is built from.
//!
//! A "page" here is the contents of a menu the bar opens: the settings window,
//! the theme picker, anything else that lists rows of labels and controls. They
//! all used to be pages of the settings window and drew from a set of shapes
//! that lived inside it; the theme picker moving out to a bar module of its own
//! is what made that set shared rather than private.
//!
//! Three layers, kept apart on purpose:
//!
//! * [`style`] is the one set of sizes. A page states which shape it wants,
//!   never how large that shape is.
//! * [`metrics`] estimates how wide a row will be before it is laid out, which
//!   is what a menu needs to ask the compositor for a size.
//! * [`widgets`] draws the shapes, reading its sizes from [`style`] so a
//!   measurement and the widget it measures can never drift apart.
//!
//! Everything is generic over the message a page publishes, so two menus with
//! two unrelated message types are still drawn by the same code.

pub(crate) mod metrics;
pub(crate) mod style;
pub(crate) mod widgets;
