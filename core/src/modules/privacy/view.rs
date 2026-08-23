//! Drawing of the privacy entry: one icon per thing being watched.

use iced::{
    Alignment, Element,
    widget::{Row, container}
};

use super::Privacy;
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        push_maybe::PushMaybe,
        scale
    },
    modules::OnModulePress
};

impl Privacy {
    /// The bar entry: an icon for the screen, the camera and the microphone,
    /// each shown only while something is using it.
    ///
    /// Draws nothing at all while nothing is watched, so the strip carries no
    /// reassurance nobody asked for.
    ///
    /// Rendered by the module itself, so the bar layer holds no privacy
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
        let service = self.service.as_ref()?;

        if service.no_access() {
            return None;
        }

        let watchers = Row::new()
            .push_maybe(
                service
                    .screenshare_access()
                    .then(|| icon(icons, Icons::ScreenShare))
            )
            .push_maybe(service.webcam_access().then(|| icon(icons, Icons::Webcam)))
            .push_maybe(
                service
                    .microphone_access()
                    .then(|| icon(icons, Icons::Mic1))
            )
            .align_y(Alignment::Center)
            .spacing(scale::item_gap());

        Some((
            container(watchers)
                .style(|theme| container::Style {
                    text_color: Some(theme.extended_palette().danger.weak.color),
                    ..Default::default()
                })
                .into(),
            None
        ))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_drawn_before_the_service_answers() {
        let privacy = Privacy::default();

        assert!(privacy.bar_view::<()>(&IconTheme::default()).is_none());
    }
}
