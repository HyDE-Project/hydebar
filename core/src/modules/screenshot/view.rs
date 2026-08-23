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
