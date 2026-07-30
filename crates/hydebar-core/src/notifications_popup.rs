//! Popups the bar draws for the notifications it serves.
//!
//! The bar already owns the notification bus, so nothing else has to be
//! installed to see a notification: the popups are drawn on a surface of the
//! bar, in the colours and sizes of the bar theme, which is what keeps them
//! consistent with everything else on the screen.

use std::time::{Duration, Instant};

use hydebar_proto::config::{Appearance, Position};
use iced::{
    Background, Border, Element, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Column, container}
};

use crate::{
    components::text::text,
    services::notifications::{Notification, Urgency}
};

/// Width of a popup, in multiples of the themed text size.
const WIDTH_EM: f32 = 20.0;

/// Padding inside a popup, in multiples of the themed text size.
const PADDING_EM: f32 = 0.9;

/// Gap between popups and to the edge, in multiples of the themed text size.
const GAP_EM: f32 = 0.6;

/// How long a popup stays when the sender named no duration.
const DEFAULT_LIFETIME: Duration = Duration::from_secs(5);

/// How long a passing notice stays.
const MIN_LIFETIME: Duration = Duration::from_secs(3);

/// How long an urgent notification stays.
const URGENT_LIFETIME: Duration = Duration::from_secs(12);

/// How many popups are shown at once, oldest dropped first.
const MAX_SHOWN: usize = 4;

/// A notification currently on screen.
#[derive(Debug, Clone)]
pub struct Popup {
    /// Identifier the sender addresses it by.
    pub id:      u32,
    /// Heading of the notification.
    pub summary: String,
    /// Body of the notification, empty when the sender gave none.
    pub body:    String,
    /// When it appeared.
    pub shown:   Instant,
    /// How long it stays.
    pub life:    Duration
}

impl Popup {
    /// Builds the popup for `notification`, shown at `now`.
    ///
    /// An urgent notification stays longer: it is the one a person is most
    /// likely to have missed while looking elsewhere.
    #[must_use]
    pub fn new(notification: &Notification, now: Instant) -> Self {
        Self {
            id:      notification.id,
            summary: notification.summary.clone(),
            body:    notification.body.clone(),
            shown:   now,
            life:    lifetime_for(&notification.urgency)
        }
    }

    /// Whether the popup has outlived its time by `now`.
    #[must_use]
    pub fn expired(&self, now: Instant) -> bool {
        now.duration_since(self.shown) >= self.life
    }
}

/// How long a notification of `urgency` stays on screen.
#[must_use]
pub fn lifetime_for(urgency: &Urgency) -> Duration {
    match urgency {
        Urgency::Low => MIN_LIFETIME,
        Urgency::Normal => DEFAULT_LIFETIME,
        Urgency::Critical => URGENT_LIFETIME
    }
}

/// Whether a clock has to run to take the popups down.
///
/// The rest of the bar redraws only when something changes, so a popup left
/// alone would sit on screen for ever; a clock is asked for exactly while
/// something is on screen, which keeps an idle bar costing nothing.
#[must_use]
pub fn needs_expiry_clock(popups: &[Popup]) -> bool {
    !popups.is_empty()
}

/// Drops the popups that have outlived their time and keeps the newest ones.
pub fn prune(popups: &mut Vec<Popup>, now: Instant) {
    popups.retain(|popup| !popup.expired(now));

    while popups.len() > MAX_SHOWN {
        popups.remove(0);
    }
}

/// Height the surface needs for `popups`, in pixels.
///
/// The surface is grown to exactly what the popups need and shrunk back once
/// they are gone: a surface left standing at full height would swallow every
/// click aimed at the windows behind it.
#[must_use]
pub fn surface_height(popups: &[Popup], appearance: &Appearance) -> u32 {
    if popups.is_empty() {
        return 1;
    }

    let font_size = appearance.font_size_px();
    let gap = GAP_EM * font_size;

    let card: f32 = popups
        .iter()
        .map(|popup| {
            let lines = if popup.body.is_empty() { 1.0 } else { 2.2 };

            lines * font_size * 1.4 + PADDING_EM * font_size * 2.0
        })
        .sum();

    let gaps = gap * (popups.len().saturating_sub(1)) as f32 + gap * 2.0;

    (card + gaps).ceil().max(1.0) as u32
}

