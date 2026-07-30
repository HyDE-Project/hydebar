use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, Row, column, container, row, rule}
};

use super::{Message, data::SystemInfoData, indicators, sensors::GpuReadings};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale,
        text::text
    },
    config::{Appearance, MemoryFormat, SystemIndicator, SystemModuleConfig},
    menu::MenuType,
    modules::OnModulePress
};

fn info_element<'a>(
    icons: &IconTheme,
    info_icon: Icons,
    label: &'a str,
    value: String
) -> Element<'a, Message> {
    row!(
        container(icon(icons, info_icon).size(scale::scaled(22.0)))
            .center_x(Length::Fixed(scale::scaled(32.0))),
        text(label).width(Length::Fill),
        text(value)
    )
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .spacing(scale::scaled(8.0))
    .into()
}

/// Value of an indicator paired with the thresholds coloring it.
#[derive(Debug, Clone, Copy)]
struct Thresholds<V> {
    value: V,
    warn:  V,
    alert: V
}

impl<V> Thresholds<V> {
    fn new(value: V, warn: V, alert: V) -> Self {
        Self {
            value,
            warn,
            alert
        }
    }
}

/// Builds the text of an indicator out of an optional prefix, a value and its
/// unit.
fn indicator_label(prefix: Option<&str>, value: impl std::fmt::Display, unit: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix} {value}{unit}"),
        None => format!("{value}{unit}")
    }
}

/// Amount of bytes rendered as gibibytes with a single decimal.
fn gigabytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// Renders a memory readout in the format the active index selects.
fn memory_label(format: MemoryFormat, prefix: Option<&str>, usage: u32, used: u64) -> String {
    match format {
        MemoryFormat::Percentage => indicator_label(prefix, usage, "%"),
        MemoryFormat::Bytes => indicator_label(prefix, gigabytes(used), "GB")
    }
}

fn indicator_info_element<V>(
    icons: &IconTheme,
    info_icon: Icons,
    label: String,
    thresholds: Option<Thresholds<V>>,
    icon_label_gap: f32
) -> Element<'static, Message>
where
    V: PartialOrd + Copy + 'static
{
    let content = container(row!(icon(icons, info_icon), text(label)).spacing(icon_label_gap));

    if let Some(thresholds) = thresholds {
        content
            .style(move |theme: &Theme| container::Style {
                text_color: if thresholds.value > thresholds.warn
                    && thresholds.value < thresholds.alert
                {
                    Some(theme.palette().warning)
                } else if thresholds.value >= thresholds.alert {
                    Some(theme.palette().danger)
                } else {
                    None
                },
                ..Default::default()
            })
            .into()
    } else {
        content.into()
    }
}

/// Name of a graphics device as the menu spells it out.
///
/// The placement is spelled out rather than abbreviated, so a machine with
/// switchable graphics says which of its two devices the bar is watching.
fn gpu_title(gpu: &GpuReadings) -> String {
    let placement = match gpu.tag() {
        Some(_) => "Integrated graphics",
        None => "Graphics"
    };

    match gpu.source.as_deref() {
        Some(source) => format!("{placement} ({source})"),
        None => format!("{placement} ({})", gpu.name)
    }
}

fn format_speed(speed: u32) -> (u32, &'static str) {
    if speed > 1000 {
        (speed / 1000, "MB/s")
    } else {
        (speed, "KB/s")
    }
}

/// Readouts this machine cannot report, each with the reason.
///
/// A readout that is simply absent from the bar leaves the user guessing, so
/// the menu names it and says what is missing. A machine that reports
/// everything shows nothing here.
fn missing_readouts(
    data: &SystemInfoData,
    config: &SystemModuleConfig
) -> Option<Element<'static, Message>> {
    let missing: Vec<Element<'static, Message>> = indicators::statuses(config, data)
        .into_iter()
        .filter_map(|status| {
            let reason = status.unavailable?.reason();

            Some(
                text(format!(
                    "{} — {reason}",
                    indicators::title(&status.indicator)
                ))
                .size(scale::scaled(12.0))
                .into()
            )
        })
        .collect();

    if missing.is_empty() {
        return None;
    }

    Some(
        Column::new()
            .push(rule::horizontal(1))
            .push(text("Not reported by this machine").size(scale::scaled(14.0)))
            .extend(missing)
            .spacing(scale::scaled(4.0))
            .into()
    )
}

