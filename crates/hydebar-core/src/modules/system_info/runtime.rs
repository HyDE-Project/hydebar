use std::{sync::Arc, time::Duration};

use log::error;
use tokio::{
    sync::Notify,
    task::JoinHandle,
    time::{MissedTickBehavior, interval}
};

use super::{Message, data::SystemInfoData};
use crate::{ModuleContext, ModuleEventSender, modules::system_info::SystemInfoSampler};

/// Interval between system information refresh ticks.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// Cadence while the user is looking at the module or its window.
pub const ATTENDED_INTERVAL: Duration = Duration::from_secs(1);

/// Pause between priming the counters and the first published sample.
///
/// The processor load is a difference between two readings, so the very
/// first reading of a fresh sampler is not a load at all — publishing
/// it painted `0%` until the second tick five seconds later. Long
/// enough for the kernel counters to move, short enough that the bar
/// shows honest numbers as soon as it is up.
pub const WARMUP: Duration = Duration::from_millis(300);

/// Source of the metrics the polling task publishes.
///
/// Sampling sits behind a trait so the schedule and the deduplication can
/// be exercised against a scripted source rather than against whatever
/// the host machine happens to be doing.
pub trait MetricSource: Send + 'static {
    /// Capture the current readouts.
    fn sample(&mut self) -> SystemInfoData;
}

impl MetricSource for SystemInfoSampler {
    fn sample(&mut self) -> SystemInfoData {
        self.sample_scoped()
    }
}

/// Manages the background polling task responsible for refreshing system
/// metrics.
#[derive(Default)]
pub struct PollingTask {
    handle: Option<JoinHandle<()>>,
    poke:   Option<Arc<Notify>>
}

impl PollingTask {
    /// Create a new polling task manager with no active background work.
    pub const fn new() -> Self {
        Self {
            handle: None,
            poke:   None
        }
    }

    /// Abort any in-flight polling task.
    pub fn abort(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        self.poke = None;
    }

    /// Asks the running task for a sample right now.
    ///
    /// The window and the hover hints read the cached sample, and a
    /// cache refreshed only on the resting cadence can be seconds
    /// old the moment somebody looks at it. The sample still
    /// happens on the task's own thread, and one whose readouts
    /// match the ones on screen is still dropped, so a poke costs
    /// a repaint only when something actually changed.
    pub fn poke(&self) {
        if let Some(poke) = self.poke.as_ref() {
            poke.notify_one();
        }
    }

    /// Spawn a periodic refresh loop bound to the provided runtime context.
    ///
    /// `preferred_gpu` pins the graphics device the readings come from on a
    /// machine that has more than one; leaving it out lets the panel
    /// choose.
    pub fn spawn(
        &mut self,
        ctx: &ModuleContext,
        sender: ModuleEventSender<Message>,
        preferred_gpu: Option<&str>,
        full: bool
    ) {
        let mut sampler = SystemInfoSampler::new();
        sampler.prefer_gpu(preferred_gpu);

        if !full {
            sampler.only_cpu_and_memory();
        }

        self.spawn_from(ctx, sender, sampler);
    }

