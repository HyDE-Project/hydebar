//! Software-drawn frame surfaces via shared memory.
//!
//! The frame's ring and its reservation strips are drawn here, on small
//! layer surfaces that never allocate a GPU swapchain buffer: a 4K ring on
//! wgpu held ~122 MiB of GPU memory, and each additional wgpu surface
//! cost ~3 MiB of fixed overhead.  Shared-memory surfaces cost the
//! compositor nothing beyond the few kilobytes they actually paint, and
//! they commit only when the configuration changes.
//!
//! The module owns its own Wayland connection and event loop on a
//! background thread.  Configuration changes are applied by stopping and
//! restarting the thread; the handle returned by [`start`] carries the
//! join handle and a stop flag for that purpose.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering}
    },
    thread::{self, JoinHandle}
};

use hydebar_proto::config::FrameConfig;
use num_traits::ToPrimitive;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure
        }
    },
    shm::{Shm, ShmHandler, slot::SlotPool}
};
use tiny_skia::{Color, Paint, Pixmap, Transform};
use wayland_client::{
    Connection, Proxy, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface}
};

use crate::config::Position;

/// Handle to a running shared-memory frame.
#[derive(Debug)]
pub struct ShmFrameHandle {
    stop:   Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>
}

impl ShmFrameHandle {
    /// Signal the background thread to exit and wait for it.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Spawn the frame on a background thread.
///
/// The thread enumerates outputs itself and creates one thin strip per
/// drawn edge plus one corner overlay per outer corner.  It blocks until
/// [`ShmFrameHandle::stop`] is called.
#[must_use]
pub fn start(
    config: FrameConfig,
    dock: Position,
    bar_height: f32,
    color: (u8, u8, u8, u8)
) -> ShmFrameHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    let thread = thread::spawn(move || {
        run(config, dock, bar_height, color, stop_clone);
    });

    ShmFrameHandle {
        stop,
        thread: Some(thread)
    }
}

#[derive(Debug, Clone, Copy)]
enum SurfaceKind {
    StripLeft,
    StripRight,
    StripTop,
    StripBottom,
    CornerTopLeft,
    CornerTopRight,
    CornerBottomLeft,
    CornerBottomRight
}

struct SurfaceState {
    kind:       SurfaceKind,
    layer:      LayerSurface,
    width:      u32,
    height:     u32,
    pool:       Option<SlotPool>,
    configured: bool,
    output:     wl_output::WlOutput
}

struct App {
    registry_state: RegistryState,
    output_state:   OutputState,
    shm:            Shm,
    compositor:     CompositorState,
    layer_shell:    LayerShell,
    config:         FrameConfig,
    dock:           Position,
    bar_height:     f32,
    color:          (u8, u8, u8, u8),
    exit:           bool,
    surfaces:       Vec<SurfaceState>
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput
    ) {
        self.create_surfaces_for_output(qh, &output);
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput
    ) {
        self.surfaces.retain(|s| s.output.id() != output.id());
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32
    ) {
        for state in &mut self.surfaces {
            if state.layer == *layer {
                state.width = configure.new_size.0.max(1);
                state.height = configure.new_size.1.max(1);

                if !state.configured {
                    let size = (state.width * state.height * 4) as usize;
                    if let Ok(pool) = SlotPool::new(size.max(4), &self.shm) {
                        state.pool = Some(pool);
                        state.configured = true;
                        draw_surface(state, &self.config, self.color);
                    }
                }
            }
        }
        let _ = qh;
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    smithay_client_toolkit::registry_handlers![OutputState];
}

smithay_client_toolkit::delegate_registry!(App);
smithay_client_toolkit::delegate_dispatch2!(App);

impl App {
    fn create_surfaces_for_output(
        &mut self,
        qh: &QueueHandle<Self>,
        output: &wl_output::WlOutput
    ) {
        let thickness = self.config.thickness.max(1.0).to_u32().unwrap_or(1);
        let radius = self
            .config
            .rounding
            .unwrap_or(FrameConfig::DEFAULT_ROUNDING)
            .max(0.0);

        self.create_strips(qh, output, thickness);

        let radius_px = radius.ceil().to_u32().unwrap_or(u32::MAX);
        let corner_size = thickness.saturating_add(radius_px);
        self.create_corners(qh, output, corner_size);
    }

