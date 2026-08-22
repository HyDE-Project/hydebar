//! The two processors: what they are, how hard they work, how hot they get.

use hydebar_core::modules::system_info::{SystemInfoData, used_of_total};

use super::super::{Panel, push};

/// The processor: what it is, how hard it is working, how hot and how fast.
pub fn processor(data: &SystemInfoData) -> Option<Panel> {
    let mut rows = vec![("load".to_owned(), format!("{}%", data.cpu_usage))];
    push(&mut rows, "threads", spread(&data.cpu_per_core));

    push(&mut rows, "model", data.cpu_model.clone());
    push(
        &mut rows,
        "cores",
        data.cpu_cores
            .map(|cores| format!("{cores} / {} threads", data.cpu_count))
    );
    push(
        &mut rows,
        "clock",
        data.cpu_current_mhz.map(|mhz| {
            data.cpu_max_mhz.map_or_else(
                || format!("{:.2} GHz", f64::from(mhz) / 1000.0),
                |max| {
                    format!(
                        "{:.2} / {:.2} GHz",
                        f64::from(mhz) / 1000.0,
                        f64::from(max) / 1000.0
                    )
                }
            )
        })
    );
    push(&mut rows, "governor", data.cpu_governor.clone());
    push(
        &mut rows,
        "temperature",
        data.cpu_temperature.map(|degrees| format!("{degrees}°C"))
    );

    Panel::of("processor", rows)
}

/// How the load is spread over the threads, in one line.
///
/// Every thread as a block of eight heights, the way a monitor has drawn a
/// load for as long as there have been monitors: the shape says at a glance
/// whether the machine is working on one thing or on everything, which no
/// single percentage can.
fn spread(cores: &[u32]) -> Option<String> {
    const STEPS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    (!cores.is_empty()).then(|| {
        cores
            .iter()
            .map(|share| {
                let step = (*share as usize * STEPS.len()) / 101;

                STEPS[step.min(STEPS.len() - 1)]
            })
            .collect()
    })
}

/// The processor temperature on its own, with the sensor it is read from.
pub fn cpu_temperature(data: &SystemInfoData) -> Option<Panel> {
    let mut rows = Vec::new();

    push(
        &mut rows,
        "temperature",
        data.cpu_temperature.map(|degrees| format!("{degrees}°C"))
    );
    push(&mut rows, "sensor", data.cpu_temperature_source.clone());

    Panel::of("cpu temperature", rows)
}

/// The graphics device, on a machine that reports one.
pub fn graphics(data: &SystemInfoData) -> Option<Panel> {
    let gpu = data.gpu.as_ref()?;
    let mut rows = vec![("driver".to_owned(), gpu.name.clone())];

    push(&mut rows, "sensor", gpu.source.clone());

    push(
        &mut rows,
        "load",
        gpu.utilisation.map(|share| format!("{share}%"))
    );
    push(
        &mut rows,
        "temperature",
        gpu.temperature.map(|degrees| format!("{degrees}°C"))
    );
    push(
        &mut rows,
        "memory",
        gpu.memory_used
            .zip(gpu.memory_total)
            .map(|(used, total)| used_of_total(used, total))
    );

    Panel::of("graphics", rows)
}
