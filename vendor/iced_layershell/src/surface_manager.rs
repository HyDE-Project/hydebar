//! Surface management for `iced_layershell`.
//!
//! Handles creation and synchronization of Wayland layer surfaces
//! with their corresponding iced rendering surfaces.

use std::collections::HashMap;

use iced_graphics::Viewport;
use iced_graphics::compositor::Compositor as _;
use iced_renderer::Compositor;
use crate::task::InputRegionRect;
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShell, LayerSurface};
use wayland_client::QueueHandle;

use crate::settings::{LayerShellSettings, SurfaceId};
use crate::state::WaylandState;
use crate::task_impl::LayerShellCommand;
use crate::window_handle::WaylandWindow;

/// Per-surface iced rendering data.
pub(crate) struct IcedSurface {
    pub surface: <Compositor as iced_graphics::Compositor>::Surface,
    pub viewport: Viewport,
    pub cache: Option<iced_runtime::user_interface::Cache>,
    pub needs_redraw: bool,
}

/// Apply a synchronous layer shell command (surface create/destroy, property changes).
pub(crate) fn apply_layer_shell_command(
    cmd: LayerShellCommand,
    state: &mut WaylandState,
    pending_creations: &mut Vec<(SurfaceId, LayerShellSettings)>,
    pending_regions: &mut Vec<(SurfaceId, Option<Vec<InputRegionRect>>)>,
    _qh: &QueueHandle<WaylandState>,
) {
    match cmd {
        LayerShellCommand::NewSurface(id, settings) => {
            pending_creations.push((id, settings));
        }
        LayerShellCommand::DestroySurface(id) => {
            state.closed_surfaces.push(id);
        }
        LayerShellCommand::SetAnchor(id, anchor) => {
            if let Some(wl) = state.surface_id_map.get(&id)
                && let Some(data) = state.surfaces.get(wl)
            {
                data.layer_surface.set_anchor(anchor.to_sctk());
                data.layer_surface.wl_surface().commit();
            }
        }
        LayerShellCommand::SetLayer(id, layer) => {
            if let Some(wl) = state.surface_id_map.get(&id)
                && let Some(data) = state.surfaces.get(wl)
            {
                data.layer_surface.set_layer(layer.to_sctk());
                let wl_surf = data.layer_surface.wl_surface();
                // When hiding (Background), set empty input region so it
                // doesn't intercept clicks meant for surfaces above it.
                if layer == crate::settings::Layer::Background {
                    if let Ok(empty) =
                        smithay_client_toolkit::compositor::Region::new(&state.compositor)
                    {
                        wl_surf.set_input_region(Some(empty.wl_region()));
                    }
                } else {
                    wl_surf.set_input_region(None);
                }
                wl_surf.commit();
            }
        }
        LayerShellCommand::SetExclusiveZone(id, zone) => {
            if let Some(wl) = state.surface_id_map.get(&id)
                && let Some(data) = state.surfaces.get(wl)
            {
                data.layer_surface.set_exclusive_zone(zone);
                data.layer_surface.wl_surface().commit();
            }
        }
        LayerShellCommand::SetKeyboardInteractivity(id, ki) => {
            if let Some(wl) = state.surface_id_map.get(&id)
                && let Some(data) = state.surfaces.get(wl)
            {
                data.layer_surface.set_keyboard_interactivity(ki.to_sctk());
                data.layer_surface.wl_surface().commit();
            }
        }
        LayerShellCommand::SetSize(id, (w, h)) => {
            if let Some(wl) = state.surface_id_map.get(&id)
                && let Some(data) = state.surfaces.get_mut(wl)
            {
                data.layer_surface.set_size(w, h);
                data.layer_surface.wl_surface().commit();
            }
        }
        LayerShellCommand::SetMargin(id, (top, right, bottom, left)) => {
            if let Some(wl) = state.surface_id_map.get(&id)
                && let Some(data) = state.surfaces.get(wl)
            {
                data.layer_surface.set_margin(top, right, bottom, left);
                data.layer_surface.wl_surface().commit();
            }
        }
        LayerShellCommand::SetInputRegion(id, rects) => {
            // A surface arrives on the map only when its creation is
            // flushed; a region command racing that flush would be dropped
            // for good. Stash it here and flush_pending_creations applies
            // it once the surface exists.
            if !state.surface_id_map.contains_key(&id) {
                pending_regions.retain(|(stashed, _)| *stashed != id);
                pending_regions.push((id, rects));

                return;
            }

            if let Some(wl) = state.surface_id_map.get(&id)
                && let Some(data) = state.surfaces.get(wl)
            {
                let wl_surf = data.layer_surface.wl_surface();
                match rects {
                    None => {
                        wl_surf.set_input_region(None);
                    }
                    Some(rects) => {
                        if let Ok(region) =
                            smithay_client_toolkit::compositor::Region::new(&state.compositor)
                        {
                            for r in &rects {
                                region.add(r.x, r.y, r.width, r.height);
                            }
                            wl_surf.set_input_region(Some(region.wl_region()));
                        }
                    }
                }
                wl_surf.commit();
            }
        }
    }
}

