use std::borrow::Cow;
use std::collections::HashMap;
use std::mem::{self, ManuallyDrop};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;
use futures::{StreamExt, channel::mpsc};
use iced_core::{Font, Size, Theme};
use iced_graphics::Viewport;
use iced_graphics::compositor::Compositor as _;
use iced_renderer::Compositor;
use iced_runtime::Action;
use iced_runtime::user_interface::{self, UserInterface};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::data_device_manager::DataDeviceManagerState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use wayland_client::Connection;
use wayland_client::globals::registry_queue_init;

use crate::error::Error;
use crate::event_loop::WakeupSender;
use crate::settings::{LayerShellSettings, SurfaceId};
use crate::state::WaylandState;
use crate::surface_manager::{
    IcedSurface, apply_layer_shell_command, create_layer_surface, flush_pending_creations,
    scaled_cursor, sync_iced_surfaces,
};
use crate::task_impl::Task;
use crate::ui_builder::{build_single_ui, build_user_interfaces};
use crate::wayland_clipboard::WaylandClipboard;
use crate::window_handle::WaylandWindow;

type Element<'a, M> = iced_core::Element<'a, M, Theme, iced_renderer::Renderer>;

/// Builder for a layer shell application.
///
/// Created via [`application()`], configured with builder methods, then started with [`run()`](Self::run).
#[allow(clippy::type_complexity)]
pub struct Application<State, Message> {
    boot: Box<dyn FnOnce() -> (State, Task<Message>)>,
    update: Box<dyn Fn(&mut State, Message) -> Task<Message>>,
    view: Box<dyn for<'a> Fn(&'a State, SurfaceId) -> Element<'a, Message>>,
    initial_settings: Option<LayerShellSettings>,
    subscription_fn: Option<Box<dyn Fn(&State) -> iced_futures::Subscription<Message>>>,
    theme_fn: Option<Box<dyn Fn(&State) -> Theme>>,
    scale_factor_fn: Option<Box<dyn Fn(&State) -> f64>>,
    fonts: Vec<Cow<'static, [u8]>>,
    default_font: Font,
    antialiasing: bool,
}

impl<State, Message> Application<State, Message>
where
    State: 'static,
    Message: std::fmt::Debug + Send + Clone + 'static,
{
    /// Configure the initial layer shell surface.
    pub fn layer_shell(mut self, settings: LayerShellSettings) -> Self {
        self.initial_settings = Some(settings);
        self
    }

    /// Set the subscription function for async background tasks (timers, streams, etc.).
    pub fn subscription(
        mut self,
        f: impl Fn(&State) -> iced_futures::Subscription<Message> + 'static,
    ) -> Self {
        self.subscription_fn = Some(Box::new(f));
        self
    }

    /// Set the theme function. Called each frame to determine the current theme.
    pub fn theme(mut self, f: impl Fn(&State) -> Theme + 'static) -> Self {
        self.theme_fn = Some(Box::new(f));
        self
    }

    /// Load a custom font from bytes at startup.
    pub fn font(mut self, bytes: impl Into<Cow<'static, [u8]>>) -> Self {
        self.fonts.push(bytes.into());
        self
    }

    /// Set the default font for text rendering.
    pub fn default_font(mut self, font: Font) -> Self {
        self.default_font = font;
        self
    }

    /// Set the application scale factor (on top of monitor DPI).
    /// For example, 1.2 means 120% zoom. Default is 1.0.
    pub fn scale_factor(mut self, f: impl Fn(&State) -> f64 + 'static) -> Self {
        self.scale_factor_fn = Some(Box::new(f));
        self
    }

    /// Enable or disable antialiasing (`MSAAx4`).
    pub fn antialiasing(mut self, enabled: bool) -> Self {
        self.antialiasing = enabled;
        self
    }

    /// Run the application, blocking until it exits.
    pub fn run(self) -> Result<(), Error> {
        run(self)
    }
}

