//! What the bar sees of weather: the entry it draws and the module wiring.

use hydebar_proto::config::WeatherModuleConfig;

use super::Weather;
use crate::{
    ModuleContext,
    modules::{Module, ModuleError}
};

impl Weather {
    /// The bar entry: the sky glyph and the reading.
    ///
    /// Drawn when the layout places weather as an entry of its own; the
    /// clock keeps hosting its readout either way, off the same refresh
    /// loop. Before the first answer arrives the temperature reads as a
    /// placeholder rather than the entry jumping in later.
    #[must_use]
    pub fn bar_view<M: 'static>(
        &self,
        icons: &crate::components::icons::IconTheme
    ) -> Option<(
        iced::Element<'static, M>,
        Option<crate::modules::OnModulePress<M>>
    )> {
        use iced::{Alignment, widget::Row};

        use crate::components::{
            icons::{Icons, icon},
            scale,
            text::text
        };

        Some((
            Row::new()
                .push(icon(icons, Icons::Weather))
                .push(text(self.data().display_temp().to_owned()))
                .spacing(scale::icon_gap())
                .align_y(Alignment::Center)
                .into(),
            None
        ))
    }
}

impl<M> Module<M> for Weather
where
    M: 'static + Clone
{
    type RegistrationData<'a> = &'a WeatherModuleConfig;
    type ViewData<'a> = ();

    /// Restates the module for the given configuration and starts its
    /// refresh loop.
    ///
    /// Weather has no bar section of its own — the clock hosts the readout —
    /// so registration is the whole contract: adopt whatever the configuration
    /// says now, then keep the reading fresh. Folding `configure` in here is
    /// what lets the bar treat weather like every other module instead of
    /// hand-wiring it.
    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.configure(
            config.location.clone(),
            config.api_key.clone(),
            config.use_celsius,
            config.update_interval_minutes
        );
        self.register(ctx);
        Ok(())
    }

    /// Stops the refresh loop once nothing on the bar shows weather.
    fn deregister(&mut self) {
        self.stop();
    }
}
