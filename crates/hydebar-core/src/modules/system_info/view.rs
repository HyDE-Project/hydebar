//! The condensed bar row of the module, assembled out of one readout
//! per configured indicator.
//!
//! The spellings live in [`format`], the coloured single readout in
//! [`indicator`] and [`threshold`]; this file only lines them up.

use iced::{
    Alignment, Element,
    widget::Row
};

use super::{Message, data::SystemInfoData, indicators};
use crate::{
    components::{icons::IconTheme, scale},
    config::{Appearance, MemoryFormat, SystemModuleConfig},
    menu::MenuType,
    modules::OnModulePress
};

mod format;
mod indicator;
mod threshold;

pub use format::{format_speed, gigabytes, gpu_title, used_of_total};
pub use indicator::single_indicator;

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
    use hydebar_proto::config::SystemIndicator;

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
}
