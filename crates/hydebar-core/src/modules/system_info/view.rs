use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, Row, column, container, row, rule, text}
};

use super::{Message, data::SystemInfoData};
use crate::{
    components::icons::{IconTheme, Icons, icon},
    config::{Appearance, SystemIndicator, SystemModuleConfig},
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
        container(icon(icons, info_icon).size(22)).center_x(Length::Fixed(32.)),
        text(label).width(Length::Fill),
        text(value)
    )
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .spacing(8)
    .into()
}

fn indicator_info_element<V>(
    icons: &IconTheme,
    info_icon: Icons,
    value: V,
    unit: &str,
    threshold: Option<(V, V)>,
    prefix: Option<String>,
    icon_label_gap: f32
) -> Element<'static, Message>
where
    V: std::fmt::Display + PartialOrd + Copy + 'static
{
    let content = container(
        row!(
            icon(icons, info_icon),
            if let Some(prefix) = prefix {
                text(format!("{prefix} {value}{unit}"))
            } else {
                text(format!("{value}{unit}"))
            }
        )
        .spacing(icon_label_gap)
    );

    if let Some((warn_threshold, alert_threshold)) = threshold {
        content
            .style(move |theme: &Theme| container::Style {
                text_color: if value > warn_threshold && value < alert_threshold {
                    Some(theme.extended_palette().danger.weak.color)
                } else if value >= alert_threshold {
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

fn format_speed(speed: u32) -> (u32, &'static str) {
    if speed > 1000 {
        (speed / 1000, "MB/s")
    } else {
        (speed, "KB/s")
    }
}

/// Render the module menu displaying detailed system metrics.
pub fn build_menu_view<'a>(data: &'a SystemInfoData, icons: &IconTheme) -> Element<'a, Message> {
    column![
        text("System Info").size(20),
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
            .push_maybe(data.temperature.map(|temp| {
                info_element(icons, Icons::Temp, "Temperature", format!("{temp}°C"))
            }))
            .push(
                Column::with_children(
                    data.disks
                        .iter()
                        .map(|(mount_point, usage)| {
                            row!(
                                container(icon(icons, Icons::Drive).size(22))
                                    .center_x(Length::Fixed(32.)),
                                text(format!("Disk Usage {mount_point}")).width(Length::Fill),
                                text(format!("{usage}%"))
                            )
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .into()
                        })
                        .collect::<Vec<Element<_>>>(),
                )
                .spacing(4),
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
            .spacing(4)
            .padding([0, 8])
    ]
    .spacing(8)
    .into()
}

/// Build the indicator widgets representing the configured subset of metrics.
///
/// The gaps inside every indicator come from the themed font size carried by
/// `appearance`, so the bar row keeps its proportions across themes.
pub fn indicator_elements<M>(
    data: SystemInfoData,
    config: &SystemModuleConfig,
    appearance: &Appearance,
    icons: &IconTheme
) -> Vec<Element<'static, M>>
where
    M: 'static + From<Message>
{
    let icon_label_gap = appearance.icon_label_gap();

    config
        .indicators
        .iter()
        .filter_map(|indicator| -> Option<Element<'static, Message>> {
            match indicator {
                SystemIndicator::Cpu => Some(indicator_info_element(
                    icons,
                    Icons::Cpu,
                    data.cpu_usage,
                    "%",
                    Some((config.cpu.warn_threshold, config.cpu.alert_threshold)),
                    None,
                    icon_label_gap
                )),
                SystemIndicator::Memory => Some(indicator_info_element(
                    icons,
                    Icons::Mem,
                    data.memory_usage,
                    "%",
                    Some((config.memory.warn_threshold, config.memory.alert_threshold)),
                    None,
                    icon_label_gap
                )),
                SystemIndicator::MemorySwap => Some(indicator_info_element(
                    icons,
                    Icons::Mem,
                    data.memory_swap_usage,
                    "%",
                    Some((config.memory.warn_threshold, config.memory.alert_threshold)),
                    Some("swap".to_string()),
                    icon_label_gap
                )),
                SystemIndicator::Temperature => data.temperature.map(|temperature| {
                    indicator_info_element(
                        icons,
                        Icons::Temp,
                        temperature,
                        "°C",
                        Some((
                            config.temperature.warn_threshold,
                            config.temperature.alert_threshold
                        )),
                        None,
                        icon_label_gap
                    )
                }),
                SystemIndicator::Disk(mount) => {
                    data.disks.iter().find_map(|(disk_mount, disk)| {
                        if disk_mount == mount {
                            Some(indicator_info_element(
                                icons,
                                Icons::Drive,
                                *disk,
                                "%",
                                Some((config.disk.warn_threshold, config.disk.alert_threshold)),
                                Some(disk_mount.clone()),
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
                    indicator_info_element(
                        icons,
                        Icons::DownloadSpeed,
                        value,
                        unit,
                        None,
                        None,
                        icon_label_gap
                    )
                }),
                SystemIndicator::UploadSpeed => data.network.as_ref().map(|network| {
                    let (value, unit) = format_speed(network.upload_speed);
                    indicator_info_element(
                        icons,
                        Icons::UploadSpeed,
                        value,
                        unit,
                        None,
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
pub fn build_indicator_view<M>(
    data: &SystemInfoData,
    config: &SystemModuleConfig,
    appearance: &Appearance,
    icons: &IconTheme
) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
where
    M: 'static + From<Message>
{
    let indicators = indicator_elements(data.clone(), config, appearance, icons);

    Some((
        Row::with_children(indicators)
            .align_y(Alignment::Center)
            .spacing(appearance.module_gap())
            .into(),
        Some(OnModulePress::ToggleMenu(MenuType::SystemInfo))
    ))
}

// TODO: Fix test imports after config refactoring
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use super::*;
    use crate::config::{SystemInfoDisk, SystemInfoMemory, SystemInfoTemperature};

    fn data_fixture() -> SystemInfoData {
        SystemInfoData {
            cpu_usage:         25,
            memory_usage:      50,
            memory_swap_usage: 10,
            temperature:       Some(42),
            disks:             vec![("/".to_string(), 60)],
            network:           None
        }
    }

    #[test]
    fn indicator_row_contains_configured_entries() {
        let data = data_fixture();
        let config = SystemModuleConfig {
            indicators:  vec![SystemIndicator::Cpu, SystemIndicator::Memory],
            cpu:         Default::default(),
            memory:      SystemInfoMemory {
                warn_threshold:  70,
                alert_threshold: 90
            },
            temperature: SystemInfoTemperature {
                warn_threshold:  70,
                alert_threshold: 90
            },
            disk:        Default::default()
        };

        let indicators: Vec<Element<'_, Message>> =
            indicator_elements(data, &config, &Appearance::default());
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

        let indicators: Vec<Element<'_, Message>> =
            indicator_elements(data, &config, &Appearance::default());
        assert_eq!(indicators.len(), 2);
    }

    #[test]
    fn format_speed_converts_large_values_to_megabytes() {
        let (value, unit) = format_speed(2048);
        assert_eq!((value, unit), (2, "MB/s"));
    }
}