/// Create a new layer shell application with the Elm architecture.
///
/// - `boot` initializes state and returns an optional startup task.
/// - `update` handles messages and returns tasks.
/// - `view` builds the UI for a given surface (called with each [`SurfaceId`]).
pub fn application<State, Message>(
    boot: impl FnOnce() -> (State, Task<Message>) + 'static,
    update: impl Fn(&mut State, Message) -> Task<Message> + 'static,
    view: impl for<'a> Fn(&'a State, SurfaceId) -> Element<'a, Message> + 'static,
) -> Application<State, Message>
where
    State: 'static,
    Message: std::fmt::Debug + Send + Clone + 'static,
{
    Application {
        boot: Box::new(boot),
        update: Box::new(update),
        view: Box::new(view),
        initial_settings: None,
        subscription_fn: None,
        theme_fn: None,
        scale_factor_fn: None,
        fonts: Vec::new(),
        default_font: Font::DEFAULT,
        antialiasing: false,
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
fn run<State, Message>(app: Application<State, Message>) -> Result<(), Error>
where
    State: 'static,
    Message: std::fmt::Debug + Send + Clone + 'static,
{
    let initial_settings = app.initial_settings.ok_or(Error::NoSettings)?;

    // Layer-shell UIs (bars, overlays) want the integrated GPU; the wgpu default
    // wakes the discrete one on hybrid laptops. User can still override with
    // WGPU_POWER_PREF=high.
    if std::env::var_os("WGPU_POWER_PREF").is_none() {
        // Safety: no other threads have been spawned yet in this process by us.
        unsafe { std::env::set_var("WGPU_POWER_PREF", "low") };
    }

    crate::output_subscription::init();

    let conn = Connection::connect_to_env()?;
    let display_ptr = conn.backend().display_ptr().cast::<std::ffi::c_void>();
    // Create clipboard early — smithay-clipboard spawns its own worker thread
    // with its own wayland connection that needs to receive selection events
    // Safety: display_ptr is valid for the lifetime of the Wayland connection
    let mut clipboard = unsafe { WaylandClipboard::new(display_ptr) };

    let (globals, mut event_queue) = registry_queue_init::<WaylandState>(&conn)?;
    let qh = event_queue.handle();

    let compositor_state =
        CompositorState::bind(&globals, &qh).map_err(|e| Error::EventLoop(e.to_string()))?;
    let layer_shell_state =
        LayerShell::bind(&globals, &qh).map_err(|_| Error::LayerShellNotSupported)?;
    let seat_state = SeatState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);
    let data_device_manager =
        DataDeviceManagerState::bind(&globals, &qh).map_err(|e| Error::EventLoop(e.to_string()))?;
    let cursor_shape_manager =
        smithay_client_toolkit::seat::pointer::cursor_shape::CursorShapeManager::bind(
            &globals, &qh,
        )
        .ok();

    // Create calloop event loop early so we can pass the LoopHandle to
    // keyboard with repeat (new_capability fires during roundtrip)
    let mut event_loop: EventLoop<WaylandState> =
        EventLoop::try_new().map_err(|e| Error::EventLoop(e.to_string()))?;

    let mut wl_state = WaylandState::new(
        registry_state,
        compositor_state,
        layer_shell_state,
        seat_state,
        output_state,
        data_device_manager,
        event_loop.handle(),
        &conn,
    );
    wl_state.cursor_shape_manager = cursor_shape_manager;

    // Create initial layer surface (SurfaceId::MAIN)
    let (initial_layer, initial_scale) = create_layer_surface(
        &wl_state.compositor,
        &wl_state.layer_shell,
        &qh,
        &initial_settings,
        &wl_state,
    );
    wl_state.register_surface(SurfaceId::MAIN, initial_layer, initial_scale);

    // Roundtrip to get initial configure
    event_queue
        .roundtrip(&mut wl_state)
        .map_err(|e| Error::EventLoop(e.to_string()))?;

    // Ensure initial surface is configured
    let main_wl = wl_state
        .surface_id_map
        .get(&SurfaceId::MAIN)
        .cloned()
        .ok_or(Error::EventLoop("main surface lost".into()))?;

    if !wl_state
        .surfaces
        .get(&main_wl)
        .is_some_and(|d| d.configured)
    {
        event_queue
            .roundtrip(&mut wl_state)
            .map_err(|e| Error::EventLoop(e.to_string()))?;
    }

    let main_data = wl_state
        .surfaces
        .get(&main_wl)
        .ok_or_else(|| Error::EventLoop("main surface data missing after registration".into()))?;
    let monitor_scale = main_data.scale_factor.max(1) as u32;
    let (width, height) = if main_data.size.0 > 0 && main_data.size.1 > 0 {
        // Convert surface-local to physical pixels
        (
            main_data.size.0 * monitor_scale,
            main_data.size.1 * monitor_scale,
        )
    } else {
        (800, 30)
    };

    let window_handle = WaylandWindow::new(wl_state.display_ptr, &main_wl)
        .ok_or(Error::EventLoop("failed to create window handle".into()))?;

    let graphics_settings = iced_graphics::Settings {
        default_font: app.default_font,
        default_text_size: iced_core::Pixels(14.0),
        antialiasing: if app.antialiasing {
            Some(iced_graphics::Antialiasing::MSAAx4)
        } else {
            None
        },
        vsync: true,
    };

    let mut compositor = futures::executor::block_on(iced_renderer::Compositor::new(
        graphics_settings,
        window_handle.clone(),
        window_handle.clone(),
        iced_graphics::Shell::headless(),
    ))
    .map_err(Error::Graphics)?;

    let mut renderer = compositor.create_renderer();

    for font_bytes in &app.fonts {
        compositor.load_font(font_bytes.clone());
    }

    let initial_app_scale = 1.0f32;
    let initial_scale = main_data.scale_factor as f32 * initial_app_scale;

    let mut iced_surfaces: HashMap<SurfaceId, IcedSurface> = HashMap::new();
    iced_surfaces.insert(
        SurfaceId::MAIN,
        IcedSurface {
            surface: compositor.create_surface(window_handle, width, height),
            viewport: Viewport::with_physical_size(Size::new(width, height), initial_scale),
            needs_redraw: true,
            cache: None,
        },
    );

    let executor = iced_futures::backend::default::Executor::new()
        .map_err(|e| Error::EventLoop(e.to_string()))?;
    let (ping, ping_source) =
        calloop::ping::make_ping().map_err(|e| Error::EventLoop(e.to_string()))?;
    let (runtime_sender, mut runtime_receiver) = mpsc::unbounded::<Action<Message>>();
    let wakeup_sender = WakeupSender {
        inner: runtime_sender,
        ping: ping.clone(),
    };
    let mut runtime = iced_futures::Runtime::new(executor, wakeup_sender);
    let exit_flag = Arc::new(AtomicBool::new(false));

    let (mut user_state, boot_task) = runtime.enter(|| (app.boot)());

    // Process boot task (no UIs exist yet, so sync actions are discarded)
    let mut pending_regions: Vec<(
        SurfaceId,
        Option<Vec<crate::task::InputRegionRect>>
    )> = Vec::new();
    let mut pending_creations: Vec<(SurfaceId, LayerShellSettings)> = Vec::new();
    let _ = process_task(
        boot_task,
        &mut wl_state,
        &mut runtime,
        &mut pending_creations,
            &mut pending_regions,
        &qh,
        &exit_flag,
        &ping,
    );

    // Create surfaces requested during boot
    flush_pending_creations(&mut wl_state, &mut pending_creations, &mut pending_regions, &qh);

    // Roundtrip so new surfaces get configured
    event_queue
        .roundtrip(&mut wl_state)
        .map_err(|e| Error::EventLoop(e.to_string()))?;

    // Create iced rendering surfaces for everything registered
    sync_iced_surfaces(&wl_state, &mut compositor, &mut iced_surfaces, 1.0);

    event_loop
        .handle()
        .insert_source(ping_source, |(), (), _| {})
        .map_err(|e| Error::EventLoop(e.to_string()))?;
    WaylandSource::new(conn, event_queue)
        .insert(event_loop.handle())
        .map_err(|e| Error::EventLoop(e.to_string()))?;

    let mut running = true;

    let mut user_interfaces = ManuallyDrop::new(build_user_interfaces(
        &app.view,
        &user_state,
        &mut iced_surfaces,
        &mut renderer,
    ));

    let mut runtime_messages: Vec<Message> = Vec::new();
    let mut surface_events: HashMap<SurfaceId, Vec<iced_core::Event>> = HashMap::new();
    let mut all_messages: Vec<Message> = Vec::new();
    let mut surface_ids: Vec<SurfaceId> = Vec::new();
    let mut pending_creations: Vec<(SurfaceId, LayerShellSettings)> = Vec::new();

    let mut first_frame = true;
    while running {
        let timeout = if first_frame {
            first_frame = false;
            Some(std::time::Duration::ZERO)
        } else {
            None
        };
        event_loop
            .dispatch(timeout, &mut wl_state)
            .map_err(|e| Error::EventLoop(e.to_string()))?;

        if exit_flag.load(Ordering::Relaxed) {
            break;
        }

        for id in wl_state.surfaces_need_redraw.drain() {
            if let Some(iced_s) = iced_surfaces.get_mut(&id) {
                iced_s.needs_redraw = true;
            }
        }

        for closed_id in wl_state.closed_surfaces.drain(..) {
            user_interfaces.remove(&closed_id);
            iced_surfaces.remove(&closed_id);
            if let Some(wl_surface) = wl_state.surface_id_map.remove(&closed_id) {
                wl_state.surfaces.remove(&wl_surface);
            }
            if wl_state.pointer_surface == Some(closed_id) {
                wl_state.pointer_surface = None;
            }
            if wl_state.keyboard_focus == Some(closed_id) {
                wl_state.keyboard_focus = None;
            }
            wl_state
                .touch_fingers
                .retain(|_, (sid, _)| *sid != closed_id);
        }

        runtime_messages.clear();
        while let Ok(action) = runtime_receiver.try_recv() {
            run_action(
                action,
                &mut runtime_messages,
                &mut clipboard,
                &mut user_interfaces,
                &mut renderer,
                &mut compositor,
                &mut iced_surfaces,
                &exit_flag,
                &ping,
            );
        }

        if let Some(ref sub_fn) = app.subscription_fn {
            let subscription = runtime.enter(|| sub_fn(&user_state)).map(Action::Output);
            let recipes = iced_futures::subscription::into_recipes(subscription);
            runtime.track(recipes);
        }

        crate::output_subscription::push_events(mem::take(&mut wl_state.output_events));

        let app_scale = app.scale_factor_fn.as_ref().map_or(1.0, |f| f(&user_state)) as f32;

        for data in wl_state.surfaces.values() {
            if let Some(iced) = iced_surfaces.get_mut(&data.id) {
                let (sw, sh) = data.size;
                if sw > 0 && sh > 0 {
                    let monitor_scale = data.scale_factor.max(1) as u32;
                    let phys_w = sw * monitor_scale.max(1);
                    let phys_h = sh * monitor_scale.max(1);
                    let combined_scale = data.scale_factor as f32 * app_scale;
                    let new_vp =
                        Viewport::with_physical_size(Size::new(phys_w, phys_h), combined_scale);
                    if iced.viewport.physical_size() != new_vp.physical_size()
                        || (iced.viewport.scale_factor() - new_vp.scale_factor()).abs()
                            > f32::EPSILON
                    {
                        compositor.configure_surface(&mut iced.surface, phys_w, phys_h);
                        iced.viewport = new_vp;
                        iced.needs_redraw = true;
                    }
                }
            }
        }

        // Create iced rendering surfaces for newly configured wayland surfaces
        sync_iced_surfaces(&wl_state, &mut compositor, &mut iced_surfaces, app_scale);

        let pending_events = mem::take(&mut wl_state.pending_events);
        let scale = |p: iced_core::Point| iced_core::Point::new(p.x / app_scale, p.y / app_scale);
        surface_events.clear();
        for (sid, event) in pending_events {
            let event = match event {
                iced_core::Event::Mouse(iced_core::mouse::Event::CursorMoved { position }) => {
                    iced_core::Event::Mouse(iced_core::mouse::Event::CursorMoved {
                        position: scale(position),
                    })
                }
                iced_core::Event::Touch(iced_core::touch::Event::FingerPressed {
                    id,
                    position,
                }) => iced_core::Event::Touch(iced_core::touch::Event::FingerPressed {
                    id,
                    position: scale(position),
                }),
                iced_core::Event::Touch(iced_core::touch::Event::FingerMoved { id, position }) => {
                    iced_core::Event::Touch(iced_core::touch::Event::FingerMoved {
                        id,
                        position: scale(position),
                    })
                }
                iced_core::Event::Touch(iced_core::touch::Event::FingerLifted { id, position }) => {
                    iced_core::Event::Touch(iced_core::touch::Event::FingerLifted {
                        id,
                        position: scale(position),
                    })
                }
                iced_core::Event::Touch(iced_core::touch::Event::FingerLost { id, position }) => {
                    iced_core::Event::Touch(iced_core::touch::Event::FingerLost {
                        id,
                        position: scale(position),
                    })
                }
                other => other,
            };
            surface_events.entry(sid).or_default().push(event);
        }

        let theme = app
            .theme_fn
            .as_ref()
            .map_or(Theme::CatppuccinMocha, |f| f(&user_state));

        all_messages.clear();
        all_messages.append(&mut runtime_messages);
        let has_runtime_messages = !all_messages.is_empty();

        // Update persistent UIs with pending events
        surface_ids.clear();
        surface_ids.extend(iced_surfaces.keys().copied());
        for surface_id in &surface_ids {
            let events = surface_events.remove(surface_id).unwrap_or_default();
            if events.is_empty() && !has_runtime_messages {
                continue;
            }

            let Some(ui) = user_interfaces.get_mut(surface_id) else {
                continue;
            };

            let cursor = scaled_cursor(&wl_state, *surface_id, app_scale);

            let (ui_state, statuses) = ui.update(
                &events,
                cursor,
                &mut renderer,
                &mut clipboard,
                &mut all_messages,
            );

            for (event, status) in events.iter().zip(statuses) {
                runtime.broadcast(iced_futures::subscription::Event::Interaction {
                    window: (*surface_id).into(),
                    event: event.clone(),
                    status,
                });
            }

            match ui_state {
                iced_runtime::user_interface::State::Updated {
                    redraw_request,
                    mouse_interaction,
                    ..
                } => {
                    wl_state.set_cursor_shape(mouse_interaction, &qh);
                    if !matches!(redraw_request, iced_core::window::RedrawRequest::Wait)
                        && let Some(s) = iced_surfaces.get_mut(surface_id)
                    {
                        s.needs_redraw = true;
                    }
                }
                iced_runtime::user_interface::State::Outdated => {
                    if let Some(s) = iced_surfaces.get_mut(surface_id) {
                        s.needs_redraw = true;
                    }
                }
            }
        }

        pending_creations.clear();
        if !all_messages.is_empty() {
            // Drop all UIs before mutating state (same as iced_winit)
            let caches: HashMap<SurfaceId, user_interface::Cache> =
                ManuallyDrop::into_inner(user_interfaces)
                    .into_iter()
                    .map(|(id, ui)| (id, ui.into_cache()))
                    .collect();

            let mut sync_actions = Vec::new();
            for message in all_messages.drain(..) {
                let task = runtime.enter(|| (app.update)(&mut user_state, message));
                sync_actions.extend(process_task(
                    task,
                    &mut wl_state,
                    &mut runtime,
                    &mut pending_creations,
                    &mut pending_regions,
                    &qh,
                    &exit_flag,
                    &ping,
                ));
            }

            // Restore caches into iced_surfaces for rebuild
            for (id, cache) in caches {
                if let Some(iced_s) = iced_surfaces.get_mut(&id) {
                    iced_s.cache = Some(cache);
                }
            }

            // Rebuild all UIs with new state
            user_interfaces = ManuallyDrop::new(build_user_interfaces(
                &app.view,
                &user_state,
                &mut iced_surfaces,
                &mut renderer,
            ));

            // Process synchronous actions (widget ops, clipboard, etc.)
            // after UIs are rebuilt, matching iced_winit's pattern.
            for action in sync_actions {
                run_action(
                    action,
                    &mut runtime_messages,
                    &mut clipboard,
                    &mut user_interfaces,
                    &mut renderer,
                    &mut compositor,
                    &mut iced_surfaces,
                    &exit_flag,
                    &ping,
                );
            }
        }

        // Flush deferred touch finger removals now that events are processed
        for id in wl_state.pending_finger_removals.drain(..) {
            wl_state.touch_fingers.remove(&id);
        }

        // Create newly requested surfaces
        flush_pending_creations(&mut wl_state, &mut pending_creations, &mut pending_regions, &qh);
        sync_iced_surfaces(&wl_state, &mut compositor, &mut iced_surfaces, app_scale);

        // Build UIs for newly created surfaces
        {
            let new_ids: Vec<SurfaceId> = iced_surfaces
                .keys()
                .filter(|id| !user_interfaces.contains_key(id))
                .copied()
                .collect();
            for id in new_ids {
                let ui = build_single_ui(
                    &*app.view,
                    &user_state,
                    id,
                    &mut iced_surfaces,
                    &mut renderer,
                );
                user_interfaces.insert(id, ui);
            }
        }

        // Draw and present surfaces that need redraw.
        // Retries up to 3 times if RedrawRequested produces new messages.
        let mut redraw_count = 0u32;
        loop {
            surface_ids.clear();
            surface_ids.extend(iced_surfaces.keys().copied());
            let message_count_before_redraw = all_messages.len();

            for surface_id in &surface_ids {
                let iced_s = match iced_surfaces.get_mut(surface_id) {
                    Some(s) if s.needs_redraw => {
                        s.needs_redraw = false;
                        s
                    }
                    _ => continue,
                };

                let cursor = scaled_cursor(&wl_state, *surface_id, app_scale);

                let wl_surface = match wl_state.surface_id_map.get(surface_id) {
                    Some(wl) => wl.clone(),
                    None => continue,
                };
                let data = match wl_state.surfaces.get_mut(&wl_surface) {
                    Some(d) if d.configured && d.size.0 > 0 && d.size.1 > 0 => d,
                    _ => continue,
                };

                let Some(ui) = user_interfaces.get_mut(surface_id) else {
                    continue;
                };

                // RedrawRequested makes widgets commit visual state
                let redraw_event = [iced_core::Event::Window(
                    iced_core::window::Event::RedrawRequested(std::time::Instant::now()),
                )];
                let (ui_state, _statuses) = ui.update(
                    &redraw_event,
                    cursor,
                    &mut renderer,
                    &mut clipboard,
                    &mut all_messages,
                );

                // If the widget requested another frame (e.g. animation in progress),
                // schedule a full redraw for after the compositor acknowledges this frame.
                // This mirrors iced_winit where NextFrame maps to vsync-gated request_redraw.
                let wants_next_frame = matches!(
                    ui_state,
                    iced_runtime::user_interface::State::Updated {
                        redraw_request: iced_core::window::RedrawRequest::NextFrame,
                        ..
                    }
                );

                // Draw
                let style = iced_core::renderer::Style {
                    text_color: theme.palette().text,
                };
                ui.draw(&mut renderer, &theme, &style, cursor);

                // Present
                if data.frame_pending {
                    data.needs_rerender = true;
                } else {
                    let bg = iced_core::Color::TRANSPARENT;
                    let wl_surf = data.layer_surface.wl_surface();
                    wl_surf.frame(&qh, wl_surf.clone());
                    data.frame_pending = true;

                    match compositor.present(
                        &mut renderer,
                        &mut iced_s.surface,
                        &iced_s.viewport,
                        bg,
                        || {},
                    ) {
                        Ok(()) => {
                            if wants_next_frame {
                                data.needs_rerender = true;
                            }
                        }
                        Err(iced_graphics::compositor::SurfaceError::OutOfMemory) => {
                            running = false;
                        }
                        Err(
                            err @ (iced_graphics::compositor::SurfaceError::Outdated
                            | iced_graphics::compositor::SurfaceError::Lost),
                        ) => {
                            // The wgpu surface can become invalid at any time
                            // (compositor restart, output change); reconfigure or
                            // recreate it and schedule a redraw, otherwise we'd
                            // stay frozen on a dead surface.
                            let size = iced_s.viewport.physical_size();
                            let recovered =
                                if matches!(err, iced_graphics::compositor::SurfaceError::Lost) {
                                    match WaylandWindow::new(
                                        wl_state.display_ptr,
                                        data.layer_surface.wl_surface(),
                                    ) {
                                        Some(window) => {
                                            iced_s.surface = compositor.create_surface(
                                                window,
                                                size.width,
                                                size.height,
                                            );
                                            true
                                        }
                                        None => false,
                                    }
                                } else {
                                    compositor.configure_surface(
                                        &mut iced_s.surface,
                                        size.width,
                                        size.height,
                                    );
                                    true
                                };
                            data.frame_pending = false;
                            // Only reschedule on recovery — otherwise we'd loop
                            // presenting on a dead surface and burn CPU.
                            if recovered {
                                iced_s.needs_redraw = true;
                            } else {
                                log::error!(
                                    "failed to recreate wgpu surface after Lost; \
                                     surface left dead"
                                );
                            }
                        }
                        Err(_) => {
                            data.frame_pending = false;
                        }
                    }
                }
            }

            if all_messages.len() == message_count_before_redraw {
                break;
            }

            if redraw_count >= 2 {
                log::warn!("More than 3 consecutive RedrawRequested cycles produced messages");
                break;
            }
            redraw_count += 1;

            // Process messages produced during RedrawRequested
            pending_creations.clear();
            let caches: HashMap<SurfaceId, user_interface::Cache> =
                ManuallyDrop::into_inner(user_interfaces)
                    .into_iter()
                    .map(|(id, ui)| (id, ui.into_cache()))
                    .collect();

            let mut sync_actions = Vec::new();
            for message in all_messages.drain(..) {
                let task = runtime.enter(|| (app.update)(&mut user_state, message));
                sync_actions.extend(process_task(
                    task,
                    &mut wl_state,
                    &mut runtime,
                    &mut pending_creations,
                    &mut pending_regions,
                    &qh,
                    &exit_flag,
                    &ping,
                ));
            }

            for (id, cache) in caches {
                if let Some(iced_s) = iced_surfaces.get_mut(&id) {
                    iced_s.cache = Some(cache);
                }
            }

            user_interfaces = ManuallyDrop::new(build_user_interfaces(
                &app.view,
                &user_state,
                &mut iced_surfaces,
                &mut renderer,
            ));

            for action in sync_actions {
                run_action(
                    action,
                    &mut runtime_messages,
                    &mut clipboard,
                    &mut user_interfaces,
                    &mut renderer,
                    &mut compositor,
                    &mut iced_surfaces,
                    &exit_flag,
                    &ping,
                );
            }

            for id in wl_state.pending_finger_removals.drain(..) {
                wl_state.touch_fingers.remove(&id);
            }

            flush_pending_creations(&mut wl_state, &mut pending_creations, &mut pending_regions, &qh);
            sync_iced_surfaces(&wl_state, &mut compositor, &mut iced_surfaces, app_scale);

            // Mark all surfaces for redraw
            for s in iced_surfaces.values_mut() {
                s.needs_redraw = true;
            }
        }
    }

    Ok(())
}

/// Process a single runtime Action synchronously on the main loop.
#[allow(clippy::too_many_arguments)]
fn run_action<Message: std::fmt::Debug>(
    action: Action<Message>,
    messages: &mut Vec<Message>,
    clipboard: &mut WaylandClipboard,
    user_interfaces: &mut HashMap<
        SurfaceId,
        UserInterface<'_, Message, Theme, iced_renderer::Renderer>,
    >,
    renderer: &mut iced_renderer::Renderer,
    compositor: &mut Compositor,
    iced_surfaces: &mut HashMap<SurfaceId, IcedSurface>,
    exit_flag: &Arc<AtomicBool>,
    ping: &calloop::ping::Ping,
) {
    match action {
        Action::Output(msg) => {
            messages.push(msg);
        }
        Action::Clipboard(action) => {
            use iced_core::clipboard::Clipboard as _;
            match action {
                iced_runtime::clipboard::Action::Read { target, channel } => {
                    let contents = clipboard.read(target);
                    let _ = channel.send(contents);
                }
                iced_runtime::clipboard::Action::Write { target, contents } => {
                    clipboard.write(target, contents);
                }
            }
        }
        Action::Widget(mut operation) => loop {
            for ui in user_interfaces.values_mut() {
                ui.operate(renderer, operation.as_mut());
            }
            match operation.finish() {
                iced_core::widget::operation::Outcome::Chain(next) => {
                    operation = next;
                }
                _ => break,
            }
        },
        Action::LoadFont { bytes, channel } => {
            compositor.load_font(bytes);
            let _ = channel.send(Ok(()));
        }
        Action::Reload => {
            for iced_s in iced_surfaces.values_mut() {
                iced_s.needs_redraw = true;
                iced_s.cache = None;
            }
        }
        Action::Exit => {
            exit_flag.store(true, Ordering::Relaxed);
            ping.ping();
        }
        _ => {
            // Window, System, Image actions are not applicable to layer shell
        }
    }
}

/// Route a [`Task`] to the appropriate handler: layer shell commands go to
/// [`apply_layer_shell_command`], iced tasks are spawned on the async runtime.
///
/// Immediately-ready actions (widget operations, clipboard, etc.) are polled
/// synchronously and returned to the caller for processing — matching
/// `iced_winit`'s behaviour. Only the async remainder is handed to `runtime.run()`.
fn process_task<M: Send + Clone + 'static>(
    task: Task<M>,
    wl_state: &mut WaylandState,
    runtime: &mut iced_futures::Runtime<
        iced_futures::backend::default::Executor,
        WakeupSender<M>,
        Action<M>,
    >,
    pending_creations: &mut Vec<(SurfaceId, LayerShellSettings)>,
    pending_regions: &mut Vec<(SurfaceId, Option<Vec<crate::task::InputRegionRect>>)>,
    qh: &wayland_client::QueueHandle<WaylandState>,
    exit_flag: &Arc<AtomicBool>,
    ping: &calloop::ping::Ping,
) -> Vec<Action<M>> {
    let mut actions = Vec::new();
    match task {
        Task::LayerShell(cmd) => {
            apply_layer_shell_command(cmd, wl_state, pending_creations, pending_regions, qh);
        }
        Task::Iced(iced_task) => {
            if let Some(mut stream) = iced_runtime::task::into_stream(iced_task) {
                let waker = std::task::Waker::noop();
                let mut cx = Context::from_waker(waker);

                loop {
                    match runtime.enter(|| stream.poll_next_unpin(&mut cx)) {
                        Poll::Ready(Some(action)) => {
                            if matches!(&action, Action::Exit) {
                                exit_flag.store(true, Ordering::Relaxed);
                                ping.ping();
                            }
                            actions.push(action);
                        }
                        Poll::Ready(None) => break,
                        Poll::Pending => {
                            let exit_flag = exit_flag.clone();
                            let ping = ping.clone();
                            let stream = stream.map(move |action| {
                                if matches!(&action, Action::Exit) {
                                    exit_flag.store(true, Ordering::Relaxed);
                                    ping.ping();
                                }
                                action
                            });
                            runtime.run(Box::pin(stream));
                            break;
                        }
                    }
                }
            }
        }
        Task::Batch(tasks) => {
            for t in tasks {
                actions.extend(process_task(
                    t,
                    wl_state,
                    runtime,
                    pending_creations,
                    pending_regions,
                    qh,
                    exit_flag,
                    ping,
                ));
            }
        }
    }
    actions
}
