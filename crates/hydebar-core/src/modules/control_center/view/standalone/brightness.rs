//! Bar entry of the standalone brightness module.

use iced::{Element, widget::{Row, mouse_area}};

use crate::{
    components::{
        icons::{IconTheme, Icons, icon},
        scale,
        text::text
    },
    modules::{
        OnModulePress,
        control_center::{
            audio::wheel_direction,
            brightness::BrightnessMessage,
            state::{ControlCenter, Message}
        }
    }
};

impl ControlCenter {
    /// Bar entry of the standalone brightness module.
    ///
    /// Renders nothing while no backlight device reported itself, so a
    /// desktop without one keeps a bar free of dead icons. The wheel nudges
    /// the backlight by a twentieth of its range per notch, the way the
    /// reference waybar module behaves.
    #[must_use]
    pub fn brightness_bar<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message> + Clone
    {
        let data = self.brightness.as_ref()?;
        let max = data.max.max(1);
        let percent = data.current.saturating_mul(100) / max;
        let current = data.current;

        let entry = Row::new()
            .push(icon(icons, Icons::Brightness))
            .push(text(format!("{percent}%")))
            .spacing(scale::icon_gap())
            .align_y(iced::Alignment::Center);

        let wheeled = mouse_area(entry)
            .on_scroll(move |delta| {
                let step = (max / 20).max(1);
                let target = match wheel_direction(delta) {
                    1 => current.saturating_add(step).min(max),
                    _ => current.saturating_sub(step)
                };

                M::from(Message::Brightness(BrightnessMessage::Change(target)))
            })
            .into();

        Some((wheeled, None))
    }
}
