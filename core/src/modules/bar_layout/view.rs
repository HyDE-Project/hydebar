//! Drawing of the layout picker as a column of pressable cards.

use iced::{
    Element,
    widget::{Column, container}
};

use super::{BarLayout, Message};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon, icon_raw},
        scale
    },
    modules::OnModulePress
};

impl BarLayout {
    /// The bar entry: the layout glyph, or the spinner while the picker is
    /// reading the desktop's roster.
    ///
    /// Rendered by the module itself, so the bar layer holds no layout
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

        Some((icon(icons, Icons::BarLayout).into(), None))
    }
    /// Renders the picker: the desktop's layouts as pressable cards.
    #[must_use]
    pub fn menu_view<'a>(&self, font_size: f32) -> Element<'a, Message> {
        let gap = scale::scaled(6.0);
        let mut column = Column::new().spacing(gap).padding(scale::scaled(10.0));

        for entry in &self.entries {
            let marker = if entry.active {
                Icons::Point.default_glyph()
            } else {
                Icons::None.default_glyph()
            };

            let face = iced::widget::Row::new()
                .push(icon_raw(marker.to_owned()))
                .push(
                    crate::components::text::text(entry.name.clone())
                        .size(scale::scaled(font_size))
                )
                .spacing(scale::scaled(8.0))
                .align_y(iced::Alignment::Center);

            let card = container(face)
                .padding([scale::scaled(6.0), scale::scaled(12.0)])
                .width(iced::Length::Fill)
                .style({
                    let active = entry.active;
                    move |theme: &iced::Theme| {
                        let palette = theme.extended_palette();

                        if active {
                            container::Style {
                                background: Some(palette.primary.weak.color.into()),
                                text_color: Some(palette.primary.weak.text),
                                border: iced::border::rounded(8),
                                ..container::Style::default()
                            }
                        } else {
                            container::Style {
                                background: Some(palette.background.weak.color.into()),
                                border: iced::border::rounded(8),
                                ..container::Style::default()
                            }
                        }
                    }
                });

            column = column
                .push(iced::widget::mouse_area(card).on_press(Message::Pick(entry.name.clone())));
        }

        column.into()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_layout_glyph_is_always_on_the_strip() {
        let layouts = BarLayout::default();

        assert!(layouts.bar_view::<()>(&IconTheme::default()).is_some());
    }
}