/// Render the module menu displaying detailed system metrics.
pub fn build_menu_view<'a>(
    data: &'a SystemInfoData,
    config: &SystemModuleConfig,
    icons: &IconTheme
) -> Element<'a, Message> {
    column![
        text("System Info").size(scale::scaled(20.0)),
        rule::horizontal(1),
        Column::new()
            .width(Length::Fill)
            .push(info_element(
                icons,
                Icons::Cpu,
                "CPU Usage",
                format!("{}%", data.cpu_usage)
            ))
            .push(info_element(
                icons,
                Icons::Mem,
                "Memory Usage",
                format!("{}%", data.memory_usage)
            ))
            .push(info_element(
                icons,
                Icons::Mem,
                "Swap memory Usage",
                format!("{}%", data.memory_swap_usage),
            ))
            .push_maybe(data.cpu_temperature.map(|temp| {
                info_element(icons, Icons::Temp, "CPU Temperature", format!("{temp}°C"))
            }))
            .push_maybe(data.gpu.as_ref().map(|gpu| {
                let title = gpu_title(gpu);

                Column::new()
                    .push(text(title).size(scale::scaled(12.0)))
                    .extend(
                        [
                            gpu.temperature.map(|temperature| {
                                info_element(
                                    icons,
                                    Icons::Temp,
                                    "GPU Temperature",
                                    format!("{temperature}°C")
                                )
                            }),
                            gpu.utilisation.map(|usage| {
                                info_element(icons, Icons::Gpu, "GPU Usage", format!("{usage}%"))
                            }),
                            gpu.memory_used.zip(gpu.memory_total).map(|(used, total)| {
                                info_element(
                                    icons,
                                    Icons::Mem,
                                    "GPU Memory",
                                    format!("{}GB / {}GB", gigabytes(used), gigabytes(total))
                                )
                            })
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<Element<_>>>()
                    )
                    .spacing(scale::scaled(4.0))
            }))
            .push(
                Column::with_children(
                    data.disks
                        .iter()
                        .map(|(mount_point, usage)| {
                            row!(
                                container(icon(icons, Icons::Drive).size(scale::scaled(22.0)))
                                    .center_x(Length::Fixed(scale::scaled(32.0))),
                                text(format!("Disk Usage {mount_point}")).width(Length::Fill),
                                text(format!("{usage}%"))
                            )
                            .align_y(Alignment::Center)
                            .spacing(scale::scaled(8.0))
                            .into()
                        })
                        .collect::<Vec<Element<_>>>(),
                )
                .spacing(scale::scaled(4.0)),
            )
            .push_maybe(data.network.as_ref().map(|network| {
                let (download_value, download_unit) = format_speed(network.download_speed);
                let (upload_value, upload_unit) = format_speed(network.upload_speed);

                Column::with_children(vec![
                    info_element(icons, Icons::IpAddress, "IP Address", network.ip.clone()),
                    info_element(
                        icons,
                        Icons::DownloadSpeed,
                        "Download Speed",
                        format!("{download_value} {download_unit}")
                    ),
                    info_element(
                        icons,
                        Icons::UploadSpeed,
                        "Upload Speed",
                        format!("{upload_value} {upload_unit}")
                    ),
                ])
            }))
            .push_maybe(missing_readouts(data, config))
            .spacing(scale::scaled(4.0))
            .padding([scale::scaled(0.0), scale::scaled(8.0)])
    ]
    .spacing(scale::scaled(8.0))
    .into()
}

/// Build the indicator widgets representing the configured subset of metrics.
///
/// The gaps inside every indicator come from the themed font size carried by
/// `appearance`, so the bar row keeps its proportions across themes.
pub fn indicator_elements<M>(
    data: SystemInfoData,
    config: &SystemModuleConfig,
    memory_format: MemoryFormat,
    appearance: &Appearance,
    icons: &IconTheme
) -> Vec<Element<'static, M>>
where
    M: 'static + From<Message>
{
    let icon_label_gap = appearance.icon_label_gap();

    indicators::resolve(config, &data)
        .iter()
        .filter_map(|indicator| -> Option<Element<'static, Message>> {
            match indicator {
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
                            Icons::Gpu,
                            indicator_label(gpu.tag(), temperature, "°C"),
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
                            indicator_label(gpu.tag(), usage, "%"),
                            Some(Thresholds::new(
                                usage,
                                config.gpu.usage_warn_threshold,
                                config.gpu.usage_alert_threshold
                            )),
                            icon_label_gap
                        )
                    })
                }),
                SystemIndicator::Disk(mount) => {
                    data.disks.iter().find_map(|(disk_mount, disk)| {
                        if disk_mount == mount.as_str() {
                            Some(indicator_info_element(
                                icons,
                                Icons::Drive,
                                indicator_label(Some(disk_mount), *disk, "%"),
                                Some(Thresholds::new(
                                    *disk,
                                    config.disk.warn_threshold,
                                    config.disk.alert_threshold
                                )),
                                icon_label_gap
                            ))
                        } else {
                            None
                        }
                    })
                }
                SystemIndicator::IpAddress => data.network.as_ref().map(|network| {
                    let ip = network.ip.clone();
                    container(
                        row!(icon(icons, Icons::IpAddress), text(ip)).spacing(icon_label_gap)
                    )
                    .into()
                }),
                SystemIndicator::DownloadSpeed => data.network.as_ref().map(|network| {
                    let (value, unit) = format_speed(network.download_speed);
                    indicator_info_element::<u32>(
                        icons,
                        Icons::DownloadSpeed,
                        indicator_label(None, value, unit),
                        None,
                        icon_label_gap
                    )
                }),
                SystemIndicator::UploadSpeed => data.network.as_ref().map(|network| {
                    let (value, unit) = format_speed(network.upload_speed);
                    indicator_info_element::<u32>(
                        icons,
                        Icons::UploadSpeed,
                        indicator_label(None, value, unit),
                        None,
                        icon_label_gap
                    )
                })
            }
        })
        .map(|elem| elem.map(M::from))
        .collect()
}

