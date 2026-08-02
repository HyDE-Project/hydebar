//! Card naming the picked module and offering its actions.

use iced::Element;

use super::island_of;
use crate::{
    components::page::widgets::{
        card, choice_button, group, labelled_row, note, rows as row_stack
    },
    modules::settings::{
        Message,
        layout::{Entry, LayoutEdit, Section, Slot}
    }
};

/// Label of the action moving a module one place earlier.
const MOVE_EARLIER: &str = "\u{2190} move left";
/// Label of the action moving a module one place later.
const MOVE_LATER: &str = "move right \u{2192}";
/// Label of the action moving a module to the section on the left.
const TO_LEFT: &str = "to Left";
/// Label of the action moving a module to the section on the right.
const TO_RIGHT: &str = "to Right";
/// Label of the action joining a module to the island beside it.
const MERGE: &str = "merge with the left";
/// Label of the action breaking a module out of its island.
const BREAK_OUT: &str = "break out";
/// Label of the action taking a module off the bar.
const REMOVE: &str = "take off the bar";

/// Label of the row the reordering actions sit on.
pub(super) const ORDER: &str = "Order";

/// Label of the row the moving and removing actions sit on.
pub(super) const MOVE_IT: &str = "Move it";

/// Labels of the actions one card row can offer, so the width is
/// measured against the very strings that are drawn.
pub(super) const ACTION_LABELS: [&str; 4] = [TO_LEFT, TO_RIGHT, MERGE, REMOVE];

/// Renders the card describing the picked module and its actions.
///
/// The card is built from the same labelled rows as every other page,
/// so a module's actions line up with the steppers on the
/// appearance tab rather than forming a grid of their own.
pub(super) fn detail<'a>(
    slot: Slot,
    entries: &[Entry],
    font_size: f32,
    opacity: f32
) -> Element<'a, Message> {
    let Some(entry) = entries.get(slot.index) else {
        return note("that module is gone", font_size);
    };

    let button = |label: &'static str, edit: LayoutEdit| {
        choice_button(label, Message::EditLayout(edit), false, font_size, opacity)
    };

    let heading = labelled_row(
        entry.module.as_str().to_owned(),
        note(
            format!(
                "{} section · island {} · position {} of {}",
                section_name(slot.section),
                island_of(entries, slot.index),
                slot.index + 1,
                entries.len()
            ),
            font_size
        ),
        font_size
    );

    let mut order = group(font_size);

    if slot.index > 0 {
        order = order.push(button(MOVE_EARLIER, LayoutEdit::MoveEarlier(slot)));
    }

    if slot.index + 1 < entries.len() {
        order = order.push(button(MOVE_LATER, LayoutEdit::MoveLater(slot)));
    }

    if slot.index == 0 && entries.len() == 1 {
        order = order.push(note("alone in this section", font_size));
    }

    let mut actions = group(font_size);

    if let Some(before) = slot.section.before() {
        actions = actions.push(button(
            section_button_label(before),
            LayoutEdit::MoveToPreviousSection(slot)
        ));
    }

    if let Some(after) = slot.section.after() {
        actions = actions.push(button(
            section_button_label(after),
            LayoutEdit::MoveToNextSection(slot)
        ));
    }

    actions = if slot.index == 0 {
        actions.push(note("first in the island", font_size))
    } else if entry.joined {
        actions.push(button(BREAK_OUT, LayoutEdit::ToggleJoin(slot)))
    } else {
        actions.push(button(MERGE, LayoutEdit::ToggleJoin(slot)))
    };

    actions = actions.push(button(REMOVE, LayoutEdit::Remove(slot)));

    card(
        row_stack(font_size)
            .push(heading)
            .push(labelled_row(ORDER, order.into(), font_size))
            .push(labelled_row(MOVE_IT, actions.into(), font_size))
            .into(),
        font_size,
        opacity
    )
}

/// Label of the button moving a module into `section`.
const fn section_button_label(section: Section) -> &'static str {
    match section {
        Section::Left => TO_LEFT,
        Section::Center => "to Center",
        Section::Right => TO_RIGHT
    }
}

/// Name of a section as the card spells it.
const fn section_name(section: Section) -> &'static str {
    match section {
        Section::Left => "Left",
        Section::Center => "Center",
        Section::Right => "Right"
    }
}
