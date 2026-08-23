//! Drawing of the screenshot entry: the camera and the recording dot.

use iced::{
    Alignment, Element,
    widget::{Row, container}
};

use super::Screenshot;
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale
    },
    menu::MenuType,
    modules::OnModulePress
};

impl Screenshot {
    /// The bar entry: the camera, with a dot beside it while a recording is
    /// running.
    ///
    /// Rendered by the module itself, so the bar layer holds no camera
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
        let content = if self.is_recording {
            Row::new()
                .push(icon(icons, Icons::Point))
                .push(icon(icons, Icons::Camera))
                .spacing(scale::icon_gap())
                .align_y(Alignment::Center)
        } else {
            Row::new().push(icon(icons, Icons::Camera))
        };

        Some((
            container(content).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Screenshot))
        ))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_camera_is_always_on_the_strip_and_opens_its_menu() {
        let screenshot = Screenshot::default();

        let (_, press) = screenshot
            .bar_view::<()>(&IconTheme::default())
            .expect("the camera draws");

        assert!(matches!(
            press,
            Some(OnModulePress::ToggleMenu(MenuType::Screenshot))
        ));
    }

    #[test]
    fn a_running_recording_is_marked_and_still_opens_the_menu() {
        let screenshot = Screenshot {
            is_recording: true,
            ..Screenshot::default()
        };

        assert!(screenshot.bar_view::<()>(&IconTheme::default()).is_some());
    }
}
