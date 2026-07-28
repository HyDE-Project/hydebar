use std::time::Duration;

use log::error;
use tokio::{
    task::JoinHandle,
    time::{MissedTickBehavior, interval}
};

use super::{Message, data::SystemInfoData};
use crate::{ModuleContext, ModuleEventSender, modules::system_info::SystemInfoSampler};

/// Interval between system information refresh ticks.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// Source of the metrics the polling task publishes.
///
/// Sampling sits behind a trait so the schedule and the deduplication can be
/// exercised against a scripted source rather than against whatever the host
/// machine happens to be doing.
pub trait MetricSource: Send + 'static {
    /// Capture the current readouts.
    fn sample(&mut self) -> SystemInfoData;
}

impl MetricSource for SystemInfoSampler {
    fn sample(&mut self) -> SystemInfoData {
        self.sample_with_extras()
    }
}

/// Manages the background polling task responsible for refreshing system
/// metrics.
#[derive(Default)]
pub struct PollingTask {
    handle: Option<JoinHandle<()>>
}

impl PollingTask {
    /// Create a new polling task manager with no active background work.
    pub fn new() -> Self {
        Self {
            handle: None
        }
    }

    /// Abort any in-flight polling task.
    pub fn abort(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }

    /// Spawn a periodic refresh loop bound to the provided runtime context.
    pub fn spawn(&mut self, ctx: &ModuleContext, sender: ModuleEventSender<Message>) {
        self.spawn_from(ctx, sender, SystemInfoSampler::new());
    }

    /// Spawn the refresh loop against an explicit metric source.
    ///
    /// Sampling happens here rather than in the module update so a tick whose
    /// readouts match the ones already on screen can be dropped: every event
    /// the module publishes rebuilds and repaints every surface the bar owns,
    /// and an idle machine reports the same numbers tick after tick.
    pub fn spawn_from<S>(
        &mut self,
        ctx: &ModuleContext,
        sender: ModuleEventSender<Message>,
        mut source: S
    ) where
        S: MetricSource
    {
        self.abort();

        let handle = ctx.runtime_handle().spawn(async move {
            let mut ticker = interval(REFRESH_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
            let _ = ticker.tick().await;
            let mut published: Option<SystemInfoData> = None;

            loop {
                ticker.tick().await;

                let sample = source.sample();

                if published
                    .as_ref()
                    .is_some_and(|previous| previous.renders_same_as(&sample))
                {
                    continue;
                }

                published = Some(sample.clone());

                if let Err(err) = sender.try_send(Message::Sampled(sample)) {
                    error!("failed to publish system info refresh: {err}");
                }
            }
        });

        self.handle = Some(handle);
    }
}

impl Drop for PollingTask {
    fn drop(&mut self) {
        self.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use tokio::{task::yield_now, time::advance};

    use super::*;
    use crate::{
        ModuleContext,
        event_bus::{BusEvent, EventBus, ModuleEvent},
        modules::system_info::Message
    };

    fn module_context() -> (ModuleContext, EventBus) {
        let capacity = NonZeroUsize::new(16).expect("non-zero capacity");
        let bus = EventBus::new(capacity);
        let ctx = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        (ctx, bus)
    }

    /// Source reporting a CPU load that climbs by one on every sample.
    struct RisingLoad {
        next: u32
    }

    impl MetricSource for RisingLoad {
        fn sample(&mut self) -> SystemInfoData {
            let data = sample_with_cpu(self.next);
            self.next += 1;

            data
        }
    }

    /// Source reporting the same readouts forever, like an idle machine.
    struct SteadyLoad;

    impl MetricSource for SteadyLoad {
        fn sample(&mut self) -> SystemInfoData {
            sample_with_cpu(7)
        }
    }

    fn sample_with_cpu(cpu_usage: u32) -> SystemInfoData {
        SystemInfoData {
            cpu_usage,
            memory_usage: 42,
            memory_used: 1024,
            memory_swap_usage: 0,
            memory_swap_used: 0,
            temperature: None,
            disks: Vec::new(),
            network: None
        }
    }

    fn expect_cpu_usage(event: Option<BusEvent>, expected: u32) {
        match event {
            Some(BusEvent::Module(ModuleEvent::SystemInfo(Message::Sampled(data)))) => {
                assert_eq!(data.cpu_usage, expected);
            }
            other => panic!("unexpected event: {other:?}")
        }
    }

    #[tokio::test(start_paused = true)]
    async fn schedules_periodic_refreshes() {
        let (ctx, bus) = module_context();
        let mut polling = PollingTask::default();
        let mut receiver = bus.receiver();

        let sender = ctx.module_sender(ModuleEvent::SystemInfo);
        polling.spawn_from(
            &ctx,
            sender,
            RisingLoad {
                next: 0
            }
        );
        yield_now().await;

        assert!(receiver.try_recv().expect("initial queue state").is_none());

        advance(REFRESH_INTERVAL).await;
        yield_now().await;

        let event = receiver.try_recv().expect("queued refresh after interval");
        expect_cpu_usage(event, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn unchanged_readouts_publish_nothing() {
        let (ctx, bus) = module_context();
        let mut polling = PollingTask::default();
        let mut receiver = bus.receiver();

        let sender = ctx.module_sender(ModuleEvent::SystemInfo);
        polling.spawn_from(&ctx, sender, SteadyLoad);
        yield_now().await;

        advance(REFRESH_INTERVAL).await;
        yield_now().await;

        let first = receiver.try_recv().expect("first refresh after interval");
        expect_cpu_usage(first, 7);

        advance(REFRESH_INTERVAL).await;
        yield_now().await;
        advance(REFRESH_INTERVAL).await;
        yield_now().await;

        assert!(
            receiver
                .try_recv()
                .expect("queue state after steady samples")
                .is_none()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn respawn_replaces_previous_task() {
        let (ctx, bus) = module_context();
        let mut polling = PollingTask::default();
        let mut receiver = bus.receiver();

        let sender = ctx.module_sender(ModuleEvent::SystemInfo);
        polling.spawn_from(
            &ctx,
            sender.clone(),
            RisingLoad {
                next: 0
            }
        );
        yield_now().await;

        advance(REFRESH_INTERVAL).await;
        yield_now().await;

        let first = receiver.try_recv().expect("first refresh after interval");
        expect_cpu_usage(first, 0);
        assert!(receiver.try_recv().expect("drain first interval").is_none());

        polling.spawn_from(
            &ctx,
            sender,
            RisingLoad {
                next: 100
            }
        );
        yield_now().await;

        advance(REFRESH_INTERVAL).await;
        yield_now().await;

        let second = receiver.try_recv().expect("refresh after respawn");
        expect_cpu_usage(second, 100);
        assert!(receiver.try_recv().expect("no duplicate refresh").is_none());
    }
}
