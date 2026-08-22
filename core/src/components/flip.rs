//! Every block glides to its new seat across the whole panel.
//!
//! Each module is wrapped in an anchor that records its absolute position
//! on every frame into one shared registry. The moment the arrangement
//! changes, the registry's live positions are frozen into a departure map;
//! for as long as the caller's transition travels, every surviving module
//! is drawn — and hit — between its old seat and its new one, wherever on
//! the panel both happen to be. A module changing islands or sections
//! therefore flies there as one piece instead of reappearing.
//!
//! Three parts: [`memo`] keeps the shared book of seats, [`anchor`] wraps
//! one block under its key, and [`widget`] is where the wrapper meets the
//! widget tree.

mod anchor;
mod memo;
mod widget;

pub use self::{
    anchor::{DESCENT, FlipAnchor},
    memo::FlipMemo
};

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::cell::RefCell;

    use iced::{
        Element, Point, Theme, Vector,
        widget::{button, container, text}
    };
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {
        Pressed
    }

    fn block(memo: &RefCell<FlipMemo>, key: u64, progress: f32) -> Element<'_, Msg> {
        FlipAnchor::new(
            key,
            progress,
            memo,
            container(button(text("Block")).on_press(Msg::Pressed)).padding(40)
        )
        .into()
    }

    #[test]
    fn a_fresh_book_remembers_nothing() {
        let memo = FlipMemo::default();

        assert!(memo.from_map().is_empty());
    }

    #[test]
    fn departing_turns_the_live_seats_into_the_ones_to_travel_from() {
        let mut memo = FlipMemo::default();
        memo.record(1, 120.0);
        memo.depart();

        assert_eq!(memo.from_map().get(&1), Some(&120.0));
    }

    #[test]
    fn a_seat_nobody_restates_is_dropped_at_the_next_departure() {
        let mut memo = FlipMemo::default();
        memo.record(1, 120.0);
        memo.depart();
        memo.depart();

        assert!(memo.from_map().is_empty());
    }

    #[test]
    fn the_latest_seat_of_a_key_is_the_one_kept() {
        let mut memo = FlipMemo::default();
        memo.record(1, 10.0);
        memo.record(1, 90.0);
        memo.depart();

        assert_eq!(memo.from_map().get(&1), Some(&90.0));
    }

    #[test]
    fn a_resting_anchor_leaves_its_block_where_it_sits() {
        let memo = RefCell::new(FlipMemo::default());
        memo.borrow_mut().record(1, 500.0);
        memo.borrow_mut().depart();

        let anchor: FlipAnchor<'_, Msg> =
            FlipAnchor::new(1, 1.0, &memo, text::<Theme, iced::Renderer>("Block"));

        assert_eq!(anchor.offset(Point::new(20.0, 0.0)), Vector::ZERO);
    }

    #[test]
    fn a_travelling_anchor_draws_its_block_part_way_from_where_it_was() {
        let memo = RefCell::new(FlipMemo::default());
        memo.borrow_mut().record(1, 100.0);
        memo.borrow_mut().depart();

        let anchor: FlipAnchor<'_, Msg> =
            FlipAnchor::new(1, 0.25, &memo, text::<Theme, iced::Renderer>("Block"));

        assert_eq!(anchor.offset(Point::new(20.0, 0.0)), Vector::new(60.0, 0.0));
    }

    #[test]
    fn an_anchor_nobody_remembers_does_not_travel() {
        let memo = RefCell::new(FlipMemo::default());
        let anchor: FlipAnchor<'_, Msg> =
            FlipAnchor::new(9, 0.5, &memo, text::<Theme, iced::Renderer>("Block"));

        assert_eq!(anchor.offset(Point::new(20.0, 0.0)), Vector::ZERO);
    }

    #[test]
    fn a_block_leaving_another_row_travels_down_as_well_as_along() {
        let memo = RefCell::new(FlipMemo::default());
        memo.borrow_mut().record(1, 100.0);
        memo.borrow_mut().depart();

        let anchor: FlipAnchor<'_, Msg> =
            FlipAnchor::new(1, 0.25, &memo, text::<Theme, iced::Renderer>("Block"))
                .departing_from(10.0);

        assert_eq!(
            anchor.offset(Point::new(20.0, 210.0)),
            Vector::new(60.0, -150.0)
        );
    }

    #[test]
    fn a_descending_block_finishes_its_journey_coming_straight_down() {
        let memo = RefCell::new(FlipMemo::default());
        memo.borrow_mut().record(1, 100.0);
        memo.borrow_mut().depart();

        let offset = |progress: f32| {
            let anchor: FlipAnchor<'_, Msg> =
                FlipAnchor::new(1, progress, &memo, text::<Theme, iced::Renderer>("Block"))
                    .departing_from(10.0)
                    .descending_first();

            anchor.offset(Point::new(20.0, 210.0))
        };

        let mut moved_along = None;
        let mut settled_along = 0.0_f32;
        let mut before = offset(0.0);

        for step in 1..=200 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 200.0;
            let now = offset(progress);

            if (now.x - before.x).abs() > 0.001 {
                moved_along.get_or_insert(progress);
                settled_along = progress;
            }

            before = now;
        }

        let set_off = moved_along.expect("it closes in along its lane");

        assert!(
            offset(set_off).y.abs() < offset(0.0).y.abs() * 0.75,
            "it falls out of the row it shared before it moves sideways at all"
        );
        assert!(
            offset(settled_along).y.abs() > 1.0,
            "the sideways move is over while it is still on its way down"
        );
        assert_eq!(
            offset(1.0),
            Vector::ZERO,
            "and the whole journey ends where the block sits"
        );

        for step in 0..=200 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 200.0;

            if progress > settled_along {
                assert_eq!(
                    offset(progress).x,
                    0.0,
                    "nothing moves along after the closing is over, at {progress:.3}"
                );
            }
        }
    }

    #[test]
    fn two_blocks_coming_down_together_never_touch() {
        use iced::{Rectangle, Size};

        const STRIP: f32 = 8.0;
        const ISLAND: Size = Size::new(120.0, 38.0);

        let memo = RefCell::new(FlipMemo::default());
        memo.borrow_mut().record(1, 940.0);
        memo.borrow_mut().record(2, 1070.0);
        memo.borrow_mut().depart();

        let resting = [Point::new(880.0, 120.0), Point::new(760.0, 300.0)];

        for step in 0..=200 {
            #[expect(clippy::cast_precision_loss, reason = "a fixed sample count")]
            let progress = step as f32 / 200.0;

            let seats: Vec<Rectangle> = [1u64, 2]
                .into_iter()
                .zip(resting)
                .map(|(key, rest)| {
                    let anchor: FlipAnchor<'_, Msg> = FlipAnchor::new(
                        key,
                        progress,
                        &memo,
                        text::<Theme, iced::Renderer>("Block")
                    )
                    .departing_from(STRIP)
                    .descending_first();

                    let offset = anchor.offset(rest);

                    Rectangle::new(rest + offset, ISLAND)
                })
                .collect();

            assert!(
                seats[0].intersection(&seats[1]).is_none(),
                "at {progress:.3} the two blocks share {:?} and {:?}",
                seats[0],
                seats[1]
            );
        }
    }

    #[test]
    fn a_block_that_has_arrived_is_left_alone_however_far_it_came() {
        let memo = RefCell::new(FlipMemo::default());
        memo.borrow_mut().record(1, 100.0);
        memo.borrow_mut().depart();

        let anchor: FlipAnchor<'_, Msg> =
            FlipAnchor::new(1, 1.0, &memo, text::<Theme, iced::Renderer>("Block"))
                .departing_from(10.0);

        assert_eq!(anchor.offset(Point::new(20.0, 210.0)), Vector::ZERO);
    }

    #[test]
    fn drawing_an_anchor_writes_the_seat_its_block_rests_at() {
        let memo = RefCell::new(FlipMemo::default());
        let mut ui = simulator(block(&memo, 1, 1.0));
        let _ = ui.snapshot(&Theme::Dark).expect("the block draws");

        memo.borrow_mut().depart();
        assert!(memo.borrow().from_map().contains_key(&1));
    }

    #[test]
    fn a_resting_block_answers_a_press_where_it_sits() {
        let memo = RefCell::new(FlipMemo::default());
        let mut ui = simulator(block(&memo, 1, 1.0));
        let _ = ui.click("Block").expect("the block carries its label");

        let published: Vec<Msg> = ui.into_messages().collect();
        assert_eq!(published, vec![Msg::Pressed]);
    }

    #[test]
    fn a_gliding_block_is_hit_where_it_is_drawn_not_where_it_will_sit() {
        let memo = RefCell::new(FlipMemo::default());
        let settled = {
            let mut ui = simulator(block(&memo, 1, 1.0));

            ui.find("Block")
                .expect("the block carries its label")
                .visible_bounds()
                .expect("a seated block is visible")
        };

        memo.borrow_mut().record(1, 200.0);
        memo.borrow_mut().depart();

        let mut ui = simulator(block(&memo, 1, 0.5));
        let _ = ui.snapshot(&Theme::Dark).expect("the block draws");
        ui.point_at(Point::new(settled.center().x + 100.0, settled.center().y));
        let _ = ui.simulate(iced_test::simulator::click());

        let published: Vec<Msg> = ui.into_messages().collect();
        assert_eq!(published, vec![Msg::Pressed]);
    }

    #[test]
    fn a_gliding_block_ignores_a_press_at_the_seat_it_left() {
        let memo = RefCell::new(FlipMemo::default());
        let settled = {
            let mut ui = simulator(block(&memo, 1, 1.0));

            ui.find("Block")
                .expect("the block carries its label")
                .visible_bounds()
                .expect("a seated block is visible")
        };

        memo.borrow_mut().record(1, 400.0);
        memo.borrow_mut().depart();

        let mut ui = simulator(block(&memo, 1, 0.5));
        let _ = ui.snapshot(&Theme::Dark).expect("the block draws");
        ui.point_at(settled.center());
        let _ = ui.simulate(iced_test::simulator::click());

        assert!(ui.into_messages().next().is_none());
    }

    #[test]
    fn an_anchor_states_its_key_and_journey_when_printed() {
        let memo = RefCell::new(FlipMemo::default());
        let anchor: FlipAnchor<'_, Msg> =
            FlipAnchor::new(7, 0.5, &memo, text::<Theme, iced::Renderer>("Block"));

        let printed = format!("{anchor:?}");

        assert!(printed.contains('7'));
        assert!(printed.contains("0.5"));
    }
}
