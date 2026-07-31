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

/// States the processor in a line or two, for the pointer resting on the
/// module.
#[must_use]
pub fn hint(data: &SystemInfoData) -> String {
    let mut lines = vec![format!("CPU: {}%", data.cpu_usage)];

    if let Some(temperature) = data.cpu_temperature {
        lines.push(format!("Temperature: {temperature}°C"));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hint_states_the_load_and_the_temperature() {
        let data = SystemInfoData {
            cpu_usage: 34,
            cpu_temperature: Some(56),
            ..SystemInfoData::default()
        };

        assert_eq!(hint(&data), "CPU: 34%\nTemperature: 56°C");
    }

    #[test]
    fn a_machine_without_a_sensor_states_the_load_alone() {
        let data = SystemInfoData {
            cpu_usage: 7,
            ..SystemInfoData::default()
        };

        assert_eq!(hint(&data), "CPU: 7%");
    }
}
