//! Module trait wiring for the bar layout entry.

use iced::Element;

use super::BarLayout;
use crate::{
    components::icons::{IconTheme, Icons, icon, icon_raw},
    modules::{Module, OnModulePress}
};

impl<M> Module<M> for BarLayout
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
            return Some((icon_raw(self.spinner.glyph().to_owned()).into(), None));
        }

        Some((icon(icons, Icons::BarLayout).into(), None))
    }
}
