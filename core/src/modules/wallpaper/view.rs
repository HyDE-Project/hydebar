//! Drawing of the wallpaper picker as a grid of pressable tiles.

use iced::{
    Element, Length,
    widget::{Column, Row}
};

use super::{Message, Wallpaper};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon, icon_raw},
        scale
    },
    modules::OnModulePress
};

/// Tiles per row of the picker grid.
const PICKER_COLUMNS: usize = 3;

impl Wallpaper {
    /// The bar entry: the wallpaper glyph, or the spinner while the picker
    /// is reading the theme's pictures.
    ///
    /// Rendered by the module itself, so the bar layer holds no wallpaper
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
        if self.loading {
            return Some((icon_raw(self.spinner.glyph().to_owned()).into(), None));
        }

        Some((icon(icons, Icons::Wallpaper).into(), None))
    }
    /// Renders the picker: the theme's wallpapers as pressable tiles.
    #[must_use]
    pub fn menu_view<'a>(&self, font_size: f32) -> Element<'a, Message> {
        let tile = scale::scaled(font_size * 7.0);
        let gap = scale::scaled(6.0);
        let mut grid = Column::new().spacing(gap);

        for band in self.entries.chunks(PICKER_COLUMNS) {
            let mut row = Row::new().spacing(gap);

            for entry in band {
                let thumb = iced::widget::image(entry.thumbnail.clone())
                    .width(Length::Fixed(tile))
                    .height(Length::Fixed(tile))
                    .content_fit(iced::ContentFit::Cover);

                row = row.push(
                    iced::widget::mouse_area(thumb).on_press(Message::Pick(entry.path.clone()))
                );
            }

            grid = grid.push(row);
        }

        grid.into()
    }
}
