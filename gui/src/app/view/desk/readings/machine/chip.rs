//! The two processors: what they are, how hard they work, how hot they get.

use hydebar_core::modules::system_info::{SystemInfoData, used_of_total};

use super::super::{Figure, Panel, push};
use crate::app::state::history::Trail;

/// The processor: what it is, how hard it is working, how hot and how fast.
pub fn processor(data: &SystemInfoData, trail: &Trail) -> Option<Panel> {
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

    traced("processor", rows, trail, 100.0)
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
pub fn cpu_temperature(data: &SystemInfoData, trail: &Trail) -> Option<Panel> {
    let mut rows = Vec::new();

    push(
        &mut rows,
        "temperature",
        data.cpu_temperature.map(|degrees| format!("{degrees}°C"))
    );
    push(&mut rows, "sensor", data.cpu_temperature_source.clone());

    traced("cpu temperature", rows, trail, HOT)
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

/// What the top of a temperature trace stands for, in degrees.
///
/// A processor that reaches it is a processor that is throttling, so the
/// ceiling is the line the shape is read against rather than a maximum
/// anything is expected to touch.
const HOT: f32 = 100.0;

/// A panel with the last few minutes of its own reading drawn above the lines.
///
/// A trail of one reading is not a shape, so a block that has only just
/// started keeping one is drawn as the table it has always been.
fn traced(
    title: &'static str,
    rows: Vec<(String, String)>,
    trail: &Trail,
    ceiling: f32
) -> Option<Panel> {
    if !trail.is_drawable() {
        return Panel::of(title, rows);
    }

    Some(Panel::drawn(
        title,
        rows,
        Figure::Trace {
            readings: trail.seen(),
            ceiling
        }
    ))
}
