//! Placement and fade shared by every menu window.

use hydebar_core::{HEIGHT, menu::MenuLayout};
use iced::{Element, Theme};

use super::super::state::{App, Message};

impl App {
    /// Applies the fade of a travelling menu to its whole subtree.
    ///
    /// This is the one place any menu window animates from: the subtree is
    /// rethemed with every palette colour scaled to the travelled share, and
    /// the default text colour is restated from that faded palette so text
    /// without a colour of its own follows too. The views below are handed the
    /// resting opacity and never animate themselves, which is what makes the
    /// box, text, icons, buttons and swatches all move as one instead of the
    /// background dying before its content.
    /// The content follows the box, not its own straight line. The box fades
    /// at the travelled share times the configured window opacity; content on
    /// a straight share stayed a factor brighter the whole way down and
    /// outlived its own frame. Bending the share to meet the box's curve at
    /// the closed end keeps the two dying together, while at the open end it
    /// still reaches one and the resting window looks untouched.
    pub(super) fn faded_menu<'a>(
        &self,
        menu: Element<'a, Message>,
        progress: f32
    ) -> Element<'a, Message> {
        if progress < 1.0 {
            let opacity = self.config.appearance.menu.opacity.clamp(0.0, 1.0);
            let share =
                (progress * (1.0 - opacity).mul_add(progress, opacity) * 64.0).round() / 64.0;
            let key = (u32::MAX, share.to_bits());

            let theme = self
                .derived_themes
                .borrow_mut()
                .entry(key)
                .or_insert_with(|| hydebar_core::style::faded_theme(&self.theme_cache, share))
                .clone();

            iced::widget::themer(Some(theme), menu)
                .text_color(|theme: &Theme| theme.palette().text)
                .into()
        } else {
            menu
        }
    }

    /// Theme facts a menu needs to place itself, at the given animated opacity.
    pub(super) fn menu_layout(&self, opacity: f32, progress: f32) -> MenuLayout {
        MenuLayout {
            font_size: self.appearance().font_size_px(),
            bar_position: self.config.position,
            style: self.appearance().style,
            opacity,
            radius: self.appearance().pill_radius(),
            menu_backdrop: self.appearance().menu.backdrop,
            finish: hydebar_core::style::IslandFinish::of(self.appearance()),
            content_height: None,
            available_height: self.menu_room(),
            progress
        }
    }

    /// Height a menu box may take before its content has to scroll.
    ///
    /// Derived from the reported screen, never from a button viewport: the
    /// strip below the bar minus the box's own breathing room.
    fn menu_room(&self) -> Option<f32> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the bar height constant is exactly representable in f32"
        )]
        let bar = self.appearance().height.unwrap_or(HEIGHT as f32);

        self.screen_height
            .map(|screen| self.appearance().font_size_px().mul_add(-6.0, screen - bar))
            .filter(|room| *room > 0.0)
    }

    /// Placement of a menu whose content height the caller measured.
    pub(super) fn measured_menu_layout(
        &self,
        opacity: f32,
        progress: f32,
        content_height: f32
    ) -> MenuLayout {
        MenuLayout {
            content_height: Some(content_height),
            ..self.menu_layout(opacity, progress)
        }
    }
}