/// Renders the stack of popups for the surface they live on.
pub fn view<'a, Message: 'a>(
    popups: &[Popup],
    appearance: &Appearance,
    bar_position: Position
) -> Element<'a, Message> {
    let font_size = appearance.font_size_px();
    let gap = GAP_EM * font_size;
    let radius = appearance.pill_radius();
    let opacity = appearance.menu.opacity;

    let mut stack = Column::new().spacing(gap).width(Length::Shrink);

    for popup in popups {
        let mut body = Column::new()
            .push(text(popup.summary.clone()).size(font_size))
            .spacing(gap * 0.4);

        if !popup.body.is_empty() {
            body = body.push(text(popup.body.clone()).size(font_size * 0.85));
        }

        stack = stack.push(
            container(body)
                .padding(PADDING_EM * font_size)
                .width(Length::Fixed(WIDTH_EM * font_size))
                .style(move |theme: &Theme| container::Style {
                    background: Some(Background::Color(
                        theme.palette().background.scale_alpha(opacity)
                    )),
                    text_color: Some(theme.palette().text),
                    border: Border {
                        width:  1.0,
                        radius: radius.into(),
                        color:  theme
                            .extended_palette()
                            .secondary
                            .strong
                            .color
                            .scale_alpha(opacity)
                    },
                    ..container::Style::default()
                })
        );
    }

    container(stack)
        .padding(gap)
        .align_x(Horizontal::Right)
        .align_y(match bar_position {
            Position::Top => Vertical::Top,
            Position::Bottom => Vertical::Bottom
        })
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn popup(id: u32, life: Duration, shown: Instant) -> Popup {
        Popup {
            id,
            summary: format!("summary {id}"),
            body: String::new(),
            shown,
            life
        }
    }

    #[test]
    fn an_empty_stack_keeps_the_surface_out_of_the_way() {
        let appearance = Appearance::default();

        assert_eq!(surface_height(&[], &appearance), 1);
    }

    #[test]
    fn more_popups_need_more_room() {
        let now = Instant::now();
        let appearance = Appearance::default();
        let one = vec![popup(1, Duration::from_secs(5), now)];
        let two = vec![
            popup(1, Duration::from_secs(5), now),
            popup(2, Duration::from_secs(5), now),
        ];

        assert!(surface_height(&two, &appearance) > surface_height(&one, &appearance));
    }

    #[test]
    fn a_popup_with_a_body_is_taller() {
        let now = Instant::now();
        let appearance = Appearance::default();
        let plain = vec![popup(1, Duration::from_secs(5), now)];
        let mut with_body = plain.clone();
        with_body[0].body = "body".to_owned();

        assert!(surface_height(&with_body, &appearance) > surface_height(&plain, &appearance));
    }

    #[test]
    fn an_urgent_notification_stays_the_longest() {
        assert!(lifetime_for(&Urgency::Critical) > lifetime_for(&Urgency::Normal));
        assert!(lifetime_for(&Urgency::Normal) > lifetime_for(&Urgency::Low));
    }

    #[test]
    fn a_popup_expires_once_its_time_is_up() {
        let now = Instant::now();
        let popup = popup(1, Duration::from_secs(3), now);

        assert!(!popup.expired(now + Duration::from_secs(2)));
        assert!(popup.expired(now + Duration::from_secs(3)));
    }

    #[test]
    fn expired_popups_are_dropped() {
        let now = Instant::now();
        let mut popups = vec![
            popup(1, Duration::from_secs(1), now),
            popup(2, Duration::from_secs(9), now),
        ];

        prune(&mut popups, now + Duration::from_secs(2));

        assert_eq!(popups.len(), 1);
        assert_eq!(popups[0].id, 2);
    }

    #[test]
    fn pruning_takes_down_the_overdue_card_and_leaves_the_fresh_one() {
        let now = Instant::now();
        let mut popups = vec![
            popup(7, Duration::from_secs(3), now),
            popup(8, Duration::from_secs(30), now + Duration::from_secs(3)),
        ];

        prune(&mut popups, now + Duration::from_secs(4));

        assert_eq!(
            popups.iter().map(|popup| popup.id).collect::<Vec<_>>(),
            vec![8]
        );
    }

    #[test]
    fn pruning_keeps_a_card_whose_time_is_not_up_yet() {
        let now = Instant::now();
        let mut popups = vec![popup(1, Duration::from_secs(5), now)];

        prune(&mut popups, now + Duration::from_secs(4));

        assert_eq!(popups.len(), 1);
    }

    #[test]
    fn no_clock_runs_while_nothing_is_on_screen() {
        assert!(!needs_expiry_clock(&[]));
    }

    #[test]
    fn a_clock_runs_as_soon_as_a_card_is_on_screen() {
        let now = Instant::now();

        assert!(needs_expiry_clock(&[popup(1, Duration::from_secs(5), now)]));
    }

    #[test]
    fn the_clock_stops_once_the_last_card_is_taken_down() {
        let now = Instant::now();
        let mut popups = vec![popup(1, Duration::from_secs(1), now)];

        assert!(needs_expiry_clock(&popups));

        prune(&mut popups, now + Duration::from_secs(2));

        assert!(!needs_expiry_clock(&popups));
    }

    #[test]
    fn only_the_newest_popups_are_kept() {
        let now = Instant::now();
        let mut popups = (0..MAX_SHOWN as u32 + 2)
            .map(|id| popup(id, Duration::from_secs(60), now))
            .collect::<Vec<_>>();

        prune(&mut popups, now);

        assert_eq!(popups.len(), MAX_SHOWN);
        assert_eq!(popups[0].id, 2);
    }
}
