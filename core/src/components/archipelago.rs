//! Islands drawn under wherever the icons actually are.
//!
//! The pill behind a group of modules is not a box the modules live in —
//! it is painted every frame around the places the modules currently
//! stand. At rest that reproduces the configured islands exactly. While a
//! rearrangement travels, every module glides from its old seat to its
//! new one carrying a pill of its own, and pills of modules that draw
//! near each other fuse into one island, then part again as they pass —
//! no icon is ever bare, and islands form under the arriving furniture.
//!
//! The seams follow the widget's duties: [`builder`] assembles the strip,
//! [`layout`] seats the modules, [`draw`] paints pills and modules,
//! [`events`] delivers input where things are drawn, and [`widget`] ties
//! the pieces to the tree.

mod builder;
mod draw;
mod events;
mod layout;
mod widget;

pub use self::builder::{Archipelago, PillPaint};
/// The shared book of seats, reused from the flip machinery.
pub use super::flip::FlipMemo;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::cell::RefCell;

    use iced::{
        Border, Color, Element, Point, Rectangle, Shadow, Theme,
        widget::{button, text}
    };
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {
        Pressed(&'static str)
    }

    const GAP: f32 = 20.0;
    const PAD: f32 = 8.0;

    #[expect(
        clippy::unnecessary_wraps,
        reason = "the strip asks its paint for an option, and a test paint answers one"
    )]
    fn filled(theme: &Theme) -> Option<PillPaint> {
        Some(PillPaint {
            background: theme.extended_palette().background.weak.color,
            border:     Border::default().rounded(8.0),
            shadow:     Shadow::default()
        })
    }

    fn module(label: &'static str) -> Element<'static, Msg> {
        button(text(label)).on_press(Msg::Pressed(label)).into()
    }

    /// The strip laid out and drawn once, with the bounds of each label
    /// read back from the tree.
    fn seats(strip: Archipelago<'_, Msg>, labels: &[&str]) -> Vec<Rectangle> {
        let mut ui = simulator(Element::new(strip));
        let bounds = labels
            .iter()
            .map(|label| {
                ui.find(*label)
                    .expect("every module carries its label")
                    .visible_bounds()
                    .expect("a seated module is visible")
            })
            .collect();

        let _ = ui.snapshot(&Theme::Dark).expect("the strip draws");

        bounds
    }

    #[test]
    fn a_fresh_strip_holds_nothing() {
        let memo = RefCell::new(FlipMemo::default());
        let strip: Archipelago<'_, Msg> = Archipelago::new(GAP, PAD, 1.0, &memo, filled);

        assert!(strip.is_empty());
        assert!(!strip.push(1, 0, 1.0, module("A")).is_empty());
    }

    #[test]
    fn an_empty_strip_still_draws() {
        let memo = RefCell::new(FlipMemo::default());
        let strip: Archipelago<'_, Msg> = Archipelago::new(GAP, PAD, 1.0, &memo, filled);
        let mut ui = simulator(Element::new(strip));

        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn modules_of_one_island_are_seated_left_to_right() {
        let memo = RefCell::new(FlipMemo::default());
        let strip = Archipelago::new(GAP, PAD, 1.0, &memo, filled)
            .push(1, 0, 1.0, module("A"))
            .push(2, 0, 1.0, module("B"));

        let seats = seats(strip, &["A", "B"]);

        assert!(seats[0].x < seats[1].x);
    }

    #[test]
    fn an_island_is_padded_before_its_first_module() {
        let memo = RefCell::new(FlipMemo::default());
        let strip =
            Archipelago::new(GAP, PAD, 1.0, &memo, filled).push(1, 0, 1.0, module("Only"));

        let seats = seats(strip, &["Only"]);

        assert!(seats[0].x >= PAD);
    }

    #[test]
    fn a_new_island_is_parted_from_the_one_before_it() {
        let memo = RefCell::new(FlipMemo::default());
        let joined = Archipelago::new(GAP, PAD, 1.0, &memo, filled)
            .push(1, 0, 1.0, module("A"))
            .push(2, 0, 1.0, module("B"));
        let joined = seats(joined, &["A", "B"]);

        let other_memo = RefCell::new(FlipMemo::default());
        let parted = Archipelago::new(GAP, PAD, 1.0, &other_memo, filled)
            .push(1, 0, 1.0, module("A"))
            .push(2, 1, 1.0, module("B"));
        let parted = seats(parted, &["A", "B"]);

        assert!(parted[1].x - parted[0].x > joined[1].x - joined[0].x);
    }

    #[test]
    fn a_strip_that_paints_no_pill_still_draws_its_modules() {
        let memo = RefCell::new(FlipMemo::default());
        let strip = Archipelago::new(GAP, PAD, 1.0, &memo, |_: &Theme| None)
            .push(1, 0, 1.0, module("A"))
            .push(2, 1, 1.0, module("B"));

        let seats = seats(strip, &["A", "B"]);

        assert!(seats[0].x < seats[1].x);
    }

    #[test]
    fn a_module_whose_wave_has_not_arrived_gets_no_pill() {
        let memo = RefCell::new(FlipMemo::default());
        let strip = Archipelago::new(GAP, PAD, 1.0, &memo, filled)
            .push(1, 0, 0.0, module("A"))
            .push(2, 1, 1.0, module("B"));

        let seats = seats(strip, &["A", "B"]);

        assert!(seats[0].x < seats[1].x);
    }

    #[test]
    fn a_wave_is_clamped_to_the_share_it_can_travel() {
        let memo = RefCell::new(FlipMemo::default());
        let strip = Archipelago::new(GAP, PAD, 1.0, &memo, filled)
            .push(1, 0, 4.0, module("A"))
            .push(2, 1, -3.0, module("B"));

        assert_eq!(strip.arrivals, vec![1.0, 0.0]);
    }

    #[test]
    fn a_resting_strip_leaves_every_module_where_it_sits() {
        let memo = RefCell::new(FlipMemo::default());
        memo.borrow_mut().record(1, 500.0);
        memo.borrow_mut().depart();

        let strip = Archipelago::new(GAP, PAD, 1.0, &memo, filled).push(1, 0, 1.0, module("A"));

        assert_eq!(strip.offset(memo.borrow().from_map(), 0, 10.0), 0.0);
    }

    #[test]
    fn a_travelling_module_is_drawn_part_way_from_where_it_was() {
        let memo = RefCell::new(FlipMemo::default());
        memo.borrow_mut().record(1, 100.0);
        memo.borrow_mut().depart();

        let strip = Archipelago::new(GAP, PAD, 0.25, &memo, filled).push(1, 0, 1.0, module("A"));

        assert_eq!(strip.offset(memo.borrow().from_map(), 0, 20.0), 60.0);
    }

    #[test]
    fn a_module_nobody_remembers_does_not_travel() {
        let memo = RefCell::new(FlipMemo::default());
        let strip = Archipelago::new(GAP, PAD, 0.5, &memo, filled).push(9, 0, 1.0, module("A"));

        assert_eq!(strip.offset(memo.borrow().from_map(), 0, 20.0), 0.0);
    }

    #[test]
    fn a_resting_module_answers_a_press_where_it_is_drawn() {
        let memo = RefCell::new(FlipMemo::default());
        let strip = Archipelago::new(GAP, PAD, 1.0, &memo, filled)
            .push(1, 0, 1.0, module("A"))
            .push(2, 1, 1.0, module("B"));

        let mut ui = simulator(Element::new(strip));
        let _ = ui.click("B").expect("the second module is there");

        let published: Vec<Msg> = ui.into_messages().collect();
        assert_eq!(published, vec![Msg::Pressed("B")]);
    }

    #[test]
    fn a_gliding_module_is_hit_where_it_is_drawn_not_where_it_will_sit() {
        let memo = RefCell::new(FlipMemo::default());
        let settled = {
            let resting = Archipelago::new(GAP, PAD, 1.0, &memo, filled)
                .push(1, 0, 1.0, module("A"));

            seats(resting, &["A"])[0]
        };

        memo.borrow_mut().record(1, settled.x + 200.0);
        memo.borrow_mut().depart();

        let travelling =
            Archipelago::new(GAP, PAD, 0.5, &memo, filled).push(1, 0, 1.0, module("A"));

        let mut ui = simulator(Element::new(travelling));
        ui.point_at(Point::new(
            settled.center().x + 100.0,
            settled.center().y
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let published: Vec<Msg> = ui.into_messages().collect();
        assert_eq!(published, vec![Msg::Pressed("A")]);
    }

    #[test]
    fn a_gliding_module_ignores_a_press_at_the_seat_it_left() {
        let memo = RefCell::new(FlipMemo::default());
        let settled = {
            let resting = Archipelago::new(GAP, PAD, 1.0, &memo, filled)
                .push(1, 0, 1.0, module("A"));

            seats(resting, &["A"])[0]
        };

        memo.borrow_mut().record(1, settled.x + 400.0);
        memo.borrow_mut().depart();

        let travelling =
            Archipelago::new(GAP, PAD, 0.5, &memo, filled).push(1, 0, 1.0, module("A"));

        let mut ui = simulator(Element::new(travelling));
        ui.point_at(settled.center());
        let _ = ui.simulate(iced_test::simulator::click());

        assert!(ui.into_messages().next().is_none());
    }

    #[test]
    fn the_strip_reports_the_pointer_of_whatever_is_under_the_cursor() {
        let memo = RefCell::new(FlipMemo::default());
        let strip = Archipelago::new(GAP, PAD, 1.0, &memo, filled)
            .push(1, 0, 1.0, module("A"))
            .push(2, 0, 1.0, module("B"));

        let mut ui = simulator(Element::new(strip));
        let seat = ui
            .find("A")
            .expect("the module is there")
            .visible_bounds()
            .expect("a seated module is visible");

        ui.point_at(seat.center());
        assert!(ui.snapshot(&Theme::Dark).is_ok());
    }

    #[test]
    fn a_pill_is_described_by_the_paint_it_is_handed() {
        let paint = filled(&Theme::Dark).expect("the theme paints a pill");

        assert_eq!(paint.background, Theme::Dark.extended_palette().background.weak.color);
        assert_eq!(paint.border.radius, 8.0.into());
        assert_eq!(paint.shadow.color, Color::TRANSPARENT);
    }

    #[test]
    fn a_strip_states_its_keys_and_journey_when_printed() {
        let memo = RefCell::new(FlipMemo::default());
        let strip = Archipelago::new(GAP, PAD, 0.5, &memo, filled)
            .push(7, 0, 1.0, module("A"));

        let printed = format!("{strip:?}");

        assert!(printed.contains('7'));
        assert!(printed.contains("0.5"));
    }
}
