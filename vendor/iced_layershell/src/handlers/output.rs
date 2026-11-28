//! Output (monitor) handler for display management.
//!
//! Tracks connected outputs and their properties, emitting events
//! when monitors are added, removed, or changed.

use smithay_client_toolkit::delegate_output;
use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::SeatState;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, QueueHandle};

use crate::settings::{OutputEvent, OutputInfo};
use crate::state::WaylandState;

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        let id = self.allocate_output_id();
        let sctk_info = self.output.info(&output);
        let info = OutputInfo {
            id,
            name: sctk_info
                .as_ref()
                .and_then(|i| i.name.clone())
                .unwrap_or_default(),
            scale_factor: sctk_info.as_ref().map_or(1, |i| i.scale_factor),
            logical_size: sctk_info
                .as_ref()
                .and_then(|i| i.logical_size.map(|s| (s.0, s.1))),
            make: sctk_info
                .as_ref()
                .map(|i| i.make.clone())
                .unwrap_or_default(),
            model: sctk_info
                .as_ref()
                .map(|i| i.model.clone())
                .unwrap_or_default(),
        };
        self.output_events.push(OutputEvent::Added(info.clone()));
        self.outputs.insert(output, info);
    }

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        if let Some(existing) = self.outputs.get_mut(&output) {
            let sctk_info = self.output.info(&output);
            if let Some(si) = sctk_info {
                existing.name = si.name.clone().unwrap_or_default();
                existing.scale_factor = si.scale_factor;
                existing.logical_size = si.logical_size.map(|s| (s.0, s.1));
                existing.make.clone_from(&si.make);
                existing.model.clone_from(&si.model);
            }
            self.output_events
                .push(OutputEvent::InfoChanged(existing.clone()));
        }
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        if let Some(info) = self.outputs.remove(&output) {
            self.output_events.push(OutputEvent::Removed(info.id));
        }
    }
}

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    smithay_client_toolkit::registry_handlers!(OutputState, SeatState);
}

delegate_output!(WaylandState);
delegate_registry!(WaylandState);
