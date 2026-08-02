//! Rows of each section of the appearance page.

use iced::Element;

use super::{HYDE_BRANCH, NOTIFICATIONS};
use crate::{
    components::{
        page::widgets::{choice_row, rows as row_stack, stepper_row},
        push_maybe::PushMaybe
    },
    config::{
        Appearance, AppearanceStyle, BarLayer, Config, HydeBranch, NotificationSource, Position
    },
    modules::settings::{Message, Settings}
};

/// Height the bar falls back to while the configuration names none.
const FALLBACK_HEIGHT: f32 = 34.0;

/// Rows of the placement section, against the running `config`.
pub(super) fn placement_rows(config: &Config, font_size: f32, opacity: f32) -> Element<'_, Message> {
    row_stack(font_size)
        .push(choice_row(
            "Position",
            vec![
                ("Top", Position::Top, config.position == Position::Top),
                (
                    "Bottom",
                    Position::Bottom,
                    config.position == Position::Bottom
                ),
            ],
            Message::SetPosition,
            font_size,
            opacity
        ))
        .push(choice_row(
            "Layer",
            vec![
                ("Bottom", BarLayer::Bottom, config.layer == BarLayer::Bottom),
                ("Top", BarLayer::Top, config.layer == BarLayer::Top),
                (
                    "Overlay",
                    BarLayer::Overlay,
                    config.layer == BarLayer::Overlay
                ),
            ],
            Message::SetLayer,
            font_size,
            opacity
        ))
        .into()
}

/// Rows of the size section, with the sizes as the file spells them.
pub(super) fn size_rows(
    config: &Config,
    magnification: f32,
    font_size: f32,
    opacity: f32
) -> Element<'_, Message> {
    let appearance: &Appearance = &config.appearance;
    let written_font_size = font_size / magnification;
    let height = appearance.height.unwrap_or(FALLBACK_HEIGHT) / magnification;
    let side_padding = appearance.bar_padding()[1] / magnification;

    row_stack(font_size)
        .push(choice_row(
            "Style",
            vec![
                (
                    "Islands",
                    AppearanceStyle::Islands,
                    appearance.style == AppearanceStyle::Islands
                ),
                (
                    "Solid",
                    AppearanceStyle::Solid,
                    appearance.style == AppearanceStyle::Solid
                ),
                (
                    "Gradient",
                    AppearanceStyle::Gradient,
                    appearance.style == AppearanceStyle::Gradient
                ),
            ],
            Message::SetStyle,
            font_size,
            opacity
        ))
        .push_maybe((!appearance.auto_scale).then(|| {
            stepper_row(
                "Height",
                format!("{height:.0}"),
                Message::SetHeight(Settings::height_below(height)),
                Message::SetHeight(Settings::height_above(height)),
                font_size,
                opacity
            )
        }))
        .push_maybe((!appearance.auto_scale).then(|| {
            stepper_row(
                "Side padding",
                format!("{side_padding:.0}"),
                Message::SetSidePadding(Settings::side_padding_below(side_padding)),
                Message::SetSidePadding(Settings::side_padding_above(side_padding)),
                font_size,
                opacity
            )
        }))
        .push_maybe((!appearance.auto_scale).then(|| {
            stepper_row(
                "Font size",
                format!("{written_font_size:.0}"),
                Message::SetFontSize(Settings::font_size_below(written_font_size)),
                Message::SetFontSize(Settings::font_size_above(written_font_size)),
                font_size,
                opacity
            )
        }))
        .push(stepper_row(
            "Opacity",
            format!("{:.2}", appearance.opacity),
            Message::SetOpacity(Settings::opacity_below(appearance.opacity)),
            Message::SetOpacity(Settings::opacity_above(appearance.opacity)),
            font_size,
            opacity
        ))
        .into()
}

