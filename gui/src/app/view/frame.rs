//! The frame: the ring around the screen.
//!
//! The surface covers the whole screen but paints only the strips the user
//! switched on, in the strip's own ground — [`App::strip_ground`], the very
//! colour the bar's backdrop paints the strip in, `bar_opacity` on island
//! styles and `opacity` on the rest. One ground for the strip and the ring:
//! whatever the bar looks like, the ring is the same paint continued around
//! the screen, and it parts together with the strip's own wash. Nothing is
//! painted while the frame is disabled, the strip is gone, or the
//! compositor has not reported this screen's size yet.
//!
//! The geometry is the output's own measured logical size, never a screen
//! another output reported: a 4K panel and a 1080p one ring themselves each
//! in their own numbers.

use hydebar_core::{
    HEIGHT,
    frame::{geometry::frame_pieces, program::FrameCanvas}
};
use iced::{Element, Length, SurfaceId as Id, widget::canvas};

use super::super::state::{App, Message};

impl App {
    /// Draws the frame of one output: the enabled edge strips, or nothing.
    pub(super) fn frame_surface(&self, id: Id) -> Element<'_, Message> {
        let frame = &self.config.appearance.frame;
        let empty = iced::widget::Row::new().into();

        if !frame.enabled {
            return empty;
        }

        let Some(name) = self.outputs.screen_of(id).flatten() else {
            return empty;
        };

        let Some(&(screen_w, screen_h)) = self.screen_sizes.get(name) else {
            return empty;
        };

        let wash = self.strip_wash(Some(name));

        if wash <= 0.0 {
            return empty;
        }

        let appearance = self.appearance();
        let theme = &self.theme_cache;
        let color = theme.palette().background.scale_alpha(appearance.opacity);

        #[expect(
            clippy::cast_possible_truncation,
            reason = "the bar height constant is exactly representable in f32"
        )]
        let bar_height = appearance.height_px().unwrap_or(HEIGHT as f32);

        let shape = frame_pieces(screen_w, screen_h, frame, self.config.position, bar_height);

        log::debug!(
            "frame: drawing the ring of {name} over {screen_w}x{screen_h}, wash {wash}, shape {shape:?}"
        );

        canvas(FrameCanvas {
            shape,
            color
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use hydebar_core::config::Outputs as Requested;
    use iced_test::simulator;

    use super::{
        super::super::state::{Message, test_support::test_app_with},
        *
    };

    fn tracked(app: &mut App, name: &str) {
        let style = app.config.appearance.style;
        let position = app.config.position;
        let task = app.outputs.add::<Message>(
            style,
            &app.config.outputs,
            position,
            name,
            iced::OutputId::for_tests(),
            &app.config,
            1.0,
            None
        );
        let _ = task;
    }

    fn ring_of(app: &App) -> iced::SurfaceId {
        app.outputs
            .frame_surfaces()
            .next()
            .map(|(id, _)| id)
            .expect("a tracked output with a frame owns a ring")
    }

    #[test]
    fn an_enabled_frame_paints_its_output() {
        let mut app = test_app_with(|config| {
            config.outputs = Requested::Targets(vec!["DP-1".to_owned()]);
            config.appearance.frame.enabled = true;
        });

        tracked(&mut app, "DP-1");
        app.screen_sizes.insert("DP-1".to_owned(), (1920.0, 1080.0));

        let mut ui = simulator(app.frame_surface(ring_of(&app)));

        assert!(ui.snapshot(&iced::Theme::Dark).is_ok(), "the ring paints");
    }

    #[test]
    fn a_frame_without_a_measured_output_paints_nothing() {
        let mut app = test_app_with(|config| {
            config.outputs = Requested::Targets(vec!["DP-1".to_owned()]);
            config.appearance.frame.enabled = true;
        });

        tracked(&mut app, "DP-1");

        let mut ui = simulator(app.frame_surface(ring_of(&app)));

        assert!(
            ui.snapshot(&iced::Theme::Dark).is_ok(),
            "an unmeasured output paints nothing, and paints it cleanly"
        );
    }

    #[test]
    fn a_disabled_frame_paints_nothing() {
        let app = test_app_with(|_| {});

        let mut ui = simulator(app.frame_surface(iced::SurfaceId::unique()));

        assert!(
            ui.snapshot(&iced::Theme::Dark).is_ok(),
            "a disabled frame paints nothing, and paints it cleanly"
        );
    }

    #[test]
    fn a_surface_the_bar_does_not_own_paints_nothing() {
        let app = test_app_with(|config| {
            config.appearance.frame.enabled = true;
        });

        let mut ui = simulator(app.frame_surface(iced::SurfaceId::unique()));

        assert!(
            ui.snapshot(&iced::Theme::Dark).is_ok(),
            "a stranger surface paints nothing, and paints it cleanly"
        );
    }

    #[test]
    fn a_frame_gone_with_the_strip_paints_nothing() {
        let mut app = test_app_with(|config| {
            config.desk.enabled = true;
            config.outputs = Requested::Targets(vec!["DP-1".to_owned()]);
            config.appearance.frame.enabled = true;
        });

        tracked(&mut app, "DP-1");
        app.screen_sizes.insert("DP-1".to_owned(), (1920.0, 1080.0));

        // the canvas holds the screen: the strip, and the ring with it, is
        // gone until the blocks travel home
        app.desk_clocks
            .entry(Some("DP-1".to_owned()))
            .or_default()
            .open();

        assert_eq!(app.strip_wash(Some("DP-1")), 0.0, "the strip is gone");

        let mut ui = simulator(app.frame_surface(ring_of(&app)));

        assert!(
            ui.snapshot(&iced::Theme::Dark).is_ok(),
            "a gone strip takes its ring with it, cleanly"
        );
    }
}
