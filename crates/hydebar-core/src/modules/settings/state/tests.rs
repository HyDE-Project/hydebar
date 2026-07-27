// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use std::{
        num::NonZeroUsize,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering}
        }
    };

    use futures::future;
    use tokio::runtime::Runtime;

    use super::*;
    use crate::{event_bus::EventBus, modules::Module};

    #[test]
    fn register_spawns_event_forwarders() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut settings = Settings::default();

        <Settings as Module<Message>>::register(&mut settings, &ctx, ())
            .expect("register should succeed");

        assert!(settings.sender.is_some());
        assert!(settings.runtime.is_some());
        assert_eq!(settings.tasks.len(), 5);

        for task in settings.tasks.drain(..) {
            task.abort();
        }
    }

    #[test]
    #[ignore = "Timing-sensitive test - needs rework"]
    fn register_aborts_existing_tasks() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut settings = Settings::default();

        let cancelled = Arc::new(AtomicBool::new(false));
        let guard_flag = Arc::clone(&cancelled);

        settings.tasks.push(runtime.spawn(async move {
            struct CancelGuard(Arc<AtomicBool>);

            impl Drop for CancelGuard {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::SeqCst);
                }
            }

            let _guard = CancelGuard(guard_flag);

            future::pending::<()>().await;
        }));

        <Settings as Module<Message>>::register(&mut settings, &ctx, ())
            .expect("register should succeed");

        assert!(cancelled.load(Ordering::SeqCst));

        for task in settings.tasks.drain(..) {
            task.abort();
        }
    }
}
