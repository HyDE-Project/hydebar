//! Widget layer: views composed from the modules they draw.

pub mod tray {
    //! Tray module view layer: one bar icon per status notifier item.

    use hydebar_core::{
        components::{
            icons::{IconTheme, Icons, icon},
            scale
        },
        menu::MenuType,
        modules::tray::TrayModule,
        position_button::position_button,
        services::tray::TrayIcon,
        style::ghost_button_style
    };
    use iced::{
        Alignment, Element, Length, SurfaceId as Id,
        widget::{Row, image, svg}
    };

    use crate::app::Message;

    /// Renders the tray strip, or nothing while no application is registered.
    ///
    /// Each icon is its own positioned button so the menu it toggles opens
    /// under the icon that was pressed, not under the strip as a whole.
    pub fn render_tray(
        module: &TrayModule,
        id: Id,
        opacity: f32,
        icons: &IconTheme
    ) -> Option<Element<'static, Message>> {
        let items = module.service.as_ref().filter(|s| !s.data.is_empty())?;
        let size = icons.size().unwrap_or_else(scale::base);

        Some(
            Row::with_children(items.data.iter().map(|item| {
                let content: Element<'static, Message> = match &item.icon {
                    Some(TrayIcon::Image(handle)) => image(handle.clone())
                        .height(Length::Fixed(size))
                        .width(Length::Fixed(size))
                        .into(),
                    Some(TrayIcon::Svg(handle)) => svg(handle.clone())
                        .height(Length::Fixed(size))
                        .width(Length::Fixed(size))
                        .into(),
                    None => icon(icons, Icons::Point).into()
                };

                let name = item.name.clone();

                position_button(content)
                    .padding([scale::scaled(2.0), scale::scaled(4.0)])
                    .style(ghost_button_style(opacity))
                    .on_press_with_position(move |button_ui_ref| {
                        Message::ToggleMenu(MenuType::Tray(name.clone()), id, button_ui_ref)
                    })
                    .into()
            }))
            .align_y(Alignment::Center)
            .spacing(scale::icon_gap())
            .into()
        )
    }
}
