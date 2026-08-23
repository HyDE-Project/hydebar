//! Bar indicator of the default sink, an icon tracking mute and level.

use iced::Element;

use crate::{
    components::icons::{IconTheme, icon},
    services::audio::{AudioData, Sinks}
};

impl AudioData {
    /// The glyph standing for the default output and its volume.
    #[must_use]
    pub fn sink_indicator<Message: 'static>(
        &self,
        icons: &IconTheme
    ) -> Option<Element<'static, Message>> {
        if self.sinks.is_empty() {
            None
        } else {
            let icon_type = self.sinks.get_icon(&self.server_info.default_sink);

            Some(icon(icons, icon_type).into())
        }
    }
}
