use iced::{
    Alignment, Element, Theme,
    widget::{Row, container, row}
};

use super::{
    Message,
    data::SystemInfoData,
    indicators,
    sensors::{GpuPlacement, GpuReadings}
};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    config::{Appearance, MemoryFormat, SystemIndicator, SystemModuleConfig},
    menu::MenuType,
    modules::OnModulePress
};

/// Value of an indicator paired with the thresholds coloring it.
#[derive(Debug, Clone, Copy)]
struct Thresholds<V> {
    value: V,
    warn:  V,
    alert: V
}

impl<V> Thresholds<V> {
    const fn new(value: V, warn: V, alert: V) -> Self {
        Self {
            value,
            warn,
            alert
        }
    }
}

/// Builds the text of an indicator out of an optional prefix, a value and
/// its unit.
fn indicator_label(prefix: Option<&str>, value: impl std::fmt::Display, unit: &str) -> String {
    prefix.map_or_else(
        || format!("{value}{unit}"),
        |prefix| format!("{prefix} {value}{unit}")
    )
}

/// Amount of bytes rendered as gibibytes with a single decimal.
///
/// The divisor is binary, so the unit next to the number has to be the
/// binary one: eight gibibytes shown as `8.0GB` overstated every
/// readout by seven percent against the unit it named.
#[expect(
    clippy::cast_precision_loss,
    reason = "byte totals are shown with one decimal; f64 keeps far more precision than the display"
)]
pub fn gigabytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// A pool stated as the amount in use against the amount there is.
pub fn used_of_total(used: u64, total: u64) -> String {
    format!("{} / {} GiB", gigabytes(used), gigabytes(total))
}

/// Renders a memory readout in the format the active index selects.
fn memory_label(format: MemoryFormat, prefix: Option<&str>, usage: u32, used: u64) -> String {
    match format {
        MemoryFormat::Percentage => indicator_label(prefix, usage, "%"),
        MemoryFormat::Bytes => indicator_label(prefix, gigabytes(used), "GiB")
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
pub fn gpu_title(gpu: &GpuReadings) -> String {
    let placement = match gpu.placement {
        GpuPlacement::Integrated => "Integrated graphics",
        GpuPlacement::Discrete | GpuPlacement::Unknown => "Graphics"
    };

    gpu.source.as_deref().map_or_else(
        || format!("{placement} ({})", gpu.name),
        |source| format!("{placement} ({source})")
    )
}

/// Glyph a graphics device wears on the bar.
///
/// An integrated device gets a glyph of its own instead of a text tag beside
/// the number, so every readout on the bar is one icon and one value.
const fn gpu_icon(gpu: &GpuReadings) -> Icons {
    match gpu.placement {
        GpuPlacement::Integrated => Icons::IntegratedGpu,
        GpuPlacement::Discrete | GpuPlacement::Unknown => Icons::Gpu
    }
}

/// A transfer rate, handed in as kilobytes per second, spelled out.
///
/// Above a thousand the rate reads in megabytes with one decimal: the
/// integer division it replaced showed `1 MB/s` for anything up to
/// `1999 KB/s`, understating a rate by up to half.
pub fn format_speed(speed: u32) -> String {
    if speed >= 1000 {
        format!("{:.1} MB/s", f64::from(speed) / 1000.0)
    } else {
        format!("{speed} KB/s")
    }
}

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
    appearance: &Appearance,
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

/// Build the indicator widgets representing the configured subset of
/// metrics.
///
/// The gaps inside every indicator come from the themed font size carried
/// by `appearance`, so the bar row keeps its proportions across themes.
#[must_use]
pub fn indicator_elements<M>(
    data: &SystemInfoData,
    config: &SystemModuleConfig,
    memory_format: MemoryFormat,
    appearance: &Appearance,
    icons: &IconTheme
) -> Vec<Element<'static, M>>
where
    M: 'static + From<Message>
{
    indicators::resolve(config, data)
        .iter()
        .filter_map(|indicator| {
            single_indicator(indicator, data, config, memory_format, appearance, icons)
        })
        .collect()
}

/// Construct the condensed indicator row shown in the module section.
///
/// A module declaring alternative readouts cycles them on the left button
/// and moves the menu to the right button, the way waybar binds
/// `format-alt`.
#[must_use]
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
    let indicators = indicator_elements(data, config, memory_format, appearance, icons);

    let on_press = if config.has_alternatives() {
        OnModulePress::Action(Box::new(M::from(Message::NextFormat)))
    } else {
        OnModulePress::ToggleMenu(MenuType::SystemInfo)
    };

    Some((
        Row::with_children(indicators)
            .align_y(Alignment::Center)
            .spacing(scale::item_gap())
            .into(),
        Some(on_press)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_fixture() -> SystemInfoData {
        SystemInfoData {
            cpu_usage: 25,
            cpu_count: 8,
            memory_usage: 50,
            memory_used: 8 * 1024 * 1024 * 1024,
            memory_total: 16 * 1024 * 1024 * 1024,
            memory_swap_usage: 10,
            memory_swap_used: 1024 * 1024 * 1024,
            memory_swap_total: 10 * 1024 * 1024 * 1024,
            cpu_temperature: Some(42),
            gpu: None,
            disks: vec![crate::modules::system_info::DiskData {
                mount:         "/".to_string(),
                used:          60 * 1024 * 1024 * 1024,
                total:         100 * 1024 * 1024 * 1024,
                usage_percent: 60
            }],
            network: None,
            ..SystemInfoData::default()
        }
    }

    #[test]
    fn indicator_row_contains_configured_entries() {
        let config = SystemModuleConfig {
            indicators: vec![SystemIndicator::Cpu, SystemIndicator::Memory],
            ..SystemModuleConfig::default()
        };

        let indicators: Vec<Element<'_, Message>> = indicator_elements(
            &data_fixture(),
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
            &data,
            &config,
            MemoryFormat::Percentage,
            &Appearance::default(),
            &IconTheme::default()
        );
        assert_eq!(indicators.len(), 2);
    }

    #[test]
    fn format_speed_converts_large_values_to_megabytes() {
        assert_eq!(format_speed(2048), "2.0 MB/s");
    }

    #[test]
    fn format_speed_keeps_the_fraction_a_truncation_used_to_drop() {
        assert_eq!(format_speed(1999), "2.0 MB/s");
        assert_eq!(format_speed(1500), "1.5 MB/s");
        assert_eq!(format_speed(999), "999 KB/s");
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
            "8.0GiB"
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
            "swap 1.0GiB"
        );
    }

    #[test]
    fn gigabytes_round_to_a_single_decimal() {
        assert_eq!(gigabytes(0), "0.0");
        assert_eq!(gigabytes(1024 * 1024 * 1024 * 3 / 2), "1.5");
    }

    #[test]
    fn a_pool_reads_as_used_against_total() {
        assert_eq!(
            used_of_total(8 * 1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024),
            "8.0 / 16.0 GiB"
        );
    }
}
