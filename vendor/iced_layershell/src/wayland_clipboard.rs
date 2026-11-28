use iced_core::clipboard::Kind;

pub(crate) struct WaylandClipboard {
    clipboard: smithay_clipboard::Clipboard,
}

impl WaylandClipboard {
    /// # Safety
    /// `display_ptr` must be a valid `*mut wl_display`.
    pub unsafe fn new(display_ptr: *mut std::ffi::c_void) -> Self {
        Self {
            // Safety: display_ptr is valid for the lifetime of the Wayland connection
            clipboard: unsafe { smithay_clipboard::Clipboard::new(display_ptr.cast()) },
        }
    }

    pub fn write_clipboard(&mut self, kind: Kind, contents: String) {
        match kind {
            Kind::Standard => self.clipboard.store(contents),
            Kind::Primary => self.clipboard.store_primary(contents),
        }
    }
}

impl iced_core::clipboard::Clipboard for WaylandClipboard {
    fn read(&self, kind: Kind) -> Option<String> {
        let result = match kind {
            Kind::Standard => self.clipboard.load(),
            Kind::Primary => self.clipboard.load_primary(),
        };
        result.ok().filter(|s| !s.is_empty())
    }

    fn write(&mut self, kind: Kind, contents: String) {
        self.write_clipboard(kind, contents);
    }
}
