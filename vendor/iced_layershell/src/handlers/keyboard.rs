//! Keyboard handler for key input events.
//!
//! Processes key press, release, and modifier events, converting
//! them to iced keyboard events.

use smithay_client_toolkit::delegate_keyboard;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, KeyboardHandler, Modifiers};
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, QueueHandle};

use crate::conversion;
use crate::state::WaylandState;

/// Create a key pressed event for iced.
pub(crate) fn make_key_pressed(
    event: &KeyEvent,
    modifiers: iced_core::keyboard::Modifiers,
    repeat: bool,
) -> iced_core::Event {
    let key = crate::conversion::keysym_to_iced_key(event.keysym);
    iced_core::Event::Keyboard(iced_core::keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: crate::conversion::scancode_to_physical(event.raw_code),
        location: iced_core::keyboard::Location::Standard,
        modifiers,
        text: crate::conversion::key_utf8_to_text(event.utf8.as_deref()),
        repeat,
    })
}

/// Create a key released event for iced.
pub(crate) fn make_key_released(
    event: &KeyEvent,
    modifiers: iced_core::keyboard::Modifiers,
) -> iced_core::Event {
    let key = crate::conversion::keysym_to_iced_key(event.keysym);
    iced_core::Event::Keyboard(iced_core::keyboard::Event::KeyReleased {
        key: key.clone(),
        modified_key: key,
        physical_key: crate::conversion::scancode_to_physical(event.raw_code),
        location: iced_core::keyboard::Location::Standard,
        modifiers,
    })
}

impl KeyboardHandler for WaylandState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
        self.keyboard_focus = self.surface_id_for(surface);
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
    ) {
        self.keyboard_focus = None;
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(surface_id) = self.keyboard_focus {
            self.pending_events
                .push((surface_id, make_key_pressed(&event, self.modifiers, false)));
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if let Some(surface_id) = self.keyboard_focus {
            self.pending_events
                .push((surface_id, make_key_released(&event, self.modifiers)));
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _layout: u32,
    ) {
        self.modifiers = conversion::modifiers_to_iced(modifiers);
        if let Some(surface_id) = self.keyboard_focus {
            self.pending_events.push((
                surface_id,
                iced_core::Event::Keyboard(iced_core::keyboard::Event::ModifiersChanged(
                    self.modifiers,
                )),
            ));
        }
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
        // No-op: client-side repeat is handled by the calloop callback
        // registered in get_keyboard_with_repeat. Compositor-side repeat
        // events would cause duplicates.
    }
}

delegate_keyboard!(WaylandState);
