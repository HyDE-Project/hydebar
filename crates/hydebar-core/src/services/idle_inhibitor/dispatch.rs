//! Wayland registry state and dispatch handlers for the idle inhibitor.

use log::{debug, warn};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    protocol::{
        wl_compositor::WlCompositor,
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface
    }
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
    zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1
};

#[derive(Default)]
pub(super) struct IdleInhibitorManagerData {
    pub(super) compositor:           Option<(WlCompositor, u32)>,
    pub(super) surface:              Option<WlSurface>,
    pub(super) idle_manager:         Option<(ZwpIdleInhibitManagerV1, u32)>,
    pub(super) idle_inhibitor_state: Option<ZwpIdleInhibitorV1>
}

impl Dispatch<WlRegistry, ()> for IdleInhibitorManagerData {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        handle: &wayland_client::QueueHandle<Self>
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version
            } => {
                if interface == WlCompositor::interface().name && state.compositor.is_none() {
                    debug!(target: "IdleInhibitor::WlRegistry::Event::Global", "Adding Compositor with name {name} and version {version}");
                    let compositor: WlCompositor = proxy.bind(name, version, handle, ());

                    state.surface = Some(compositor.create_surface(handle, ()));
                    state.compositor = Some((compositor, name));
                } else if interface == ZwpIdleInhibitManagerV1::interface().name
                    && state.idle_manager.is_none()
                {
                    debug!(target: "IdleInhibitor::WlRegistry::Event::Global", "Adding IdleInhibitManager with name {name} and version {version}");
                    state.idle_manager = Some((proxy.bind(name, version, handle, ()), name));
                }
            }
            wl_registry::Event::GlobalRemove {
                name
            } => match &state.compositor {
                Some((_, compositor_name)) => {
                    if name == *compositor_name {
                        warn!(target: "IdleInhibitor::GlobalRemove", "Compositor was removed!");

                        state.compositor = None;
                        state.surface = None;
                    }
                }
                _ => {
                    if let Some((_, idle_manager_name)) = &state.idle_manager
                        && name == *idle_manager_name
                    {
                        warn!(target: "IdleInhibitor::GlobalRemove", "IdleInhibitManager was removed!");

                        state.idle_manager = None;
                    }
                }
            },
            _ => {}
        }
    }
}

/// This interface has no events.
impl Dispatch<WlCompositor, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>
    ) {
    }
}

impl Dispatch<WlSurface, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>
    ) {
    }
}

/// This interface has no events.
impl Dispatch<ZwpIdleInhibitManagerV1, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitManagerV1,
        _event: <ZwpIdleInhibitManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>
    ) {
    }
}

/// This interface has no events.
impl Dispatch<ZwpIdleInhibitorV1, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitorV1,
        _event: <ZwpIdleInhibitorV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>
    ) {
    }
}
