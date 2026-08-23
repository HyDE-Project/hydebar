//! Drawing of the taskbar strip as one row of window entries.

use iced::{
    Alignment, Element, Length,
    widget::{Row, image, mouse_area, svg}
};

use super::{Message, Taskbar};
use crate::{
    components::{scale, text::text},
    modules::OnModulePress,
    services::tray::{TrayIcon, icon_from_name}
};

impl Taskbar {
    /// The bar entry: one row of window entries, drawn at `font_size`.
    ///
    /// Pressing an entry is handled by the entry itself rather than by the
    /// strip, so the strip names no press of its own.
    ///
    /// Rendered by the module itself, so the bar layer holds no taskbar
    /// drawing of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        font_size: f32
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + Clone + From<Message>
    {
        self.entries_row(font_size).map(|row| (row, None))
    }

    /// The row of entries, one per mapped client, or `None` while nothing is
    /// mapped.
    fn entries_row<M>(&self, font_size: f32) -> Option<Element<'static, M>>
    where
        M: 'static + Clone + From<Message>
    {
        if self.clients.is_empty() {
            return None;
        }

        let side = scale::scaled(font_size * 1.2);

        let entries = self.clients.iter().map(|client| {
            let face: Element<'static, M> = match icon_from_name(&client.class.to_lowercase()) {
                Some(TrayIcon::Image(handle)) => image(handle)
                    .width(Length::Fixed(side))
                    .height(Length::Fixed(side))
                    .opacity(entry_strength(client.focused))
                    .into(),
                Some(TrayIcon::Svg(handle)) => svg(handle)
                    .width(Length::Fixed(side))
                    .height(Length::Fixed(side))
                    .opacity(entry_strength(client.focused))
                    .into(),
                None => text(fallback_glyph(&client.class)).into()
            };

            mouse_area(face)
                .on_press(M::from(Message::Focus(client.address.clone())))
                .into()
        });

        Some(
            Row::with_children(entries)
                .spacing(scale::icon_gap())
                .align_y(Alignment::Center)
                .into()
        )
    }
}

/// How strongly an entry is drawn: the focused window at full strength.
const fn entry_strength(focused: bool) -> f32 {
    if focused { 1.0 } else { 0.55 }
}

/// The letter standing in for a class no icon theme answers for.
fn fallback_glyph(class: &str) -> String {
    class
        .chars()
        .next()
        .map_or_else(|| String::from("?"), |first| first.to_uppercase().collect())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::Arc;

    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::{super::test_client, *};
    use crate::test_utils::MockHyprlandPort;

    fn module() -> Taskbar {
        let port: Arc<dyn HyprlandPort> = Arc::new(MockHyprlandPort::default());

        Taskbar::new(port)
    }

    #[test]
    fn the_focused_window_stands_out() {
        assert!(entry_strength(true) > entry_strength(false));
    }

    #[test]
    fn a_class_without_an_icon_still_shows_a_letter() {
        assert_eq!(fallback_glyph("kitty"), "K");
        assert_eq!(fallback_glyph(""), "?");
    }

    #[test]
    fn a_workspace_with_no_window_carries_no_strip() {
        let mut taskbar = module();
        taskbar.update(Message::ClientsChanged(Vec::new()));

        assert!(taskbar.bar_view::<Message>(12.0).is_none());
    }

    #[test]
    fn a_mapped_window_puts_the_strip_on_the_bar() {
        let mut taskbar = module();
        taskbar.update(Message::ClientsChanged(vec![test_client("0x1", true)]));

        assert!(taskbar.bar_view::<Message>(12.0).is_some());
    }
}
