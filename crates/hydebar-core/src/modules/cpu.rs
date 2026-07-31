//! Standalone processor readout: a thin bar entry over the system monitor.
//!
//! The system monitor owns the sampler and the data; this module only draws
//! the processor share of one sample handed in from outside, the way the
//! standalone audio and network entries draw from the control centre. A press
//! opens the monitor's own window, so the layouts that spell the readouts as
//! `cpu` and `memory` get two entries and one window.

use hydebar_proto::config::{Appearance, MemoryFormat, SystemIndicator, SystemModuleConfig};
use iced::Element;

use super::{
    OnModulePress,
    system_info::{Message, SystemInfoData, single_indicator}
};
use crate::{components::icons::IconTheme, menu::MenuType};

/// Bar entry drawing the processor load out of the shared sample.
pub fn bar_view<M>(
    data: &SystemInfoData,
    config: &SystemModuleConfig,
    appearance: &Appearance,
    icons: &IconTheme
) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
where
    M: 'static + From<Message>
{
    let element = single_indicator(
        &SystemIndicator::Cpu,
        data,
        config,
        MemoryFormat::Percentage,
        appearance,
        icons
    )?;

    Some((
        element,
        Some(OnModulePress::ToggleMenu(MenuType::SystemInfo))
    ))
}

/// States the processor for the pointer resting on the module.
///
/// The load averages are read here rather than carried in the sample: they
/// move on every reading, and a sample that always differs would defeat the
/// deduplication that keeps an idle bar from repainting.
#[must_use]
pub fn hint(data: &SystemInfoData) -> String {
    let load = sysinfo::System::load_average();

    compose(data, Some((load.one, load.five, load.fifteen)))
}

/// The hint text, from facts alone.
fn compose(data: &SystemInfoData, load: Option<(f64, f64, f64)>) -> String {
    let mut lines = vec![match data.cpu_count {
        0 => format!("CPU: {}%", data.cpu_usage),
        count => format!("CPU: {}% of {count} threads", data.cpu_usage)
    }];

    if let Some((one, five, fifteen)) = load
        && (one > 0.0 || five > 0.0 || fifteen > 0.0)
    {
        lines.push(format!("Load: {one:.2} · {five:.2} · {fifteen:.2}"));
    }

    if let Some(temperature) = data.cpu_temperature {
        lines.push(format!("Temperature: {temperature}°C"));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hint_states_the_load_the_averages_and_the_temperature() {
        let data = SystemInfoData {
            cpu_usage: 34,
            cpu_count: 32,
            cpu_temperature: Some(56),
            ..SystemInfoData::default()
        };

        assert_eq!(
            compose(&data, Some((1.245, 1.1, 0.949))),
            "CPU: 34% of 32 threads\nLoad: 1.25 · 1.10 · 0.95\nTemperature: 56°C"
        );
    }

    #[test]
    fn a_machine_without_a_sensor_states_the_load_alone() {
        let data = SystemInfoData {
            cpu_usage: 7,
            ..SystemInfoData::default()
        };

        assert_eq!(compose(&data, None), "CPU: 7%");
    }

    /// A platform that reports no averages — all zeroes — earns no line of
    /// noise for them.
    #[test]
    fn silent_averages_are_left_out() {
        let data = SystemInfoData {
            cpu_usage: 12,
            cpu_count: 8,
            ..SystemInfoData::default()
        };

        assert_eq!(
            compose(&data, Some((0.0, 0.0, 0.0))),
            "CPU: 12% of 8 threads"
        );
    }
}
