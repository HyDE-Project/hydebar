//! Rows of each section of the appearance page.
//!
//! Three rooms, one per section of the page: [`placement`] is where the bar
//! stands, [`size`] is how big it is drawn, and [`desktop`] is what it takes
//! from the desktop around it.

mod desktop;
mod placement;
mod size;

pub(super) use desktop::desktop_rows;
pub(super) use placement::placement_rows;
pub(super) use size::size_rows;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use iced_test::simulator;

    use super::{
        super::{HYDE_BRANCH, NOTIFICATIONS},
        size::FALLBACK_HEIGHT,
        *
    };
    use crate::{
        config::{Config, HydeBranch, NotificationSource, Position},
        modules::settings::{Message, Settings}
    };

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

        assert!(
            ui.into_messages()
                .any(|message| message == Message::SetPosition(Position::Bottom))
        );
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

        assert!(
            ui.into_messages()
                .any(|message| message == Message::SetOpacity(Settings::opacity_above(opacity)))
        );
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

        assert!(
            ui.into_messages()
                .any(|message| message == Message::SetHydeBranch(wanted))
        );
    }
}
