//! The bar readout of each configured indicator, one spelling per
//! subject.

use iced::{Element, widget::container, widget::row};

use super::{
    super::{Message, data::SystemInfoData},
    format::{format_speed, gpu_icon, indicator_label, memory_label},
    threshold::{Thresholds, indicator_info_element}
};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        text::text
    },
    config::{MemoryFormat, SystemIndicator, SystemModuleConfig}
};

/// Bar readout of one indicator, or nothing while this machine cannot
/// draw it.
///
/// The standalone processor and memory modules draw single readouts out
/// of the same sample the combined module renders, so the one spelling of
/// every readout lives here and the thin entries cannot drift from it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one match arm per configured indicator keeps every readout spelling in one place"
)]
pub fn single_indicator<M>(
    indicator: &SystemIndicator,
    data: &SystemInfoData,
    config: &SystemModuleConfig,
    memory_format: MemoryFormat,
    appearance: &crate::config::Appearance,
    icons: &IconTheme
) -> Option<Element<'static, M>>
where
    M: 'static + From<Message>
{
    let icon_label_gap = appearance.icon_label_gap();

    let element: Option<Element<'static, Message>> = match indicator {
        SystemIndicator::Cpu => Some(indicator_info_element(
            icons,
            Icons::Cpu,
            indicator_label(None, data.cpu_usage, "%"),
            Some(Thresholds::new(
                data.cpu_usage,
                config.cpu.warn_threshold,
                config.cpu.alert_threshold
            )),
            icon_label_gap
        )),
        SystemIndicator::Memory => Some(indicator_info_element(
            icons,
            Icons::Mem,
            memory_label(memory_format, None, data.memory_usage, data.memory_used),
            Some(Thresholds::new(
                data.memory_usage,
                config.memory.warn_threshold,
                config.memory.alert_threshold
            )),
            icon_label_gap
        )),
        SystemIndicator::MemorySwap => Some(indicator_info_element(
            icons,
            Icons::Mem,
            memory_label(
                memory_format,
                Some("swap"),
                data.memory_swap_usage,
                data.memory_swap_used
            ),
            Some(Thresholds::new(
                data.memory_swap_usage,
                config.memory.warn_threshold,
                config.memory.alert_threshold
            )),
            icon_label_gap
        )),
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
        SystemIndicator::Disk(mount) => data.disks.iter().find_map(|disk| {
            if disk.mount == mount.as_str() {
                Some(indicator_info_element(
                    icons,
                    Icons::Drive,
                    indicator_label(Some(disk.mount.as_str()), disk.usage_percent, "%"),
                    Some(Thresholds::new(
                        disk.usage_percent,
                        config.disk.warn_threshold,
                        config.disk.alert_threshold
                    )),
                    icon_label_gap
                ))
            } else {
                None
            }
        }),
        SystemIndicator::IpAddress => data.network.as_ref().map(|network| {
            let ip = network.ip.clone();
            container(row!(icon(icons, Icons::IpAddress), text(ip)).spacing(icon_label_gap)).into()
        }),
        SystemIndicator::DownloadSpeed => data.network.as_ref().map(|network| {
            indicator_info_element::<u32>(
                icons,
                Icons::DownloadSpeed,
                format_speed(network.download_speed),
                None,
                icon_label_gap
            )
        }),
        SystemIndicator::UploadSpeed => data.network.as_ref().map(|network| {
            indicator_info_element::<u32>(
                icons,
                Icons::UploadSpeed,
                format_speed(network.upload_speed),
                None,
                icon_label_gap
            )
        })
    };

    element.map(|element| element.map(M::from))
}
