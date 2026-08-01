//! Standalone graphics temperature readout.
//!
//! A thin bar entry over the system monitor, after the pattern of the
//! standalone processor and memory entries. The hover hint names the sensor
//! the number is read from, and a press opens the graphics window.

use hydebar_proto::config::{Appearance, MemoryFormat, SystemIndicator, SystemModuleConfig};
use iced::Element;

use super::{
    OnModulePress,
    system_info::{Message, SystemInfoData, single_indicator}
};
use crate::{components::icons::IconTheme, menu::MenuType};

/// Bar entry drawing the graphics temperature out of the shared sample.
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
        &SystemIndicator::GpuTemperature,
        data,
        config,
        MemoryFormat::Percentage,
        appearance,
        icons
    )?;

    Some((element, Some(OnModulePress::ToggleMenu(MenuType::Gpu))))
}

/// States what the number measures for the pointer resting on the module.
#[must_use]
pub fn hint(data: &SystemInfoData) -> String {
    let Some(gpu) = data.gpu.as_ref() else {
        return "GPU temperature".to_owned();
    };

    let source = gpu.source.as_deref().unwrap_or(gpu.name.as_str());

    match gpu.temperature {
        Some(temperature) => format!("GPU temperature: {temperature}°C ({source})"),
        None => format!("GPU temperature ({source})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::system_info::{GpuPlacement, GpuReadings, GpuVendor};

    fn readings(temperature: Option<i32>, source: Option<&str>) -> SystemInfoData {
        SystemInfoData {
            gpu: Some(GpuReadings {
                name: "amdgpu".to_owned(),
                source: source.map(str::to_owned),
                vendor: GpuVendor::Amd,
                placement: GpuPlacement::Discrete,
                temperature,
                utilisation: None,
                memory_used: None,
                memory_total: None
            }),
            ..SystemInfoData::default()
        }
    }

    #[test]
    fn the_hint_names_the_sensor_behind_the_number() {
        assert_eq!(
            hint(&readings(Some(48), Some("amdgpu junction"))),
            "GPU temperature: 48°C (amdgpu junction)"
        );
    }

    #[test]
    fn the_driver_answers_when_no_sensor_is_named() {
        assert_eq!(
            hint(&readings(Some(48), None)),
            "GPU temperature: 48°C (amdgpu)"
        );
    }
}
