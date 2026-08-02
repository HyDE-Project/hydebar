//! One indicator on the bar: an icon, a label and the thresholds that
//! colour it.

use iced::{
    Element, Theme,
    widget::{container, row}
};

use super::super::Message;
use crate::components::{
    icons::{IconTheme, Icons, icon},
    text::text
};

/// Value of an indicator paired with the thresholds coloring it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Thresholds<V> {
    value: V,
    warn:  V,
    alert: V
}

impl<V> Thresholds<V> {
    pub(super) const fn new(value: V, warn: V, alert: V) -> Self {
        Self {
            value,
            warn,
            alert
        }
    }
}

pub(super) fn indicator_info_element<V>(
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
