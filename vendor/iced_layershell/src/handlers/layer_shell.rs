//! Layer shell handler for `wlr_layer_shell` protocol.
//!
//! Manages layer surface lifecycle including configure events that
//! communicate the compositor-assigned size and close events.

use smithay_client_toolkit::delegate_layer;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShellHandler, LayerSurface};
use wayland_client::{Connection, QueueHandle};

use crate::state::WaylandState;

impl LayerShellHandler for WaylandState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        let wl_surface = layer.wl_surface();
        if let Some(data) = self.surfaces.get(wl_surface) {
            self.closed_surfaces.push(data.id);
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let wl_surface = layer.wl_surface();
        if let Some(data) = self.surfaces.get_mut(wl_surface) {
            let new_size = configure.new_size;
            // (0, 0) means compositor hasn't set a size — keep current or use default
            if new_size.0 > 0 && new_size.1 > 0 {
                data.size = new_size;
            }
            data.configured = true;
            self.surfaces_need_redraw.insert(data.id);
        }
    }
}

delegate_layer!(WaylandState);
