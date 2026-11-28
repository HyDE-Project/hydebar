//! Touch handler for touchscreen input events.
//!
//! Processes touch down, up, motion, and cancel events for
//! multi-touch capable displays.

use smithay_client_toolkit::delegate_touch;
use smithay_client_toolkit::seat::touch::TouchHandler;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::protocol::wl_touch::WlTouch;
use wayland_client::{Connection, QueueHandle};

use crate::state::WaylandState;

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
impl TouchHandler for WaylandState {
    fn down(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _serial: u32,
        _time: u32,
        surface: WlSurface,
        id: i32,
        position: (f64, f64),
    ) {
        if let Some(surface_id) = self.surface_id_for(&surface) {
            self.pending_finger_removals.retain(|&fid| fid != id);
            let pos = iced_core::Point::new(position.0 as f32, position.1 as f32);
            self.touch_fingers.insert(id, (surface_id, pos));
            self.pending_events.push((
                surface_id,
                iced_core::Event::Touch(iced_core::touch::Event::FingerPressed {
                    id: iced_core::touch::Finger(id as u64),
                    position: pos,
                }),
            ));
        }
    }

    fn up(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _serial: u32,
        _time: u32,
        id: i32,
    ) {
        if let Some(&(surface_id, pos)) = self.touch_fingers.get(&id) {
            self.pending_finger_removals.push(id);
            self.pending_events.push((
                surface_id,
                iced_core::Event::Touch(iced_core::touch::Event::FingerLifted {
                    id: iced_core::touch::Finger(id as u64),
                    position: pos,
                }),
            ));
        }
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _time: u32,
        id: i32,
        position: (f64, f64),
    ) {
        let pos = iced_core::Point::new(position.0 as f32, position.1 as f32);
        if let Some((surface_id, stored_pos)) = self.touch_fingers.get_mut(&id) {
            *stored_pos = pos;
            self.pending_events.push((
                *surface_id,
                iced_core::Event::Touch(iced_core::touch::Event::FingerMoved {
                    id: iced_core::touch::Finger(id as u64),
                    position: pos,
                }),
            ));
        }
    }

    fn cancel(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _touch: &WlTouch) {
        self.pending_finger_removals.clear();
        for (id, (surface_id, pos)) in self.touch_fingers.drain() {
            self.pending_events.push((
                surface_id,
                iced_core::Event::Touch(iced_core::touch::Event::FingerLost {
                    id: iced_core::touch::Finger(id as u64),
                    position: pos,
                }),
            ));
        }
    }

    fn shape(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _id: i32,
        _major: f64,
        _minor: f64,
    ) {
    }

    fn orientation(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &WlTouch,
        _id: i32,
        _orientation: f64,
    ) {
    }
}

delegate_touch!(WaylandState);
