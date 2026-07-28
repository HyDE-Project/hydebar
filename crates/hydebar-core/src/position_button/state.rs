use iced::{core::mouse, widget::button::Status};

/// Interaction state the widget tree keeps for a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct State {
    pub(super) is_hovered:        bool,
    pub(super) is_pressed:        bool,
    pub(super) is_right_pressed:  bool,
    pub(super) is_middle_pressed: bool,
    pub(super) is_focused:        bool,
    /// Status the last painted frame was drawn with.
    ///
    /// The runtime only produces a frame when a widget asks for one, so the
    /// button has to compare the status it would paint now against the one the
    /// visible frame carries and request a redraw whenever the two drift apart.
    pub(super) painted:           Option<Status>
}

impl State {
    /// Reports whether any mouse button is held down over the button.
    pub(super) fn is_pressed(&self) -> bool {
        self.is_pressed || self.is_right_pressed || self.is_middle_pressed
    }

    /// Borrows the flag tracking whether `button` is held down.
    pub(super) fn hold_mut(&mut self, button: &mouse::Button) -> &mut bool {
        match button {
            mouse::Button::Right => &mut self.is_right_pressed,
            mouse::Button::Middle => &mut self.is_middle_pressed,
            _ => &mut self.is_pressed
        }
    }

    /// Clears every held mouse button.
    pub(super) fn release_all(&mut self) {
        self.is_pressed = false;
        self.is_right_pressed = false;
        self.is_middle_pressed = false;
    }
}

/// Resolves the status a button paints itself with for the given cursor.
pub(super) fn resolve_status(is_pressable: bool, is_mouse_over: bool, is_pressed: bool) -> Status {
    if !is_pressable {
        Status::Disabled
    } else if is_mouse_over {
        if is_pressed {
            Status::Pressed
        } else {
            Status::Hovered
        }
    } else {
        Status::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_the_status_the_style_is_picked_from() {
        assert_eq!(resolve_status(false, true, false), Status::Disabled);
        assert_eq!(resolve_status(true, false, false), Status::Active);
        assert_eq!(resolve_status(true, true, false), Status::Hovered);
        assert_eq!(resolve_status(true, true, true), Status::Pressed);
    }
}
