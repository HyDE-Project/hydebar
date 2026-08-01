//! Standalone processor temperature readout.
//!
//! A thin bar entry over the system monitor, after the pattern of the
//! standalone processor and memory entries: the monitor owns the sampler,
//! this module draws one reading of one sample. The hover hint names the
//! sensor the number is read from, and a press opens the processor window,
//! which states the same sensor beside the same number.

use hydebar_proto::config::{Appearance, MemoryFormat, SystemIndicator, SystemModuleConfig};
use iced::Element;

use super::{
    OnModulePress,
    system_info::{Message, SystemInfoData, single_indicator}
};
use crate::{components::icons::IconTheme, menu::MenuType};

/// Bar entry drawing the processor temperature out of the shared sample.
#[must_use]
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
        &SystemIndicator::CpuTemperature,
        data,
        config,
        MemoryFormat::Percentage,
        appearance,
        icons
    )?;

    Some((element, Some(OnModulePress::ToggleMenu(MenuType::CpuTemp))))
}

/// States what the number measures for the pointer resting on the module.
#[must_use]
pub fn hint(data: &SystemInfoData) -> String {
    match (data.cpu_temperature, data.cpu_temperature_source.as_ref()) {
        (Some(temperature), Some(source)) => {
            format!("CPU temperature: {temperature}°C ({source})")
        }
        (Some(temperature), None) => format!("CPU temperature: {temperature}°C"),
        (None, _) => "CPU temperature".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hint_names_the_sensor_behind_the_number() {
        let data = SystemInfoData {
            cpu_temperature: Some(77),
            cpu_temperature_source: Some("k10temp Tctl".to_owned()),
            ..SystemInfoData::default()
        };

        assert_eq!(hint(&data), "CPU temperature: 77°C (k10temp Tctl)");
    }

    #[test]
    fn a_machine_without_a_named_sensor_still_states_the_subject() {
        let data = SystemInfoData {
            cpu_temperature: Some(60),
            ..SystemInfoData::default()
        };

        assert_eq!(hint(&data), "CPU temperature: 60°C");
    }
}
