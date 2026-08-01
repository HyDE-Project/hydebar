//! The appearance in force and everything the bar derives from it.

use std::sync::Arc;

use hydebar_core::{components::icons::IconTheme, config::ModuleDef};
use hydebar_proto::{
    compositor_look::CompositorLook,
    config::{Appearance, Config}
};
use iced::Task;

use super::{App, Message};

impl App {
    /// Appearance to render with this frame.
    ///
    /// While a config reload is blending this differs from the configured
    /// appearance: colours and opacities lag behind their targets until the
    /// transition settles.
    #[must_use]
    pub const fn appearance(&self) -> &Appearance {
        self.appearance_transition.current()
    }

    /// Rebuilds the cached theme from the appearance in force.
    pub(crate) fn rebuild_theme(&mut self) {
        self.theme_cache = hydebar_core::style::hydebar_theme(self.appearance());
    }

    /// Returns `config` restated for the screen the bar runs on.
    ///
    /// A configuration read from disk carries the sizes the user wrote, not the
    /// sizes this screen needs and nothing the compositor knows; a reload that
    /// only folded the magnification in would drop the window gap the
    /// outermost islands line up with and fall back to the font-derived
    /// margin, which is why the whole restatement lives in one place and both
    /// the first load and every reload after it go through it.
    pub(crate) fn adopted(&self, config: &Config) -> Arc<Config> {
        self.adopted_with(config, &CompositorLook::read())
    }

    /// Restates `config` against a compositor look the caller already has.
    fn adopted_with(&self, config: &Config, look: &CompositorLook) -> Arc<Config> {
        let mut adopted = config.clone();
        adopted.appearance.adopt_screen(self.magnification, look);

        Arc::new(adopted)
    }

    /// Appearance the bar renders with.
    ///
    /// The magnification the screen calls for is already folded into the
    /// configuration before the renderer starts, so nothing is added here.
    #[must_use]
    pub fn scaled_appearance(&self) -> Appearance {
        self.config.appearance.clone()
    }

    /// Rebuilds everything derived from the appearance after the sizes changed.
    ///
    /// The surfaces are re-stated as well: the height of a layer surface is
    /// fixed when it is created, so a bar that changed height has to tell the
    /// compositor, otherwise the strip it occupies keeps the old size while its
    /// contents are drawn at the new one.
    ///
    /// The compositor is only told about the strip when the strip moved: a
    /// reload that recoloured the bar must not re-state the size and exclusive
    /// zone of every surface it did not touch.
    pub(crate) fn refresh_appearance(&mut self) -> Task<Message> {
        let appearance = self.scaled_appearance();

        hydebar_core::components::scale::set_base(appearance.font_size_px());

        self.icons =
            IconTheme::from_config(&self.config.icons).with_size(appearance.font_size_px());
        self.rebuild_theme();

        let blend_palette = appearance.animations.enabled;

        let metrics = (
            appearance.style,
            appearance.scale_factor.to_bits(),
            appearance.height.map(f32::to_bits)
        );
        let resize = if self.stated_layer_metrics == Some(metrics) {
            Task::none()
        } else {
            self.stated_layer_metrics = Some(metrics);
            self.outputs
                .resize(appearance.style, appearance.scale_factor, appearance.height)
        };

        let incoming = self
            .themes
            .switching()
            .or_else(|| self.themes.hyde().theme.as_deref());
        self.sweep = hydebar_core::style::SweepStyle::of(incoming, &appearance);
        self.appearance_transition
            .restyle(self.sweep.response, self.sweep.damping);

        self.appearance_transition
            .set_target(appearance, blend_palette);

        resize
    }