    /// Creates the four thin reservation strips, skipping disabled edges.
    ///
    /// Size along a strip's own axis is set by the compositor; the
    /// perpendicular axis is one pixel wide, reserving the full thickness.
    fn create_strips(
        &mut self,
        qh: &QueueHandle<Self>,
        output: &wl_output::WlOutput,
        thickness: u32
    ) {
        let bar_margin = self.bar_height.round().to_i32().unwrap_or(0);
        let top_margin = if self.dock == Position::Top {
            bar_margin
        } else {
            0
        };
        let bottom_margin = if self.dock == Position::Bottom {
            bar_margin
        } else {
            0
        };

        let edges: &[(SurfaceKind, Anchor, i32, i32, i32, i32)] = &[
            (
                SurfaceKind::StripLeft,
                Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
                top_margin,
                0,
                bottom_margin,
                0
            ),
            (
                SurfaceKind::StripRight,
                Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM,
                top_margin,
                0,
                bottom_margin,
                0
            ),
            (
                SurfaceKind::StripTop,
                Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                0,
                0,
                0,
                0
            ),
            (
                SurfaceKind::StripBottom,
                Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
                0,
                0,
                0,
                0
            )
        ];

        for (kind, anchor, mt, mr, mb, ml) in edges {
            if !self.strip_enabled(*kind) {
                continue;
            }
            let surface = self.compositor.create_surface(qh);
            let ls = self.layer_shell.create_layer_surface(
                qh,
                surface,
                Layer::Top,
                Some("hydebar-frame-strip"),
                Some(output)
            );
            ls.set_anchor(*anchor);
            ls.set_keyboard_interactivity(KeyboardInteractivity::None);
            match *kind {
                SurfaceKind::StripLeft | SurfaceKind::StripRight => ls.set_size(thickness, 0),
                SurfaceKind::StripTop | SurfaceKind::StripBottom => ls.set_size(0, thickness),
                _ => {}
            }
            ls.set_exclusive_zone(i32::try_from(thickness).unwrap_or(i32::MAX));
            ls.set_margin(*mt, *mr, *mb, *ml);
            ls.commit();

            self.surfaces.push(SurfaceState {
                kind:       *kind,
                layer:      ls,
                width:      1,
                height:     1,
                pool:       None,
                configured: false,
                output:     output.clone()
            });
        }
    }

    /// Creates the four corner overlays, sized to cover the rounded aperture.
    fn create_corners(
        &mut self,
        qh: &QueueHandle<Self>,
        output: &wl_output::WlOutput,
        corner_size: u32
    ) {
        let corners = [
            (SurfaceKind::CornerTopLeft, Anchor::LEFT | Anchor::TOP),
            (SurfaceKind::CornerTopRight, Anchor::RIGHT | Anchor::TOP),
            (SurfaceKind::CornerBottomLeft, Anchor::LEFT | Anchor::BOTTOM),
            (
                SurfaceKind::CornerBottomRight,
                Anchor::RIGHT | Anchor::BOTTOM
            )
        ];

        for (kind, anchor) in corners {
            let surface = self.compositor.create_surface(qh);
            let ls = self.layer_shell.create_layer_surface(
                qh,
                surface,
                Layer::Top,
                Some("hydebar-frame-corner"),
                Some(output)
            );
            ls.set_anchor(anchor);
            ls.set_size(corner_size, corner_size);
            ls.set_keyboard_interactivity(KeyboardInteractivity::None);
            ls.set_exclusive_zone(-1);
            ls.commit();

            self.surfaces.push(SurfaceState {
                kind,
                layer: ls,
                width: corner_size,
                height: corner_size,
                pool: None,
                configured: false,
                output: output.clone()
            });
        }
    }

