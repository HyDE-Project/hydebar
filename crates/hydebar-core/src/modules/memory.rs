//! Standalone memory readout: a thin bar entry over the system monitor.
//!
//! The system monitor owns the sampler and the data; this module only draws
//! the memory share of one sample handed in from outside, in whatever format
//! the monitor's cycling currently selects. A press opens the monitor's own
//! window rather than a window of its own.

use hydebar_proto::config::{Appearance, MemoryFormat, SystemIndicator, SystemModuleConfig};
use iced::Element;

use super::{
    OnModulePress,
    system_info::{Message, SystemInfoData, single_indicator, used_of_total}
};
use crate::{components::icons::IconTheme, menu::MenuType};

/// Bar entry drawing the memory readout out of the shared sample.
pub fn bar_view<M>(
    data: &SystemInfoData,
    config: &SystemModuleConfig,
    memory_format: MemoryFormat,
    appearance: &Appearance,
    icons: &IconTheme
) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
where
    M: 'static + From<Message>
{
    let element = single_indicator(
        &SystemIndicator::Memory,
        data,
        config,
        memory_format,
        appearance,
        icons
    )?;

    Some((
        element,
        Some(OnModulePress::ToggleMenu(MenuType::SystemInfo))
    ))
}

/// States the memory pools in a line or two, for the pointer resting on the
/// module.
#[must_use]
pub fn hint(data: &SystemInfoData) -> String {
    let mut lines = vec![format!(
        "Memory: {} ({}%)",
        used_of_total(data.memory_used, data.memory_total),
        data.memory_usage
    )];

    if data.memory_swap_total > 0 {
        lines.push(format!(
            "Swap: {} ({}%)",
            used_of_total(data.memory_swap_used, data.memory_swap_total),
            data.memory_swap_usage
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn the_hint_states_both_pools() {
        let data = SystemInfoData {
            memory_usage: 15,
            memory_used: 9 * GIB,
            memory_total: 62 * GIB,
            memory_swap_usage: 6,
            memory_swap_used: GIB / 2,
            memory_swap_total: 8 * GIB,
            ..SystemInfoData::default()
        };

        assert_eq!(
            hint(&data),
            "Memory: 9.0 / 62.0 GiB (15%)\nSwap: 0.5 / 8.0 GiB (6%)"
        );
    }

    #[test]
    fn a_machine_without_swap_states_memory_alone() {
        let data = SystemInfoData {
            memory_usage: 50,
            memory_used: 8 * GIB,
            memory_total: 16 * GIB,
            ..SystemInfoData::default()
        };

        assert_eq!(hint(&data), "Memory: 8.0 / 16.0 GiB (50%)");
    }
}