    /// Arms the entrance wave and the greeting for the bar's birth.
    ///
    /// The wave rides on the signature of the theme in force; with animations
    /// off the entrance snaps straight to its resting end and no greeting is
    /// armed at all.
    pub(super) fn arm_birth_animations(&mut self) {
        self.sweep = hydebar_core::style::SweepStyle::of(
            self.themes.hyde().theme.as_deref(),
            &self.config.appearance
        );
        self.entrance = hydebar_core::animation::Spring::new(0.0)
            .with_response(self.sweep.response)
            .with_damping_ratio(self.sweep.damping);

        if self.config.appearance.animations.enabled {
            self.entrance.set_target(1.0);

            if self.config.appearance.greeting
                && hydebar_core::components::greeting::claim_first_entry()
            {
                self.greeting_line = hydebar_core::components::greeting::current();
                self.greeting = hydebar_core::animation::Spring::new(0.0)
                    .with_response(hydebar_core::animation::STANDARD);
                self.greeting.set_target(1.0);
            }
        } else {
            self.entrance.snap_to(1.0);
        }
    }

    /// Glyph table to render module icons with this frame.
    ///
    /// Rebuilt whenever the configuration changes so `[icons]` overrides take
    /// effect on a hot reload.
    #[must_use]
    pub const fn icons(&self) -> &IconTheme {
        &self.icons
    }

    #[must_use]
    pub fn get_all_modules_count(&self) -> usize {
        let count_modules = |modules_def: &[ModuleDef]| -> usize {
            modules_def
                .iter()
                .map(|def| match def {
                    ModuleDef::Single(_) => 1,
                    ModuleDef::Group(group) => group.len()
                })
                .sum()
        };

        count_modules(&self.config.modules.left)
            + count_modules(&self.config.modules.center)
            + count_modules(&self.config.modules.right)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::{num::NonZeroUsize, path::PathBuf, sync::Arc};

    use hydebar_core::{config::ConfigManager, event_bus::EventBus, test_utils::MockHyprlandPort};
    use hydebar_proto::ports::hyprland::HyprlandPort;

    use super::{super::test_support::test_logger, *};

    fn test_app(magnification: f32) -> App {
        let logger = test_logger();
        let config = Config::default();
        let mock_port: Arc<dyn HyprlandPort> = Arc::new(MockHyprlandPort::default());
        let config_manager = Arc::new(ConfigManager::new(config.clone()));
        let capacity = NonZeroUsize::new(16).expect("non-zero");
        let bus = EventBus::new(capacity);
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let event_sender = bus.sender();
        let runtime_handle = runtime.handle().clone();
        let bus_receiver = bus.receiver();

        let (mut app, _) = App::new((
            logger,
            Arc::new(config),
            config_manager,
            PathBuf::new(),
            mock_port,
            event_sender,
            runtime_handle,
            bus_receiver
        ));
        app.magnification = magnification;

        app
    }

    fn window_look() -> CompositorLook {
        CompositorLook {
            rounding: Some(3.0),
            gaps_out: Some(8.0),
            gaps_in: Some(3.0),
            animations: Some(true),
            blur: Some(true),
            ..CompositorLook::default()
        }
    }

    #[test]
    fn a_reloaded_config_keeps_the_islands_at_the_window_gap() {
        let app = test_app(2.0);
        let mut config = Config::default();
        config.appearance.font_size = Some(10.0);
        config.appearance.side_padding = None;

        let reloaded = app.adopted_with(&config, &window_look());

        assert_eq!(reloaded.appearance.side_padding, Some(8.0));
        assert_eq!(reloaded.appearance.bar_padding()[1], 8.0);
    }

    #[test]
    fn reloading_over_and_over_never_moves_the_islands() {
        let app = test_app(2.0);
        let mut config = Config::default();
        config.appearance.font_size = Some(10.0);

        let once = app.adopted_with(&config, &window_look());
        let twice = app.adopted_with(&config, &window_look());

        assert_eq!(
            once.appearance.bar_padding(),
            twice.appearance.bar_padding()
        );
        assert_eq!(once.appearance.font_size, twice.appearance.font_size);
    }

    #[test]
    fn an_unmagnified_bar_is_restated_all_the_same() {
        let app = test_app(1.0);
        let config = Config::default();

        let reloaded = app.adopted_with(&config, &window_look());

        assert_eq!(reloaded.appearance.side_padding, Some(8.0));
    }
}
