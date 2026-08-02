//! Measurements of the room the appearance page asks for.

use super::{SCALED_ROWS, SECTIONS};
use crate::{
    components::page::{metrics::row_width, style},
    config::{HydeBranch, NotificationSource}
};

/// Rows this page draws, its section headings counted in.
///
/// The notification row is the one that is not written down in
/// [`SECTIONS`], so it is added here rather than baked into a
/// literal that could drift.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "section and row counts are small, fit f32 exactly"
)]
pub fn rows(auto_scale: bool, hyde_branch: bool) -> f32 {
    let settings: usize = SECTIONS.iter().map(|(_, rows)| rows.len()).sum();
    let scaled = if auto_scale { SCALED_ROWS } else { 0.0 };
    let branch = if hyde_branch { 1.0 } else { 0.0 };

    (SECTIONS.len() as f32).mul_add(style::SECTION_TITLE_ROWS, settings as f32) + 1.0 + branch
        - scaled
}

/// Longest row of this page, which is how wide the window has to be.
///
/// The notification row is measured from the choices themselves rather
/// than from a copy of their names: a fourth source, or a
/// renamed one, would otherwise be cut off by a window sized
/// for the old list.
#[must_use]
pub fn desired_width(font_size: f32) -> f32 {
    let notifications = row_width(
        NotificationSource::ALL
            .into_iter()
            .map(NotificationSource::label),
        font_size
    );
    let branches = row_width(
        HydeBranch::ALL.into_iter().map(HydeBranch::label),
        font_size
    );

    SECTIONS
        .into_iter()
        .flat_map(|(_, rows)| rows.iter())
        .map(|(_, controls)| row_width(controls.iter().copied(), font_size))
        .fold(notifications.max(branches), f32::max)
}

/// Height this page needs.
#[must_use]
pub fn desired_height(font_size: f32, auto_scale: bool, hyde_branch: bool) -> f32 {
    style::page_height(rows(auto_scale, hyde_branch), font_size)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp, clippy::cast_precision_loss)]

    use super::{super::NOTIFICATIONS, *};
    use crate::components::page::metrics::text_width;

    /// Every label this page draws in the shared label column.
    fn labels() -> Vec<&'static str> {
        SECTIONS
            .into_iter()
            .flat_map(|(_, rows)| rows.iter().map(|(label, _)| *label))
            .chain(std::iter::once(NOTIFICATIONS))
            .collect()
    }

    #[test]
    fn the_window_is_wide_enough_for_every_notification_source() {
        let font_size = 16.0;
        let notifications = row_width(
            NotificationSource::ALL
                .into_iter()
                .map(NotificationSource::label),
            font_size
        );

        assert!(desired_width(font_size) >= notifications);
    }

    #[test]
    fn the_notification_row_is_measured_from_all_three_sources() {
        assert_eq!(NotificationSource::ALL.len(), 3);
    }

    #[test]
    fn every_notification_source_has_room_for_its_name() {
        let font_size = 16.0;

        for source in NotificationSource::ALL {
            assert!(desired_width(font_size) >= text_width(source.label(), font_size));
        }
    }

    #[test]
    fn every_row_label_fits_the_shared_label_column() {
        let font_size = 16.0;

        for label in labels() {
            assert!(
                text_width(label, font_size) <= style::label_width(font_size),
                "{label} overflows the label column"
            );
        }
    }

    #[test]
    fn the_page_reserves_a_row_for_every_row_and_every_heading_it_draws() {
        assert_eq!(
            rows(false, false),
            labels().len() as f32 + SECTIONS.len() as f32
        );
        assert_eq!(rows(true, false), rows(false, false) - SCALED_ROWS);
    }

    #[test]
    fn a_configured_updates_module_earns_the_branch_row() {
        assert_eq!(rows(false, true), rows(false, false) + 1.0);
    }

    #[test]
    fn the_page_height_follows_the_shared_row_pitch() {
        let font_size = 16.0;

        assert_eq!(
            desired_height(font_size, false, false),
            style::page_height(rows(false, false), font_size)
        );
    }
}
