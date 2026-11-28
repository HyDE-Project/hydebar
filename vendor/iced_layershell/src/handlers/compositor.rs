//! Wayland compositor handler for surface management.
//!
//! Handles surface lifecycle events including scale factor changes,
//! frame callbacks for synchronized rendering, and output enter/leave events.

use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::delegate_compositor;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, QueueHandle};

use crate::settings::OutputEvent;
use crate::state::WaylandState;

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        new_factor: i32,
    ) {
        if let Some(data) = self.surfaces.get_mut(surface) {
            data.scale_factor = new_factor;
            surface.set_buffer_scale(new_factor);
            // Don't commit here — the next render will commit with the correctly-scaled buffer
            self.surfaces_need_redraw.insert(data.id);
        }
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        _time: u32,
    ) {
        if let Some(data) = self.surfaces.get_mut(surface) {
            data.frame_pending = false;
            // Only trigger redraw if we have pending visual changes
            // that couldn't be presented last time.
            if data.needs_rerender {
                data.needs_rerender = false;
                self.surfaces_need_redraw.insert(data.id);
            }
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        output: &WlOutput,
    ) {
        let Some((output_id, output_scale)) = self
            .outputs
            .get(output)
            .map(|info| (info.id, info.scale_factor))
        else {
            return;
        };
        let Some(data) = self.surfaces.get_mut(surface) else {
            return;
        };
        // Some compositors deliver `preferred_buffer_scale` only after a first
        // commit; using the entered output's known scale here avoids a brief
        // wrong-scale render.
        if data.scale_factor != output_scale {
            data.scale_factor = output_scale;
            surface.set_buffer_scale(output_scale);
            self.surfaces_need_redraw.insert(data.id);
        }
        // Compositors can refire `enter` on commit — only emit when the output
        // actually changes so subscribers don't see redundant updates.
        if data.current_output == Some(output_id) {
            return;
        }
        data.current_output = Some(output_id);
        self.output_events.push(OutputEvent::SurfaceEnteredOutput {
            surface: data.id,
            output: output_id,
        });
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        output: &WlOutput,
    ) {
        let Some(output_id) = self.outputs.get(output).map(|info| info.id) else {
            return;
        };
        let Some(data) = self.surfaces.get_mut(surface) else {
            return;
        };
        if data.current_output == Some(output_id) {
            data.current_output = None;
        }
        self.output_events.push(OutputEvent::SurfaceLeftOutput {
            surface: data.id,
            output: output_id,
        });
    }
}

delegate_compositor!(WaylandState);
