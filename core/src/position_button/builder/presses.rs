//! What the button is to answer to, mouse button by mouse button.
//!
//! Each of the three is set on its own and dispatched on its own, so a button
//! may carry any of them, all of them or none: a module that answers a right
//! press and nothing else is an ordinary thing on a bar.

use iced::{core::mouse, widget::button::Catalog};

use super::{
    super::press::{ButtonUIRef, OnPress},
    PositionButton
};

impl<'a, Message, Theme, Renderer> PositionButton<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
    Theme: Catalog
{
    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// Unless `on_press` is called, the [`Button`] will be disabled.
    #[must_use]
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(OnPress::Message(on_press));
        self
    }

    /// Answers a press with a message built from where the press landed.
    #[must_use]
    pub fn on_press_with_position(
        mut self,
        on_press: impl Fn(ButtonUIRef) -> Message + 'a
    ) -> Self {
        self.on_press = Some(OnPress::MessageWithPosition(Box::new(on_press)));
        self
    }

    /// Sets the message produced when the [`Button`] is pressed with the right
    /// mouse button.
    ///
    /// Right presses are dispatched independently of the left ones, so a button
    /// may carry either, both or neither handler.
    #[must_use]
    pub fn on_right_press(mut self, on_press: Message) -> Self {
        self.on_right_press = Some(OnPress::Message(on_press));
        self
    }

    /// Sets the message produced when the [`Button`] is pressed with the right
    /// mouse button, built from the on-screen position of the button.
    #[must_use]
    pub fn on_right_press_with_position(
        mut self,
        on_press: impl Fn(ButtonUIRef) -> Message + 'a
    ) -> Self {
        self.on_right_press = Some(OnPress::MessageWithPosition(Box::new(on_press)));
        self
    }

    /// Sets the message produced when the [`Button`] is pressed with the middle
    /// mouse button.
    ///
    /// Middle presses are dispatched independently of the left ones, so a
    /// button may carry either, both or neither handler.
    #[must_use]
    pub fn on_middle_press(mut self, on_press: Message) -> Self {
        self.on_middle_press = Some(OnPress::Message(on_press));
        self
    }

    /// Sets the message produced when the [`Button`] is pressed with the middle
    /// mouse button, built from the on-screen position of the button.
    #[must_use]
    pub fn on_middle_press_with_position(
        mut self,
        on_press: impl Fn(ButtonUIRef) -> Message + 'a
    ) -> Self {
        self.on_middle_press = Some(OnPress::MessageWithPosition(Box::new(on_press)));
        self
    }

    /// Reports whether any mouse button carries a handler.
    pub(in crate::position_button) const fn is_pressable(&self) -> bool {
        self.on_press.is_some() || self.on_right_press.is_some() || self.on_middle_press.is_some()
    }

    /// Borrows the handler the given mouse button carries, if any.
    pub(in crate::position_button) const fn handler(
        &self,
        button: mouse::Button
    ) -> Option<&OnPress<'a, Message>> {
        match button {
            mouse::Button::Right => self.on_right_press.as_ref(),
            mouse::Button::Middle => self.on_middle_press.as_ref(),
            _ => self.on_press.as_ref()
        }
    }
}
