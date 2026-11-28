//! Raw window handle wrapper for iced graphics integration.
//!
//! Provides raw-window-handle trait implementations to bridge
//! between Wayland surfaces and iced's graphics compositor.

use std::ptr::NonNull;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use wayland_client::Proxy;
use wayland_client::protocol::wl_surface::WlSurface;

/// Wrapper for Wayland display and surface pointers.
///
/// Implements `HasDisplayHandle` and `HasWindowHandle` traits required
/// by iced's graphics compositor for surface creation.
#[derive(Clone)]
pub(crate) struct WaylandWindow {
    display: NonNull<std::ffi::c_void>,
    surface: NonNull<std::ffi::c_void>,
}

// Safety: the pointers remain valid for the lifetime of the wayland connection/surface
unsafe impl Send for WaylandWindow {}
unsafe impl Sync for WaylandWindow {}

impl WaylandWindow {
    /// Create a new `WaylandWindow` from display pointer and surface.
    ///
    /// # Safety
    /// The `display_ptr` must be valid for the lifetime of the Wayland connection,
    /// and the surface must be valid for the lifetime of the surface.
    pub fn new(display_ptr: *mut std::ffi::c_void, surface: &WlSurface) -> Option<Self> {
        let display = NonNull::new(display_ptr)?;
        let surface_ptr = Proxy::id(surface).as_ptr().cast::<std::ffi::c_void>();
        let surface = NonNull::new(surface_ptr)?;
        Some(Self { display, surface })
    }
}

impl HasDisplayHandle for WaylandWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let handle = WaylandDisplayHandle::new(self.display);
        let raw = RawDisplayHandle::Wayland(handle);
        // Safety: the display pointer is valid for the lifetime of the connection
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

impl HasWindowHandle for WaylandWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = WaylandWindowHandle::new(self.surface);
        let raw = RawWindowHandle::Wayland(handle);
        // Safety: the surface pointer is valid for the lifetime of the surface
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}
