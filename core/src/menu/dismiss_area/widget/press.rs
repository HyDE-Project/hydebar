//! The press the area answers to, and the release that completes it.
//!
//! All the area's own work. A press is reported before the children see it,
//! so a child consuming it cannot hide it from the menu standing over them;
//! the completion is reported only once the same press releases over the
//! area, and a pointer that left takes the press with it.

use iced::core::{Layout, Shell, event::Event, mouse, touch};

use super::super::element::{DismissArea, State};

impl<'a, Message, Theme, Renderer> DismissArea<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: 'a + iced_core::Renderer
{
    /// Reports a press over the area, before its children are given the event.
    pub(super) fn note_the_press(
        &self,
        state: &mut State,
        event: &Event,
        is_over: bool,
        shell: &mut Shell<'_, Message>
    ) {
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(_))
                | Event::Touch(touch::Event::FingerPressed { .. })
        ) && is_over
        {
            state.pressed = true;
            shell.publish(self.on_press.clone());
        }
    }

    /// Reports the release that completes a press the area began.
    pub(super) fn note_the_release(
        &self,
        state: &mut State,
        event: &Event,
        is_over: bool,
        shell: &mut Shell<'_, Message>
    ) {
        match event {
            Event::Mouse(mouse::Event::ButtonReleased(_))
            | Event::Touch(touch::Event::FingerLifted {
                ..
            }) => {
                if std::mem::take(&mut state.pressed) && is_over {
                    shell.publish(self.on_release.clone());
                }
            }
            Event::Mouse(mouse::Event::CursorLeft)
            | Event::Touch(touch::Event::FingerLost {
                ..
            }) => {
                state.pressed = false;
            }
            _ => {}
        }
    }

    /// Reports whether `layout` still has the pointer over it.
    pub(super) fn is_over(layout: Layout<'_>, cursor: mouse::Cursor) -> bool {
        cursor.is_over(layout.bounds())
    }
}
