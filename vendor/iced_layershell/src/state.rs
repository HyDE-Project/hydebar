//! Central Wayland state management.
//!
//! This module defines the core state structures and provides
//! methods for surface management. Protocol handlers are in
//! the [`handlers`](crate::handlers) module.

use std::collections::{HashMap, HashSet};

use calloop::LoopHandle;
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::data_device_manager::DataDeviceManagerState;
use smithay_client_toolkit::data_device_manager::data_device::DataDevice;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::seat::pointer::cursor_shape::CursorShapeManager;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShell, LayerSurface};
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, QueueHandle};

use crate::settings::{OutputEvent, OutputId, OutputInfo, SurfaceId};

/// Per-surface Wayland data.
pub(crate) struct SurfaceData {
    pub id: SurfaceId,
    pub layer_surface: LayerSurface,
    pub size: (u32, u32),
    pub scale_factor: i32,
    pub configured: bool,
    pub frame_pending: bool,
    /// Set when we drew but couldn't present (`frame_pending` was true).
    /// Frame callback will trigger a redraw to flush the pending visual.
    pub needs_rerender: bool,
    /// Output the surface is currently mapped on, set from `wl_surface.enter`
    /// events. `None` until the compositor maps the surface (or after a
    /// `wl_surface.leave` if not re-entered).
    pub current_output: Option<OutputId>,
}

/// Central Wayland state, holding all SCTK protocol states and event queues.
pub(crate) struct WaylandState {
    pub registry: RegistryState,
    pub compositor: CompositorState,
    pub layer_shell: LayerShell,
    pub seat: SeatState,
    pub output: OutputState,

    // Surface tracking
    pub surfaces: HashMap<WlSurface, SurfaceData>,
    pub surface_id_map: HashMap<SurfaceId, WlSurface>,

    // Input state
    pub cursor_position: iced_core::Point,
    pub pointer_surface: Option<SurfaceId>,
    pub keyboard_focus: Option<SurfaceId>,
    pub modifiers: iced_core::keyboard::Modifiers,

    // Event queues (drained each frame by the application runner)
    pub pending_events: Vec<(SurfaceId, iced_core::Event)>,
    pub output_events: Vec<OutputEvent>,
    pub surfaces_need_redraw: HashSet<SurfaceId>,
    pub closed_surfaces: Vec<SurfaceId>,

    // Output tracking
    pub outputs: HashMap<WlOutput, OutputInfo>,
    next_output_id: u32,

    // Clipboard / data device
    pub data_device_manager: DataDeviceManagerState,
    pub data_device: Option<DataDevice>,

    // Touch state: track active finger positions for cancel events
    pub touch_fingers: HashMap<i32, (SurfaceId, iced_core::Point)>,
    // Finger IDs to remove from touch_fingers after event processing.
    // Deferred so scaled_cursor() still sees the finger during FingerLifted.
    pub pending_finger_removals: Vec<i32>,

    // Cursor shape
    pub cursor_shape_manager: Option<CursorShapeManager>,
    pub current_mouse_interaction: iced_core::mouse::Interaction,
    pub pointer_enter_serial: u32,
    pub wl_pointer: Option<WlPointer>,

    // Calloop loop handle for keyboard repeat
    pub loop_handle: LoopHandle<'static, WaylandState>,

    // Raw display pointer for raw-window-handle
    pub display_ptr: *mut std::ffi::c_void,
}

impl WaylandState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: RegistryState,
        compositor: CompositorState,
        layer_shell: LayerShell,
        seat: SeatState,
        output: OutputState,
        data_device_manager: DataDeviceManagerState,
        loop_handle: LoopHandle<'static, WaylandState>,
        conn: &Connection,
    ) -> Self {
        let display_ptr = conn.backend().display_ptr().cast::<std::ffi::c_void>();

        Self {
            registry,
            compositor,
            layer_shell,
            seat,
            output,
            surfaces: HashMap::new(),
            surface_id_map: HashMap::new(),
            cursor_position: iced_core::Point::ORIGIN,
            pointer_surface: None,
            keyboard_focus: None,
            modifiers: iced_core::keyboard::Modifiers::empty(),
            pending_events: Vec::new(),
            output_events: Vec::new(),
            surfaces_need_redraw: HashSet::new(),
            closed_surfaces: Vec::new(),
            outputs: HashMap::new(),
            next_output_id: 0,
            data_device_manager,
            data_device: None,
            touch_fingers: HashMap::new(),
            pending_finger_removals: Vec::new(),
            cursor_shape_manager: None,
            current_mouse_interaction: iced_core::mouse::Interaction::default(),
            pointer_enter_serial: 0,
            wl_pointer: None,
            loop_handle,
            display_ptr,
        }
    }

    /// Register a new layer surface with the given ID and initial buffer scale.
    /// `scale_factor` must match whatever was applied via
    /// `wl_surface.set_buffer_scale` at creation, so the renderer and the
    /// compositor agree on the buffer's coordinate system.
    pub fn register_surface(
        &mut self,
        id: SurfaceId,
        layer_surface: LayerSurface,
        scale_factor: i32,
    ) {
        let wl_surface = layer_surface.wl_surface().clone();
        self.surfaces.insert(
            wl_surface.clone(),
            SurfaceData {
                id,
                layer_surface,
                size: (0, 0),
                scale_factor,
                configured: false,
                frame_pending: false,
                needs_rerender: false,
                current_output: None,
            },
        );
        self.surface_id_map.insert(id, wl_surface);
    }

    /// Get the `SurfaceId` for a given Wayland surface.
    pub fn surface_id_for(&self, wl_surface: &WlSurface) -> Option<SurfaceId> {
        self.surfaces.get(wl_surface).map(|d| d.id)
    }

    /// Set the cursor shape based on mouse interaction.
    pub fn set_cursor_shape(
        &mut self,
        interaction: iced_core::mouse::Interaction,
        qh: &QueueHandle<WaylandState>,
    ) {
        use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;

        if interaction == self.current_mouse_interaction {
            return;
        }
        self.current_mouse_interaction = interaction;

        let Some(manager) = &self.cursor_shape_manager else {
            return;
        };
        let Some(pointer) = &self.wl_pointer else {
            return;
        };

        // For a status bar, only change cursor for text input fields.
        // Everything else keeps the default arrow.
        let shape = match interaction {
            iced_core::mouse::Interaction::Text => Shape::Text,
            _ => Shape::Default,
        };

        let device = manager.get_shape_device(pointer, qh);
        device.set_shape(self.pointer_enter_serial, shape);
        device.destroy();
    }

    /// Allocate a new unique output ID.
    pub(crate) fn allocate_output_id(&mut self) -> OutputId {
        let id = OutputId(self.next_output_id);
        self.next_output_id += 1;
        id
    }
}
