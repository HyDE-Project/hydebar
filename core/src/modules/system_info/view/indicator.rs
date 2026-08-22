//! The bar readout of each configured indicator, one spelling per
//! subject.
//!
//! Three rooms, by what the reading is of: [`machine`] is the processor and
//! the memory it works in, [`heat`] is what the sensors report, and [`link`]
//! is the disks and the network. The standalone modules draw single readouts
//! out of the same sample the combined module renders, so the one spelling of
//! every readout lives in one of the three and the thin entries cannot drift
//! from it.

mod heat;
mod link;
mod machine;

use iced::Element;

use super::super::{Message, data::SystemInfoData};
use crate::{
    components::icons::IconTheme,
    config::{MemoryFormat, SystemIndicator, SystemModuleConfig}
};

/// Bar readout of one indicator, or nothing while this machine cannot
/// draw it.
#[must_use]
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
    let gap = appearance.icon_label_gap();

    let element = machine::readout(indicator, data, config, memory_format, gap, icons)
        .or_else(|| heat::readout(indicator, data, config, gap, icons))
        .or_else(|| link::readout(indicator, data, config, gap, icons));

    element.map(|element| element.map(M::from))
}
