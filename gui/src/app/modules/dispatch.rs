//! Per module dispatch of the view and subscription of a bar module.
//!
//! The view stays a table: half the entries are drawn by a plain function and
//! own no state, which is where every module is headed. Everything asked of a
//! module that does own state — its subscription, its cadence, its samples —
//! goes through the one owner lookup in [`owner`].

mod owner;
mod poll;
mod subscription;
mod view;
