//! What each block of the canvas says, as plain text pairs.
//!
//! The canvas is a table of labels and values, so the reading is settled here
//! — before a single widget exists — and the drawing beside it only has to
//! place what this file already decided.
//!
//! Two rooms: [`machine`] reads the hardware off one system sample,
//! [`session`] reads the session the bar is running in.

mod machine;
mod session;

pub(super) use machine::{cpu_temperature, graphics, memory, network, processor, storage, system};
pub(super) use session::{
    battery, keyboard, link, notifications, own, playing, privacy, radio, screen, session_idle,
    sound, submap, theme, tray, updates, weather, windows, workspaces
};

/// One block of the canvas: a heading and the lines under it.
///
/// The heading is borrowed where the crate wrote it down and owned where the
/// user did: every block but one is named here, and a module the user wrote
/// carries the name their configuration gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    /// Heading, drawn in the small capitals the columns are ruled by.
    pub title: std::borrow::Cow<'static, str>,
    /// Label and value of every line, in drawing order.
    pub rows:  Vec<(String, String)>
}

impl Panel {
    /// A panel with rows, or nothing at all when the source reported none.
    fn of(
        title: impl Into<std::borrow::Cow<'static, str>>,
        rows: Vec<(String, String)>
    ) -> Option<Self> {
        (!rows.is_empty()).then_some(Self {
            title: title.into(),
            rows
        })
    }
}

/// Pushes a row when the source answered for it.
fn push(rows: &mut Vec<(String, String)>, label: &str, value: Option<String>) {
    if let Some(value) = value {
        rows.push((label.to_owned(), value));
    }
}
