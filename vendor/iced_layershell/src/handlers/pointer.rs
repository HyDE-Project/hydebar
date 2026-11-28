//! Pointer (mouse) handler for cursor input events.
//!
//! Processes pointer motion, button presses, and scroll events,
//! converting them to iced mouse events.

use smithay_client_toolkit::delegate_pointer;
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::{Connection, QueueHandle};

use crate::conversion;
use crate::state::WaylandState;

impl PointerHandler for WaylandState {
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let Some(surface_id) = self.surface_id_for(&event.surface) else {
                continue;
            };

            match event.kind {
                PointerEventKind::Enter { serial, .. } => {
                    self.pointer_enter_serial = serial;
                    self.pointer_surface = Some(surface_id);
                    self.cursor_position =
                        iced_core::Point::new(event.position.0 as f32, event.position.1 as f32);
                    self.pending_events.push((
                        surface_id,
                        iced_core::Event::Mouse(iced_core::mouse::Event::CursorEntered),
                    ));
                }
                PointerEventKind::Leave { .. } => {
                    self.pointer_surface = None;
                    self.pending_events.push((
                        surface_id,
                        iced_core::Event::Mouse(iced_core::mouse::Event::CursorLeft),
                    ));
                }
                PointerEventKind::Motion { .. } => {
                    self.cursor_position =
                        iced_core::Point::new(event.position.0 as f32, event.position.1 as f32);
                    self.pending_events.push((
                        surface_id,
                        iced_core::Event::Mouse(iced_core::mouse::Event::CursorMoved {
                            position: self.cursor_position,
                        }),
                    ));
                }
                PointerEventKind::Press { button, .. } => {
                    self.pending_events.push((
                        surface_id,
                        iced_core::Event::Mouse(iced_core::mouse::Event::ButtonPressed(
                            conversion::pointer_button_to_iced(button),
                        )),
                    ));
                }
                PointerEventKind::Release { button, .. } => {
                    self.pending_events.push((
                        surface_id,
                        iced_core::Event::Mouse(iced_core::mouse::Event::ButtonReleased(
                            conversion::pointer_button_to_iced(button),
                        )),
                    ));
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    // Wayland axis values encode the desired content-scroll
                    // direction (positive Y = scroll down); iced's ScrollDelta
                    // uses the inverse convention, so always negate. The
                    // `axis_relative_direction` flag is informational only
                    // (physical vs. logical motion, for kinetic scrolling)
                    // and must not affect the sign here — the compositor
                    // has already encoded per-device natural-scroll intent
                    // into the axis value itself.
                    let delta = if horizontal.value120 != 0 || vertical.value120 != 0 {
                        iced_core::mouse::ScrollDelta::Lines {
                            x: -horizontal.value120 as f32 / 120.0,
                            y: -vertical.value120 as f32 / 120.0,
                        }
                    } else if horizontal.discrete != 0 || vertical.discrete != 0 {
                        iced_core::mouse::ScrollDelta::Lines {
                            x: -horizontal.discrete as f32,
                            y: -vertical.discrete as f32,
                        }
                    } else {
                        iced_core::mouse::ScrollDelta::Pixels {
                            x: -horizontal.absolute as f32,
                            y: -vertical.absolute as f32,
                        }
                    };
                    self.pending_events.push((
                        surface_id,
                        iced_core::Event::Mouse(iced_core::mouse::Event::WheelScrolled { delta }),
                    ));
                }
            }
        }
    }
}

delegate_pointer!(WaylandState);
