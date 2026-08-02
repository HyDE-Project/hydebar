//! Module trait wiring for the screenshot module.

use iced::{
    Alignment, Element,
    widget::{Row, container}
};

use super::{Screenshot, ScreenshotMessage};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale
    },
    menu::MenuType,
    modules::{Module, OnModulePress}
};

impl<M> Module<M> for Screenshot
where
    M: 'static + Clone + From<ScreenshotMessage>
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    /// Render camera icon with recording indicator.
    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
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