/// Flush pending surface creations.
pub(crate) fn flush_pending_creations(
    wl: &mut WaylandState,
    pending: &mut Vec<(SurfaceId, LayerShellSettings)>,
    pending_regions: &mut Vec<(SurfaceId, Option<Vec<InputRegionRect>>)>,
    qh: &QueueHandle<WaylandState>,
) {
    while let Some((id, settings)) = pending.pop() {
        let (layer, scale) =
            create_layer_surface(&wl.compositor, &wl.layer_shell, qh, &settings, wl);
        wl.register_surface(id, layer, scale);

        if let Some(index) = pending_regions
            .iter()
            .position(|(stashed, _)| *stashed == id)
        {
            let (_, rects) = pending_regions.remove(index);

            apply_layer_shell_command(
                LayerShellCommand::SetInputRegion(id, rects),
                wl,
                pending,
                pending_regions,
                qh,
            );
        }
    }
}

/// Create a new Wayland layer surface from settings, targeting a specific output if configured.
/// Returns the layer surface together with the initial buffer scale applied to it.
pub(crate) fn create_layer_surface(
    compositor_state: &CompositorState,
    layer_shell_state: &LayerShell,
    qh: &QueueHandle<WaylandState>,
    settings: &LayerShellSettings,
    wl_state: &WaylandState,
) -> (LayerSurface, i32) {
    let surface = compositor_state.create_surface(qh);

    // Resolve OutputId → WlOutput for targeting a specific monitor
    let wl_output = settings.output.and_then(|output_id| {
        wl_state
            .outputs
            .iter()
            .find(|(_, info)| info.id == output_id)
            .map(|(wl_output, _)| wl_output.clone())
    });

    let layer_surface = layer_shell_state.create_layer_surface(
        qh,
        surface,
        settings.layer.to_sctk(),
        Some(settings.namespace.clone()),
        wl_output.as_ref(),
    );

    layer_surface.set_anchor(settings.anchor.to_sctk());
    layer_surface.set_exclusive_zone(settings.exclusive_zone);
    layer_surface.set_keyboard_interactivity(settings.keyboard_interactivity.to_sctk());

    if let Some((w, h)) = settings.size {
        layer_surface.set_size(w, h);
    }

    let (top, right, bottom, left) = settings.margin;
    layer_surface.set_margin(top, right, bottom, left);

    // A region stated here is part of the surface's initial state: the first
    // commit ships it, and no timing race can drop it. Background surfaces
    // default to an empty region so they never intercept input meant for the
    // surfaces stacked above them.
    let initial_region = settings
        .input_region
        .clone()
        .or_else(|| {
            (settings.layer == crate::settings::Layer::Background).then(Vec::new)
        });

    if let Some(rects) = initial_region
        && let Ok(region) = smithay_client_toolkit::compositor::Region::new(compositor_state)
    {
        for rect in &rects {
            region.add(rect.x, rect.y, rect.width, rect.height);
        }

        layer_surface
            .wl_surface()
            .set_input_region(Some(region.wl_region()));
    }

    // Set buffer scale for HiDPI: when the target output is known, use its
    // scale; otherwise default to 1 and rely on `wl_surface.enter` /
    // `preferred_buffer_scale` to correct it. Picking an arbitrary output's
    // scale here would mismatch the compositor's choice on multi-monitor
    // setups with different scales.
    let scale = wl_output
        .as_ref()
        .and_then(|wo| wl_state.outputs.get(wo))
        .map_or(1, |info| info.scale_factor);
    if scale > 1 {
        layer_surface.wl_surface().set_buffer_scale(scale);
    }

    layer_surface.commit();
    (layer_surface, scale)
}

/// Create a scaled cursor for the given surface.
///
/// Active touch position takes precedence over the pointer —
/// iced widgets check `cursor.is_over(bounds)` for touch events too.
pub(crate) fn scaled_cursor(
    wl_state: &WaylandState,
    surface_id: SurfaceId,
    app_scale: f32,
) -> iced_core::mouse::Cursor {
    if let Some((_, pos)) = wl_state
        .touch_fingers
        .values()
        .find(|(sid, _)| *sid == surface_id)
    {
        return iced_core::mouse::Cursor::Available(iced_core::Point::new(
            pos.x / app_scale,
            pos.y / app_scale,
        ));
    }

    if wl_state.pointer_surface == Some(surface_id) {
        let pos = wl_state.cursor_position;
        iced_core::mouse::Cursor::Available(iced_core::Point::new(
            pos.x / app_scale,
            pos.y / app_scale,
        ))
    } else {
        iced_core::mouse::Cursor::Unavailable
    }
}

/// Ensure every registered wayland surface has a corresponding iced rendering surface.
#[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
pub(crate) fn sync_iced_surfaces(
    wl_state: &WaylandState,
    compositor: &mut Compositor,
    iced_surfaces: &mut HashMap<SurfaceId, IcedSurface>,
    app_scale: f32,
) {
    for (wl_surface, data) in &wl_state.surfaces {
        if iced_surfaces.contains_key(&data.id) {
            continue;
        }
        // Only create wgpu surface after configure (need real dimensions)
        if !data.configured || data.size.0 == 0 || data.size.1 == 0 {
            continue;
        }
        if let Some(window) = WaylandWindow::new(wl_state.display_ptr, wl_surface) {
            let monitor_scale = data.scale_factor.max(1) as u32;
            let (w, h) = (
                data.size.0 * monitor_scale.max(1),
                data.size.1 * monitor_scale.max(1),
            );
            let combined_scale = data.scale_factor as f32 * app_scale;
            iced_surfaces.insert(
                data.id,
                IcedSurface {
                    surface: compositor.create_surface(window, w, h),
                    viewport: Viewport::with_physical_size(
                        iced_core::Size::new(w, h),
                        combined_scale,
                    ),
                    cache: None,
                    needs_redraw: true,
                },
            );
        }
    }
}
