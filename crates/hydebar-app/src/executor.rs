//! Executor handing iced the runtime the bar already owns.
//!
//! `iced` builds a fresh multi-threaded tokio runtime for its own executor by
//! default, so a bar that also builds one ends up with two work-stealing pools
//! sized to the CPU count. The bar never computes anything heavy — it parks on
//! D-Bus, Wayland and Hyprland sockets — so both pools sit idle while their
//! workers still take part in every wakeup. Pointing iced at the runtime built
//! in `main` collapses the two pools into one deliberately sized pool.

use std::{future::Future, io, sync::OnceLock};

use tokio::runtime::Handle;

static SHARED_HANDLE: OnceLock<Handle> = OnceLock::new();

/// Publishes the handle [`SharedRuntime::new`] picks up.
///
/// `iced` constructs the executor itself and offers no place to pass state, so
/// the handle travels through a process global. Call this before handing the
/// daemon to `iced`.
pub(crate) fn install(handle: Handle) {
    let _ = SHARED_HANDLE.set(handle);
}

/// Executor forwarding every `iced` task onto the runtime built in `main`.
pub(crate) struct SharedRuntime(Handle);

impl iced::Executor for SharedRuntime {
    fn new() -> Result<Self, io::Error> {
        SHARED_HANDLE
            .get()
            .cloned()
            .map(Self)
            .ok_or_else(|| io::Error::other("the shared runtime handle was never installed"))
    }

    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        drop(self.0.spawn(future));
    }

    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = self.0.enter();
        f()
    }

    fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        self.0.block_on(future)
    }
}
