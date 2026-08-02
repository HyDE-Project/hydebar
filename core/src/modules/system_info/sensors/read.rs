//! One pass over the chosen inputs: the files the panel settled on,
//! read through the reused buffer.

use log::warn;

use super::{GpuReadings, HardwareSensors, Input, Readings, hwmon, utility::Feed};

impl HardwareSensors {
    /// Latest readings, rebuilding the sensor set when it is due.
    pub fn read(&mut self) -> Readings {
        if self
            .discovered_at
            .is_none_or(|discovered| discovered.elapsed() >= super::DISCOVERY_INTERVAL)
        {
            self.discover();
        }

        let mut state = ReadState {
            failed:   false,
            reported: &mut self.reported
        };
        let cpu = read_input(self.cpu.as_ref(), &mut self.buffer, &mut state);

        let gpu = self.gpu.as_ref().map(|gpu| {
            let temperature = read_input(gpu.input.as_ref(), &mut self.buffer, &mut state);
            let awake = gpu.card.as_ref().is_none_or(|card| !card.is_asleep());
            let utilisation = awake
                .then(|| {
                    gpu.card
                        .as_ref()
                        .and_then(|card| card.utilisation(&mut self.buffer))
                })
                .flatten();
            let memory = awake
                .then(|| {
                    gpu.card
                        .as_ref()
                        .and_then(|card| card.memory(&mut self.buffer))
                })
                .flatten();

            GpuReadings {
                name: gpu.name.clone(),
                source: gpu.input.as_ref().map(Input::describe),
                vendor: gpu.vendor,
                placement: gpu.placement,
                temperature,
                utilisation,
                memory_used: memory.map(|(used, _)| used),
                memory_total: memory.map(|(_, total)| total)
            }
        });

        let failed = state.failed;
        let gpu = self
            .complete_with_utility(gpu)
            .filter(|gpu| !gpu.is_empty());

        if failed {
            self.discovered_at = None;
        } else {
            self.reported = false;
        }

        Readings {
            cpu,
            cpu_source: self.cpu.as_ref().map(Input::describe),
            gpu
        }
    }

    /// Fills in what the kernel does not publish from the vendor utility.
    fn complete_with_utility(&self, gpu: Option<GpuReadings>) -> Option<GpuReadings> {
        let mut gpu = gpu?;
        let Some(metrics) = self
            .utility
            .as_ref()
            .filter(|feed| feed.vendor() == gpu.vendor)
            .and_then(Feed::latest)
        else {
            return Some(gpu);
        };

        gpu.temperature = gpu.temperature.or(metrics.temperature);
        gpu.utilisation = gpu.utilisation.or(metrics.utilisation);
        gpu.memory_used = gpu.memory_used.or(metrics.memory_used);
        gpu.memory_total = gpu.memory_total.or(metrics.memory_total);

        Some(gpu)
    }
}

/// State one pass over the chosen inputs leaves behind.
struct ReadState<'a> {
    /// An input that was there at discovery no longer reads.
    failed:   bool,
    /// The failure has already been written to the log.
    reported: &'a mut bool
}

/// Reading behind an input, noting a failure for the caller.
///
/// A sensor that stops answering is written to the log once and then left
/// alone: the panel reads every few seconds, and a machine whose card was
/// unplugged would otherwise fill the journal with the same line. The next
/// successful pass clears the mark, so a fault that comes back is reported
/// again.
fn read_input(
    input: Option<&Input>,
    buffer: &mut String,
    state: &mut ReadState<'_>
) -> Option<i32> {
    let input = input?;

    match hwmon::read_temperature(&input.path, buffer) {
        Ok(temperature) => Some(temperature),
        Err(error) => {
            if !*state.reported {
                warn!("sensor {} stopped reporting: {error}", input.describe());
                *state.reported = true;
            }

            state.failed = true;

            None
        }
    }
}
