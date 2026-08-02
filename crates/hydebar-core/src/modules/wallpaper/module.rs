//! Module trait wiring for the wallpaper entry.

use iced::Element;

use super::Wallpaper;
use crate::{
    components::icons::{IconTheme, Icons, icon},
    modules::{Module, OnModulePress}
};

impl<M> Module<M> for Wallpaper
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if self.loading {
            return Some((
                crate::components::icons::icon_raw(self.spinner.glyph().to_owned()).into(),
                None
            ));
        }

        Some((icon(icons, Icons::Wallpaper).into(), None))
    }
}
