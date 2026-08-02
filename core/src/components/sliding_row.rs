//! A row whose children glide to their new places instead of jumping.
//!
//! When the bar's arrangement changes, every block is given a key and the
//! row remembers where each key last stood. For as long as the caller's
//! transition is travelling, a surviving block is drawn — and hit — at a
//! position interpolated between its old seat and its new one, so a layout
//! switch reads as furniture sliding across the shelf rather than as a cut.
//!
//! The seams follow the widget's duties: [`builder`] assembles the row,
//! [`state`] is what it remembers between frames, [`layout`] seats the
//! children, [`draw`] paints them where the slide holds them, [`events`]
//! delivers input there too, and [`widget`] ties the pieces to the tree.

mod builder;
mod draw;
mod events;
mod layout;
mod state;
mod widget;

pub use self::builder::SlidingRow;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #![allow(clippy::float_cmp)]

    use iced::{
        Element, Point, Rectangle, Size, Theme,
        widget::{button, text}
    };
    use iced_test::{
        core::{Event, clipboard, mouse, renderer::Style},
        runtime::user_interface::{Cache, UserInterface},
        simulator
    };

    use super::{state::State, *};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Msg {
        Pressed(&'static str)
    }

    const SPACING: f32 = 10.0;
    const VIEWPORT: Size = Size::new(1024.0, 768.0);

    fn child(label: &'static str) -> Element<'static, Msg> {
        button(text(label)).on_press(Msg::Pressed(label)).into()
    }

    fn renderer() -> iced::Renderer {
        iced_test::futures::futures::executor::block_on(
            <iced::Renderer as iced::advanced::renderer::Headless>::new(
                iced::Font::with_name("Fira Sans"),
                iced::Pixels(16.0),
                None
            )
        )
        .expect("a headless renderer is available")
    }

    /// Runs one frame of `row` over the tree `cache` carries, returning the
    /// messages it published and the cache the next frame departs from.
    ///
    /// A slide is only visible across two frames: the first seats the
    /// children and writes the book of settled seats, the second reads it
    /// back as the places the journey departs from. One simulator cannot
    /// show that, because it holds one element for its whole life.
    fn frame(
        row: SlidingRow<'static, Msg>,
        cache: Cache,
        cursor: mouse::Cursor,
        events: &[Event]
    ) -> (Cache, Vec<Msg>) {
        let mut renderer = renderer();
        let mut interface =
            UserInterface::build(Element::new(row), VIEWPORT, cache, &mut renderer);

        let mut messages = Vec::new();
        let _ = interface.update(
            events,
            cursor,
            &mut renderer,
            &mut clipboard::Null,
            &mut messages
        );
        interface.draw(&mut renderer, &Theme::Dark, &Style::default(), cursor);

        (interface.into_cache(), messages)
    }

    fn resting_row() -> SlidingRow<'static, Msg> {
        SlidingRow::new(SPACING, 1.0)
            .push(1, child("A"))
            .push(2, child("B"))
    }

    /// The same two children, swapped, part way through their journey.
    fn swapped_row(progress: f32) -> SlidingRow<'static, Msg> {
        SlidingRow::new(SPACING, progress)
            .push(2, child("B"))
            .push(1, child("A"))
    }

    fn a_click() -> Vec<Event> {
        iced_test::simulator::click().collect()
    }

    fn seats(row: SlidingRow<'static, Msg>, labels: &[&str]) -> Vec<Rectangle> {
        let mut ui = simulator(Element::new(row));

        labels
            .iter()
            .map(|label| {
                ui.find(*label)
                    .expect("every child carries its label")
                    .visible_bounds()
                    .expect("a seated child is visible")
            })
            .collect()
    }

    #[test]
    fn children_are_seated_left_to_right_with_the_spacing_between_them() {
        let seats = seats(resting_row(), &["A", "B"]);

        assert!(seats[0].x < seats[1].x);
        assert!(seats[1].x - (seats[0].x + seats[0].width) >= SPACING - 1.0);
    }

    #[test]
    fn an_empty_row_measures_nothing() {
        let row: SlidingRow<'static, Msg> = SlidingRow::new(SPACING, 1.0);
        let (cache, messages) = frame(row, Cache::default(), mouse::Cursor::Unavailable, &[]);

        assert!(messages.is_empty());
        drop(cache);
    }

    #[test]
    fn a_resting_child_answers_a_press_where_it_sits() {
        let mut ui = simulator(Element::new(resting_row()));
        let _ = ui.click("B").expect("the second child is there");

        let published: Vec<Msg> = ui.into_messages().collect();
        assert_eq!(published, vec![Msg::Pressed("B")]);
    }

    #[test]
    fn a_resting_row_leaves_every_child_where_it_sits() {
        let row = resting_row();
        let state = State::default();

        assert_eq!(row.offset(&state, 0, 40.0), 0.0);
    }

    #[test]
    fn a_travelling_child_is_drawn_part_way_from_where_it_was() {
        let row = swapped_row(0.25);
        let mut state = State::default();
        state.from.insert(2, 100.0);

        assert_eq!(row.offset(&state, 0, 20.0), 60.0);
    }

    #[test]
    fn a_child_nobody_remembers_does_not_travel() {
        let row = swapped_row(0.5);
        let state = State::default();

        assert_eq!(row.offset(&state, 0, 20.0), 0.0);
    }

    #[test]
    fn the_settled_seats_of_one_frame_are_where_the_next_slide_departs_from() {
        let (cache, _) = frame(
            resting_row(),
            Cache::default(),
            mouse::Cursor::Unavailable,
            &[]
        );

        let (cache, messages) = frame(
            swapped_row(0.5),
            cache,
            mouse::Cursor::Available(Point::new(1.0, 1.0)),
            &[]
        );

        assert!(messages.is_empty());
        drop(cache);
    }

    #[test]
    fn a_gliding_child_is_hit_where_it_is_drawn_not_where_it_will_sit() {
        let settled = seats(resting_row(), &["A", "B"]);

        let (cache, _) = frame(
            resting_row(),
            Cache::default(),
            mouse::Cursor::Unavailable,
            &[]
        );

        let drawn = Point::new(settled[1].center().x, settled[1].center().y);
        let (_, messages) = frame(
            swapped_row(0.0),
            cache,
            mouse::Cursor::Available(drawn),
            &a_click()
        );

        assert_eq!(messages, vec![Msg::Pressed("B")]);
    }

    #[test]
    fn a_row_states_its_keys_and_journey_when_printed() {
        let printed = format!("{:?}", swapped_row(0.5));

        assert!(printed.contains('2'));
        assert!(printed.contains("0.5"));
    }
}
