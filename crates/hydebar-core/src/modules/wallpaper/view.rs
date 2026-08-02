//! Drawing of the wallpaper picker as a grid of pressable tiles.

use iced::{
    Element, Length,
    widget::{Column, Row}
};

use super::{Message, Wallpaper};
use crate::components::scale;

/// Tiles per row of the picker grid.
const PICKER_COLUMNS: usize = 3;

impl Wallpaper {
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
