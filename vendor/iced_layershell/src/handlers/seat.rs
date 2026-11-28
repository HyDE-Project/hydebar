//! Seat handler for input device management.
//!
//! Manages seat capabilities (keyboard, pointer, touch) and
//! initializes input devices when they become available.

use smithay_client_toolkit::delegate_seat;
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, QueueHandle};

use crate::handlers::keyboard::make_key_pressed;
use crate::state::WaylandState;

impl SeatHandler for WaylandState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Keyboard => {
                self.seat
                    .get_keyboard_with_repeat(
                        qh,
                        &seat,
                        None,
                        self.loop_handle.clone(),
                        Box::new(|state, _keyboard, event| {
                            if let Some(surface_id) = state.keyboard_focus {
                                state.pending_events.push((
                                    surface_id,
                                    make_key_pressed(&event, state.modifiers, true),
                                ));
                            }
                        }),
                    )
                    .ok();
                if self.data_device.is_none() {
                    self.data_device = Some(self.data_device_manager.get_data_device(qh, &seat));
                }
            }
            Capability::Pointer => {
                if let Ok(pointer) = self.seat.get_pointer(qh, &seat) {
                    self.wl_pointer = Some(pointer);
                }
            }
            Capability::Touch => {
                self.seat.get_touch(qh, &seat).ok();
            }
            _ => {}
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Pointer => {
                self.wl_pointer = None;
                self.pointer_surface = None;
            }
            Capability::Keyboard => {
                self.keyboard_focus = None;
            }
            _ => {}
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}
}

delegate_seat!(WaylandState);