    fn strip_enabled(&self, kind: SurfaceKind) -> bool {
        match kind {
            SurfaceKind::StripLeft => self.config.edges.left.enabled,
            SurfaceKind::StripRight => self.config.edges.right.enabled,
            SurfaceKind::StripTop => self.config.edges.top.enabled,
            SurfaceKind::StripBottom => self.config.edges.bottom.enabled,
            SurfaceKind::CornerTopLeft
            | SurfaceKind::CornerTopRight
            | SurfaceKind::CornerBottomLeft
            | SurfaceKind::CornerBottomRight => false
        }
    }
}

fn draw_surface(state: &mut SurfaceState, config: &FrameConfig, color: (u8, u8, u8, u8)) {
    let (Ok(width), Ok(height), Ok(stride)) = (
        i32::try_from(state.width),
        i32::try_from(state.height),
        i32::try_from(state.width.saturating_mul(4))
    ) else {
        return;
    };
    let Some(mut pixmap) = Pixmap::new(state.width, state.height) else {
        return;
    };
    let Some(pool) = state.pool.as_mut() else {
        return;
    };
    let Ok((buffer, canvas)) = pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888)
    else {
        return;
    };

    pixmap.fill(Color::from_rgba8(color.0, color.1, color.2, color.3));

    // Strips are solid 1px lines; corners get a quarter-circle cut so
    // the aperture reads rounded instead of square.
    let corner_center = match (
        state.kind,
        u16::try_from(state.width),
        u16::try_from(state.height)
    ) {
        (SurfaceKind::CornerTopLeft, Ok(w), Ok(h)) => Some((f32::from(w), f32::from(h))),
        (SurfaceKind::CornerTopRight, _, Ok(h)) => Some((0.0, f32::from(h))),
        (SurfaceKind::CornerBottomLeft, Ok(w), _) => Some((f32::from(w), 0.0)),
        (SurfaceKind::CornerBottomRight, _, _) => Some((0.0, 0.0)),
        _ => None
    };
    if let Some((cx, cy)) = corner_center {
        let mut clear = Paint::default();
        clear.set_color(Color::from_rgba8(0, 0, 0, 0));
        clear.blend_mode = tiny_skia::BlendMode::Clear;
        clear.anti_alias = true;

        let r = config
            .rounding
            .unwrap_or(FrameConfig::DEFAULT_ROUNDING)
            .max(1.0);

        if let Some(path) = tiny_skia::PathBuilder::from_circle(cx, cy, r) {
            pixmap.fill_path(
                &path,
                &clear,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None
            );
        }
    }

    let n = canvas.len().min(pixmap.data().len());
    canvas[..n].copy_from_slice(&pixmap.data()[..n]);

    state
        .layer
        .wl_surface()
        .attach(Some(buffer.wl_buffer()), 0, 0);
    state.layer.wl_surface().damage_buffer(0, 0, width, height);
    state.layer.commit();
}

fn run(
    config: FrameConfig,
    dock: Position,
    bar_height: f32,
    color: (u8, u8, u8, u8),
    stop: Arc<AtomicBool>
) {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(_) => return
    };
    let (globals, mut event_queue) = match registry_queue_init(&conn) {
        Ok(v) => v,
        Err(_) => return
    };
    let qh = event_queue.handle();

    let compositor = match CompositorState::bind(&globals, &qh) {
        Ok(c) => c,
        Err(_) => return
    };
    let layer_shell = match LayerShell::bind(&globals, &qh) {
        Ok(l) => l,
        Err(_) => return
    };
    let shm = match Shm::bind(&globals, &qh) {
        Ok(s) => s,
        Err(_) => return
    };

    let mut app = App {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        shm,
        compositor,
        layer_shell,
        config,
        dock,
        bar_height,
        color,
        exit: false,
        surfaces: Vec::new()
    };

    loop {
        if stop.load(Ordering::Relaxed) || app.exit {
            break;
        }
        if event_queue.blocking_dispatch(&mut app).is_err() {
            break;
        }
    }
}
