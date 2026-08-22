//! What the sensors report: the heat and the load of the two chips.

use iced::Element;

use super::super::{
    super::{Message, data::SystemInfoData},
    format::{gpu_icon, indicator_label},
    threshold::{Thresholds, indicator_info_element}
};
use crate::{
    components::icons::{IconTheme, Icons},
    config::{SystemIndicator, SystemModuleConfig}
};

/// The readout of one of this room's indicators, if it is one of them.
///
/// [`None`] both for an indicator another room answers for and for one this
/// machine cannot draw, which the caller treats the same: it asks each room
/// in turn and draws whatever answers.
pub(super) fn readout(
    indicator: &SystemIndicator,
    data: &SystemInfoData,
    config: &SystemModuleConfig,
    icon_label_gap: f32,
    icons: &IconTheme
) -> Option<Element<'static, Message>> {
    match indicator {
        SystemIndicator::CpuTemperature => data.cpu_temperature.map(|temperature| {
            indicator_info_element(
                icons,
                Icons::Temp,
                indicator_label(None, temperature, "°C"),
                Some(Thresholds::new(
                    temperature,
                    config.temperature.warn_threshold,
                    config.temperature.alert_threshold
                )),
                icon_label_gap
            )
        }),
        SystemIndicator::GpuTemperature => data.gpu.as_ref().and_then(|gpu| {
            gpu.temperature.map(|temperature| {
                indicator_info_element(
                    icons,
                    gpu_icon(gpu),
                    indicator_label(None, temperature, "°C"),
                    Some(Thresholds::new(
                        temperature,
                        config.gpu.warn_threshold,
                        config.gpu.alert_threshold
                    )),
                    icon_label_gap
                )
            })
        }),
        SystemIndicator::GpuUsage => data.gpu.as_ref().and_then(|gpu| {
            gpu.utilisation.map(|usage| {
                indicator_info_element(
                    icons,
                    Icons::Accelerator,
                    indicator_label(None, usage, "%"),
                    Some(Thresholds::new(
                        usage,
                        config.gpu.usage_warn_threshold,
                        config.gpu.usage_alert_threshold
                    )),
                    icon_label_gap
                )
            })
        }),
        _ => None
    }
}
