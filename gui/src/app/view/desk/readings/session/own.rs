//! What a module the user wrote themselves has to say.
//!
//! A custom module is a command the configuration names and a line of json it
//! answers with, and the bar draws whichever part of that line fits an island.
//! The canvas has a column, so it writes out the whole answer: the text, the
//! state the command reported itself in, the tooltip it would have shown on a
//! hover, and the reading behind a progress figure.

use super::super::{Panel, push};
use crate::app::state::App;

/// What the custom module named `name` last answered, if it answered at all.
pub fn own(app: &App, name: &str) -> Option<Panel> {
    let module = app.custom.get(name)?;
    let data = module.readings();
    let mut rows = Vec::new();

    push(&mut rows, "reading", data.text.clone());
    push(
        &mut rows,
        "state",
        (!data.alt.is_empty()).then(|| data.alt.clone())
    );
    push(&mut rows, "class", data.class.clone());
    push(
        &mut rows,
        "share",
        data.percentage.map(|share| format!("{share:.0}%"))
    );

    for line in data
        .tooltip
        .iter()
        .flat_map(|tooltip| tooltip.lines())
        .filter(|line| !line.trim().is_empty())
    {
        rows.push((String::new(), line.trim().to_owned()));
    }

    if let Some(failure) = module.failure() {
        rows.push(("failed".to_owned(), failure.to_string()));
    } else if rows.is_empty() && module.is_listening() {
        rows.push(("state".to_owned(), "waiting for a first answer".to_owned()));
    }

    Panel::of(name.to_owned(), rows)
}