/// Rows of the desktop section, against the running `config`.
pub(super) fn desktop_rows(config: &Config, font_size: f32, opacity: f32) -> Element<'_, Message> {
    row_stack(font_size)
        .push(choice_row(
            NOTIFICATIONS,
            NotificationSource::ALL
                .into_iter()
                .map(|source| {
                    (
                        source.label(),
                        source,
                        config.notifications.source == source
                    )
                })
                .collect(),
            Message::SetNotificationSource,
            font_size,
            opacity
        ))
        .push_maybe(config.updates.as_ref().map(|updates| {
            choice_row(
                HYDE_BRANCH,
                HydeBranch::ALL
                    .into_iter()
                    .map(|branch| (branch.label(), branch, updates.hyde_branch == branch))
                    .collect(),
                Message::SetHydeBranch,
                font_size,
                opacity
            )
        }))
        .into()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced_test::simulator;

    use super::*;

    const FONT: f32 = 14.0;

    fn config() -> Config {
        Config::default()
    }

    #[test]
    fn the_placement_section_offers_both_edges_and_every_layer() {
        let config = config();
        let mut ui = simulator(placement_rows(&config, FONT, 1.0));

        assert!(ui.find("Position").is_ok());
        assert!(ui.find("Layer").is_ok());
        assert!(ui.find("Overlay").is_ok());
        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn picking_an_edge_asks_for_that_position() {
        let config = config();
        let mut ui = simulator(placement_rows(&config, FONT, 1.0));
        let _ = ui.click("Bottom").expect("the bottom edge is offered");

        let published: Vec<Message> = ui.into_messages().collect();
        assert!(published.contains(&Message::SetPosition(Position::Bottom)));
    }

    #[test]
    fn the_size_section_offers_every_style() {
        let config = config();
        let mut ui = simulator(size_rows(&config, 1.0, FONT, 1.0));

        assert!(ui.find("Style").is_ok());
        assert!(ui.find("Islands").is_ok());
        assert!(ui.find("Solid").is_ok());
        assert!(ui.find("Gradient").is_ok());
    }

    #[test]
    fn a_hand_sized_bar_offers_its_measurements() {
        let mut config = config();
        config.appearance.auto_scale = false;
        config.appearance.height = Some(40.0);

        let mut ui = simulator(size_rows(&config, 1.0, FONT, 1.0));

        assert!(ui.find("Height").is_ok());
        assert!(ui.find("Side padding").is_ok());
        assert!(ui.find("Font size").is_ok());
        assert!(ui.find("40").is_ok());
    }

    #[test]
    fn a_bar_that_sizes_itself_hides_the_measurements_it_owns() {
        let mut config = config();
        config.appearance.auto_scale = true;

        let mut ui = simulator(size_rows(&config, 1.0, FONT, 1.0));

        assert!(ui.find("Height").is_err());
        assert!(ui.find("Side padding").is_err());
        assert!(ui.find("Font size").is_err());
        assert!(ui.find("Opacity").is_ok());
    }

    #[test]
    fn a_bar_naming_no_height_falls_back_to_the_stock_one() {
        let mut config = config();
        config.appearance.auto_scale = false;
        config.appearance.height = None;

        let mut ui = simulator(size_rows(&config, 1.0, FONT, 1.0));

        assert!(
            ui.find(format!("{FALLBACK_HEIGHT:.0}")).is_ok(),
            "the stock height is the one shown"
        );
    }

    #[test]
    fn the_sizes_are_shown_as_the_file_spells_them_not_as_the_screen_scales_them() {
        let mut config = config();
        config.appearance.auto_scale = false;
        config.appearance.height = Some(68.0);

        let mut ui = simulator(size_rows(&config, 2.0, FONT, 1.0));

        assert!(ui.find("34").is_ok(), "a doubled bar is written as half");
    }

    #[test]
    fn stepping_the_opacity_asks_for_the_neighbouring_value() {
        let config = config();
        let opacity = config.appearance.opacity;
        let mut ui = simulator(size_rows(&config, 1.0, FONT, 1.0));

        let _ = ui.click("+").expect("the stepper offers a step up");

        let published: Vec<Message> = ui.into_messages().collect();
        assert!(published.contains(&Message::SetOpacity(Settings::opacity_above(opacity))));
    }

    #[test]
    fn the_desktop_section_offers_every_notification_source() {
        let config = config();
        let mut ui = simulator(desktop_rows(&config, FONT, 1.0));

        assert!(ui.find(NOTIFICATIONS.to_owned()).is_ok());
        for source in NotificationSource::ALL {
            assert!(ui.find(source.label().to_owned()).is_ok());
        }
    }

    #[test]
    fn a_desktop_without_updates_offers_no_branch() {
        let mut config = config();
        config.updates = None;

        let mut ui = simulator(desktop_rows(&config, FONT, 1.0));

        assert!(ui.find(HYDE_BRANCH.to_owned()).is_err());
    }

    #[test]
    fn a_desktop_with_updates_offers_every_branch() {
        let mut config = config();
        config.updates = Some(crate::config::UpdatesModuleConfig::default());

        let mut ui = simulator(desktop_rows(&config, FONT, 1.0));

        assert!(ui.find(HYDE_BRANCH.to_owned()).is_ok());
        for branch in HydeBranch::ALL {
            assert!(ui.find(branch.label().to_owned()).is_ok());
        }
    }

    #[test]
    fn picking_a_branch_asks_for_that_branch() {
        let mut config = config();
        config.updates = Some(crate::config::UpdatesModuleConfig::default());
        let wanted = HydeBranch::ALL
            .into_iter()
            .find(|branch| *branch != config.updates.as_ref().expect("updates are on").hyde_branch)
            .expect("more than one branch exists");

        let mut ui = simulator(desktop_rows(&config, FONT, 1.0));
        let _ = ui
            .click(wanted.label().to_owned())
            .expect("the branch is offered");

        let published: Vec<Message> = ui.into_messages().collect();
        assert!(published.contains(&Message::SetHydeBranch(wanted)));
    }
}