    /// Spawn the refresh loop against an explicit metric source.
    ///
    /// Sampling happens here rather than in the module update so a tick
    /// whose readouts match the ones already on screen can be
    /// dropped: every event the module publishes rebuilds and
    /// repaints every surface the bar owns, and an idle machine
    /// reports the same numbers tick after tick.
    ///
    /// The first sample is taken and thrown away: it only primes the
    /// counters the loads are differences of. The first published one
    /// follows [`WARMUP`] later, so the bar shows honest numbers at
    /// once instead of a zero it would keep for a whole interval.
    pub fn spawn_from<S>(
        &mut self,
        ctx: &ModuleContext,
        sender: ModuleEventSender<Message>,
        mut source: S
    ) where
        S: MetricSource
    {
        self.abort();

        let poke = Arc::new(Notify::new());
        let poked = Arc::clone(&poke);

        let handle = ctx.runtime_handle().spawn(async move {
            let mut ticker = interval(REFRESH_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
            let _ = ticker.tick().await;

            let Ok(mut source) =
                tokio::task::spawn_blocking(move || {
                    let _ = source.sample();
                    source
                })
                .await
            else {
                error!("system info priming sample panicked; sampling stops");
                return;
            };

            tokio::time::sleep(WARMUP).await;
            let mut published: Option<SystemInfoData> = None;

            loop {
                let Ok((returned, sample)) = tokio::task::spawn_blocking(move || {
                    let sample = source.sample();
                    (source, sample)
                })
                .await
                else {
                    error!("system info sampling panicked; sampling stops");
                    return;
                };
                source = returned;

                if published
                    .as_ref()
                    .is_none_or(|previous| !previous.renders_same_as(&sample))
                {
                    published = Some(sample.clone());

                    if let Err(err) = sender.try_send(Message::Sampled(sample)) {
                        error!("failed to publish system info refresh: {err}");
                    }
                }

                tokio::select! {
                    _ = ticker.tick() => {}
                    () = poked.notified() => {}
                }
            }
        });

        self.handle = Some(handle);
        self.poke = Some(poke);
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
            ..SystemInfoData::default()
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

    /// Waits for the next published event without touching the clock.
    ///
    /// Sampling runs on a real thread of the blocking pool, so the event
    /// lands a moment after the paused clock says it is due; spinning on
    /// yields, with a real nap now and then, lets that thread finish.
    async fn next_event(
        receiver: &mut crate::event_bus::EventReceiver
    ) -> Option<BusEvent> {
        for spin in 0..10_000u32 {
            if let Some(event) = receiver.try_recv().expect("bus read") {
                return Some(event);
            }

            if spin % 64 == 63 {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            yield_now().await;
        }

        None
    }

    /// Walks the paused clock forward until the next event is published.
    ///
    /// The sampler re-arms its timers from whatever the clock says when a
    /// blocking sample lands, so one big advance from the test can slip
    /// past a deadline registered a moment later; small steps with a spin
    /// between them reach the deadline wherever it settled.
    async fn next_event_advancing(
        receiver: &mut crate::event_bus::EventReceiver,
        step: std::time::Duration
    ) -> Option<BusEvent> {
        for _ in 0..32u32 {
            for spin in 0..256u32 {
                if let Some(event) = receiver.try_recv().expect("bus read") {
                    return Some(event);
                }

                if spin % 64 == 63 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }

                yield_now().await;
            }

            advance(step).await;
        }

        None
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

        let event = next_event_advancing(&mut receiver, WARMUP / 2).await;
        expect_cpu_usage(event, 1);

        let event = next_event_advancing(&mut receiver, REFRESH_INTERVAL / 2).await;
        expect_cpu_usage(event, 2);
    }

    #[tokio::test(start_paused = true)]
    async fn the_priming_sample_is_never_published() {
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

        let event = next_event_advancing(&mut receiver, WARMUP / 2).await;
        expect_cpu_usage(event, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn unchanged_readouts_publish_nothing() {
        let (ctx, bus) = module_context();
        let mut polling = PollingTask::default();
        let mut receiver = bus.receiver();

        let sender = ctx.module_sender(ModuleEvent::SystemInfo);
        polling.spawn_from(&ctx, sender, SteadyLoad);
        yield_now().await;

        let first = next_event_advancing(&mut receiver, WARMUP / 2).await;
        expect_cpu_usage(first, 7);

        for _ in 0..4u32 {
            advance(REFRESH_INTERVAL).await;
            for _ in 0..64u32 {
                yield_now().await;
            }
        }

        assert!(
            receiver
                .try_recv()
                .expect("queue state after steady samples")
                .is_none()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_poke_samples_without_waiting_for_the_tick() {
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

        let first = next_event_advancing(&mut receiver, WARMUP / 2).await;
        expect_cpu_usage(first, 1);

        polling.poke();

        let poked = next_event(&mut receiver).await;
        expect_cpu_usage(poked, 2);
    }

    #[test]
    fn a_poke_without_a_running_task_is_a_no_op() {
        PollingTask::default().poke();
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

        let first = next_event_advancing(&mut receiver, WARMUP / 2).await;
        expect_cpu_usage(first, 1);
        assert!(receiver.try_recv().expect("drain first refresh").is_none());

        polling.spawn_from(
            &ctx,
            sender,
            RisingLoad {
                next: 100
            }
        );
        yield_now().await;

        let second = next_event_advancing(&mut receiver, WARMUP / 2).await;
        expect_cpu_usage(second, 101);
        assert!(receiver.try_recv().expect("no duplicate refresh").is_none());
    }
}
