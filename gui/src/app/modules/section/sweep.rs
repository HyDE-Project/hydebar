//! The palette front travelling across the islands of the bar.

use iced::Element;

use crate::app::state::{App, Message};

/// Snaps a fade share to a sixty-fourth.
///
/// Neighbouring islands land on near-identical shares mid-sweep; snapping
/// them to one step lets a single derived theme serve them all, and a step
/// of one sixty-fourth is beneath what the eye tells apart.
fn quantized(share: f32) -> f32 {
    (share * 64.0).round() / 64.0
}

impl App {
    /// Wraps an island in the palette of its place under the travelling front.
    ///
    /// Two fronts share the wrap: the palette of a running theme change, and
    /// the birth of the bar, where each island fades in as the entrance wave
    /// reaches it. Both travel with the signature of the theme in force.
    ///
    /// `themes` memoises one section pass: with the fronts spread out, most
    /// islands sit at exactly the resting or the arrived end of the travel and
    /// share one derivation instead of each paying for a palette of its own.
    pub(super) fn swept_island<'a>(
        &self,
        island: Element<'a, Message>,
        position: f32
    ) -> Element<'a, Message> {
        let palette_local = self.appearance_transition.is_animating().then(|| {
            quantized(hydebar_core::animation::sweep(
                self.appearance_transition.progress(),
                position,
                self.sweep.spread
            ))
        });

        let arrival = quantized(hydebar_core::animation::sweep(
            self.entrance.value().clamp(0.0, 1.0),
            position,
            self.sweep.spread
        ));

        if palette_local.is_none() && arrival >= 1.0 {
            return island;
        }

        let key = (
            palette_local.unwrap_or(f32::NAN).to_bits(),
            arrival.to_bits()
        );
        let theme = self
            .derived_themes
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| {
                let base = palette_local.map_or_else(
                    || self.theme_cache.clone(),
                    |local| {
                        hydebar_core::style::hydebar_theme(
                            &self.appearance_transition.sample(local)
                        )
                    }
                );

                if arrival < 1.0 {
                    hydebar_core::style::faded_theme(&base, arrival)
                } else {
                    base
                }
            })
            .clone();

        iced::widget::themer(Some(theme), island)
            .text_color(|theme: &iced::Theme| theme.palette().text)
            .into()
    }
}
