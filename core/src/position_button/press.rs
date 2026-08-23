use iced::{
    Point, Rectangle,
    core::{Layout, Shell}
};

/// On-screen reference of a button, handed to the handlers that build their
/// message from the place the button is drawn at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonUIRef {
    /// Where the press landed, in surface coordinates.
    pub position: Point,
    /// How wide and how tall the surface is.
    pub viewport: (f32, f32)
}

impl Eq for ButtonUIRef {}

/// Handler a mouse button carries.
pub(super) enum OnPress<'a, Message> {
    Message(Message),
    MessageWithPosition(Box<dyn Fn(ButtonUIRef) -> Message + 'a>)
}

/// Publishes the message a handler carries, resolving the button position for
/// the handlers that ask for it.
pub(super) fn publish<Message: Clone>(
    on_press: &OnPress<'_, Message>,
    layout: Layout<'_>,
    viewport: &Rectangle,
    shell: &mut Shell<'_, Message>
) {
    match on_press {
        OnPress::Message(message) => {
            shell.publish(message.clone());
        }
        OnPress::MessageWithPosition(on_press) => {
            let ui_data = ButtonUIRef {
                position: Point::new(
                    layout.bounds().width / 2. + layout.position().x,
                    layout.bounds().height / 2. + layout.position().y
                ),
                viewport: (viewport.width, viewport.height)
            };
            shell.publish(on_press(ui_data));
        }
    }
}