/// Construct the condensed indicator row shown in the module section.
/// A module declaring alternative readouts cycles them on the left button and
/// moves the menu to the right button, the way waybar binds `format-alt`.
pub fn build_indicator_view<M>(
    data: &SystemInfoData,
    config: &SystemModuleConfig,
    memory_format: MemoryFormat,
    appearance: &Appearance,
    icons: &IconTheme
) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
where
    M: 'static + From<Message>
{
    let indicators = indicator_elements(data.clone(), config, memory_format, appearance, icons);

    let on_press = if config.has_alternatives() {
        OnModulePress::Action(Box::new(M::from(Message::NextFormat)))
    } else {
        OnModulePress::ToggleMenu(MenuType::SystemInfo)
    };

    Some((
        Row::with_children(indicators)
            .align_y(Alignment::Center)
            .spacing(appearance.module_gap())
            .into(),
        Some(on_press)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_fixture() -> SystemInfoData {
        SystemInfoData {
            cpu_usage:         25,
            memory_usage:      50,
            memory_used:       8 * 1024 * 1024 * 1024,
            memory_swap_usage: 10,
            memory_swap_used:  1024 * 1024 * 1024,
            cpu_temperature:   Some(42),
            gpu:               None,
            disks:             vec![("/".to_string(), 60)],
            network:           None
        }
    }

    #[test]
    fn indicator_row_contains_configured_entries() {
        let config = SystemModuleConfig {
            indicators: vec![SystemIndicator::Cpu, SystemIndicator::Memory],
            ..SystemModuleConfig::default()
        };

        let indicators: Vec<Element<'_, Message>> = indicator_elements(
            data_fixture(),
            &config,
            MemoryFormat::Percentage,
            &Appearance::default(),
            &IconTheme::default()
        );
        assert_eq!(indicators.len(), 2);
    }

    #[test]
    fn indicator_elements_include_network_entries_when_available() {
        let mut data = data_fixture();
        data.network = Some(crate::modules::system_info::NetworkData::new(
            "127.0.0.1".to_string(),
            2048,
            1024,
            std::time::Instant::now()
        ));

        let config = SystemModuleConfig {
            indicators: vec![SystemIndicator::IpAddress, SystemIndicator::DownloadSpeed],
            ..SystemModuleConfig::default()
        };

        let indicators: Vec<Element<'_, Message>> = indicator_elements(
            data,
            &config,
            MemoryFormat::Percentage,
            &Appearance::default(),
            &IconTheme::default()
        );
        assert_eq!(indicators.len(), 2);
    }

    #[test]
    fn format_speed_converts_large_values_to_megabytes() {
        let (value, unit) = format_speed(2048);
        assert_eq!((value, unit), (2, "MB/s"));
    }

    #[test]
    fn the_memory_readout_follows_the_active_format() {
        let data = data_fixture();

        assert_eq!(
            memory_label(
                MemoryFormat::Percentage,
                None,
                data.memory_usage,
                data.memory_used
            ),
            "50%"
        );
        assert_eq!(
            memory_label(
                MemoryFormat::Bytes,
                None,
                data.memory_usage,
                data.memory_used
            ),
            "8.0GB"
        );
    }

    #[test]
    fn a_prefixed_memory_readout_keeps_its_prefix_in_both_formats() {
        assert_eq!(
            memory_label(
                MemoryFormat::Percentage,
                Some("swap"),
                10,
                1024 * 1024 * 1024
            ),
            "swap 10%"
        );
        assert_eq!(
            memory_label(MemoryFormat::Bytes, Some("swap"), 10, 1024 * 1024 * 1024),
            "swap 1.0GB"
        );
    }

    #[test]
    fn gigabytes_round_to_a_single_decimal() {
        assert_eq!(gigabytes(0), "0.0");
        assert_eq!(gigabytes(1024 * 1024 * 1024 * 3 / 2), "1.5");
    }
}
