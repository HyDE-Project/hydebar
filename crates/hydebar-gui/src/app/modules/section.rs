//! Assembly of the module rows and subscriptions of a bar section.

use hydebar_core::{components::push_maybe::PushMaybe, config::ModuleDef, modules::OnModulePress};
use iced::{Alignment, Element, Length, Subscription, SurfaceId as Id, widget::row};

use crate::app::state::{App, Message};

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
    fn swept_island<'a>(
        &self,
        island: Element<'a, Message>,
        position: f32,
        themes: &mut std::collections::HashMap<(u32, u32), iced::Theme>
    ) -> Element<'a, Message> {
        let palette_local = self.appearance_transition.is_animating().then(|| {
            hydebar_core::animation::sweep(
                self.appearance_transition.progress(),
                position,
                self.sweep.spread
            )
        });

        let arrival = hydebar_core::animation::sweep(
            self.entrance.value().clamp(0.0, 1.0),
            position,
            self.sweep.spread
        );

        if palette_local.is_none() && arrival >= 1.0 {
            return island;
        }

        let key = (
            palette_local.unwrap_or(f32::NAN).to_bits(),
            arrival.to_bits()
        );
        let theme = themes
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

    #[must_use]
    pub fn get_module_at_index(
        &self,
        index: usize,
        window_id: Id
    ) -> Option<OnModulePress<Message>> {
        use hydebar_core::config::{ModuleDef, ModuleName};

        let mut current_index = 0;
        let sections = [
            &self.config.modules.left[..],
            &self.config.modules.center[..],
            &self.config.modules.right[..]
        ];

        for section in sections {
            for module_def in section {
                let modules_in_def: Vec<&ModuleName> = match module_def {
                    ModuleDef::Single(m) => vec![m],
                    ModuleDef::Group(group) => group.iter().collect()
                };

                for module_name in modules_in_def {
                    if current_index == index
                        && let Some((_, action)) =
                            self.get_module_view(module_name, window_id, 1.0)
                    {
                        return action;
                    }
                    current_index += 1;
                }
            }
        }

        None
    }

    /// Islands the whole layout declares, counted the way the sweep places
    /// them.
    pub(super) fn island_count(&self) -> usize {
        self.config.modules.left.len()
            + self.config.modules.center.len()
            + self.config.modules.right.len()
    }

    /// Builds one bar section, its islands numbered on from `island_offset`.
    ///
    /// The offset threads the bar-wide island position through to the theme
    /// sweep, so a travelling palette crosses the sections as one front
    /// instead of restarting in each.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "island counts are tiny and far below f32 precision limits"
    )]
    pub fn modules_section<'a>(
        &'a self,
        modules_def: &'a [ModuleDef],
        id: Id,
        opacity: f32,
        island_offset: usize
    ) -> Element<'a, Message> {
        let mut row = row!()
            .height(Length::Shrink)
            .align_y(Alignment::Center)
            .spacing(self.appearance().island_gap());

        let total = self.island_count().max(1) as f32;
        let mut themes = std::collections::HashMap::new();

        for (index, module_def) in modules_def.iter().enumerate() {
            let island = match module_def {
                ModuleDef::Single(module) => self.single_module_wrapper(module, id, opacity),
                ModuleDef::Group(group) => self.group_module_wrapper(group, id, opacity)
            };

            let ordinal = ((island_offset + index) as f32 + 0.5) / total;
            let position = if self.sweep.from_left {
                ordinal
            } else {
                1.0 - ordinal
            };

            row = row
                .push_maybe(island.map(|island| self.swept_island(island, position, &mut themes)));
        }

        row.into()
    }

    #[must_use]
    pub fn modules_subscriptions(&self, modules_def: &[ModuleDef]) -> Vec<Subscription<Message>> {
        let mut subscriptions = Vec::new();

        for module_def in modules_def {
            match module_def {
                ModuleDef::Single(module) => {
                    if let Some(subscription) = self.get_module_subscription(module) {
                        subscriptions.push(subscription);
                    }
                }
                ModuleDef::Group(group) => {
                    for module in group {
                        if let Some(subscription) = self.get_module_subscription(module) {
                            subscriptions.push(subscription);
                        }
                    }
                }
            }
        }

        subscriptions
    }
}
