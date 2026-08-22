//! What the processor is doing, and the memory it works in.

use iced::Element;

use super::super::{
    super::{Message, data::SystemInfoData},
    format::{indicator_label, memory_label},
    threshold::{Thresholds, indicator_info_element}
};
use crate::{
    components::icons::{IconTheme, Icons},
    config::{MemoryFormat, SystemIndicator, SystemModuleConfig}
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
    memory_format: MemoryFormat,
    icon_label_gap: f32,
    icons: &IconTheme
) -> Option<Element<'static, Message>> {
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
        _ => None
    }
}
