//! Appearance page of the settings window.
//!
//! One folder, two rooms: [`sections`] builds the rows of each section
//! and [`metrics`] measures the room the page asks for. The root holds
//! the table of sections the rooms share and assembles the page.

mod metrics;
mod sections;

#[cfg(test)]
pub(super) use metrics::rows;
pub(super) use metrics::{desired_height, desired_width};

use iced::Element;
use sections::{desktop_rows, placement_rows, size_rows};

use crate::{
    components::page::widgets::{page, section},
    config::Config,
    modules::settings::Message
};

/// Title of the section deciding where on the screen the bar sits.
const PLACEMENT: &str = "Placement";

/// Title of the section deciding how large and how solid the bar is
/// drawn.
const SIZE: &str = "Size and colour";

/// Title of the section about the desktop the bar sits on.
const DESKTOP: &str = "Desktop";

/// Every section of the page, as its title and the rows it holds.
///
/// Each row is written down as its label and the controls beside it, so
/// the width the page asks for and the number of rows it
/// reserves height for both come from the same list: a row
/// added to the page without an entry here would be measured
/// out of existence. Rows the size section drops while the bar
/// scales itself.
///
/// Height, side padding and text size are then decided from the screen,
/// so a stepper offering to change them would be offering
/// something that is overwritten the moment it is written.
const SCALED_ROWS: f32 = 3.0;

/// Rows of one section, each as its label and the controls beside it.
type SectionRows = &'static [(&'static str, &'static [&'static str])];

const SECTIONS: [(&str, SectionRows); 3] = [
    (
        PLACEMENT,
        &[
            ("Position", &["Top", "Bottom"]),
            ("Layer", &["Bottom", "Top", "Overlay"])
        ]
    ),
    (
        SIZE,
        &[
            ("Style", &["Islands", "Solid", "Gradient"]),
            ("Height", &["\u{2212}", "000", "+"]),
            ("Side padding", &["\u{2212}", "000", "+"]),
            ("Font size", &["\u{2212}", "000", "+"]),
            ("Opacity", &["\u{2212}", "0.00", "+"])
        ]
    ),
    (DESKTOP, &[])
];

/// Label of the row the notification source is picked on.
///
/// Kept out of [`SECTIONS`] because its choices are named by the source
/// list itself rather than written down here.
const NOTIFICATIONS: &str = "Notifications";

/// Label of the row the `HyDE` branch is picked on.
///
/// Kept out of [`SECTIONS`] like the notification row, and drawn only
/// while the updates module is configured: the choice is stored in
/// that module's section of the file, and writing into a section
/// that does not exist would leave one behind that cannot be read.
const HYDE_BRANCH: &str = "HyDE branch";

/// Renders the appearance page against the running `config`.
///
/// Sizes are shown as they are written in the file, not as the bar
/// draws them: the window magnifies what it renders, and a
/// stepper that showed the magnified size would write it back
/// and magnify it a second time.
///
/// The side padding is shown as the one in force rather than as the one
/// the file names, since a file that names none leaves the bar
/// following the window gaps of the compositor: stepping from
/// the gap actually drawn is what makes the first press nudge
/// the bar instead of jumping it.
pub(super) fn view(config: &Config, opacity: f32, magnification: f32) -> Element<'_, Message> {
    let magnification = if magnification > 0.0 {
        magnification
    } else {
        1.0
    };
    let font_size = config.appearance.font_size_px();

    page(font_size)
        .push(section(
            PLACEMENT,
            placement_rows(config, font_size, opacity),
            font_size
        ))
        .push(section(
            SIZE,
            size_rows(config, magnification, font_size, opacity),
            font_size
        ))
        .push(section(
            DESKTOP,
            desktop_rows(config, font_size, opacity),
            font_size
        ))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_carries_a_title() {
        for (title, _) in SECTIONS {
            assert!(!title.is_empty());
        }
    }

    #[test]
    fn the_desktop_section_holds_only_the_row_named_elsewhere() {
        // its single row is the notification source, whose choices come from
        // the source list rather than from the table, and the row count adds it
        // back on its own
        let desktop = SECTIONS
            .iter()
            .find(|(title, _)| *title == DESKTOP)
            .expect("the desktop section is drawn");

        assert!(desktop.1.is_empty());
    }

    #[test]
    fn nothing_the_bar_decides_for_itself_is_offered() {
        // following the desktop theme and scaling to the screen are not
        // choices: the bar does both, always, and a switch that pretended
        // otherwise would be a switch the bar ignores
        for (_, section_rows) in SECTIONS {
            for (label, _) in section_rows {
                assert_ne!(*label, "Follow HyDE theme");
                assert_ne!(*label, "Scale to the screen");
            }
        }
    }
}
