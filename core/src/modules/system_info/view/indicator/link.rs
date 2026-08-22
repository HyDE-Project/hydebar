//! Where the machine's bytes are: its disks and its network.

use iced::{
    Element,
    widget::{container, row}
};

use super::super::{
    super::{Message, data::SystemInfoData},
    format::{format_speed, indicator_label},
    threshold::{Thresholds, indicator_info_element}
};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        text::text
    },
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
        }),
        _ => None
    }
}
