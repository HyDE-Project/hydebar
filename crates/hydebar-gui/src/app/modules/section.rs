//! Assembly of the module rows and subscriptions of a bar section.

use hydebar_core::{components::push_maybe::PushMaybe, config::ModuleDef, modules::OnModulePress};
use iced::{Alignment, Element, Length, Subscription, SurfaceId as Id, widget::row};

use crate::app::state::{App, Message};

impl App {
    /// Theme of the island standing at `position` while a theme change runs.
    ///
    /// `position` is zero at the corner the front starts from and one at the
    /// far end; which corner leads, how wide the front is and how it moves all
    /// come from the signature of the incoming theme. [`None`] whenever the
    /// palette rests, which is almost always, so the sweep costs nothing
    /// outside the frames it actually travels.
    fn sweep_theme(&self, position: f32) -> Option<iced::Theme> {
        if !self.appearance_transition.is_animating() {
            return None;
        }

        let local = hydebar_core::animation::sweep(
            self.appearance_transition.progress(),
            position,
            self.sweep.spread
        );

        Some(hydebar_core::style::hydebar_theme(
            &self.appearance_transition.sample(local)
        ))
    }

    /// Wraps an island in the palette of its place under the travelling front.
    fn swept_island<'a>(
        &self,
        island: Element<'a, Message>,
        position: f32
    ) -> Element<'a, Message> {
        match self.sweep_theme(position) {
            Some(theme) => iced::widget::themer(Some(theme), island)
                .text_color(|theme: &iced::Theme| theme.palette().text)
                .into(),
            None => island
        }
    }

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

            row = row.push_maybe(island.map(|island| self.swept_island(island, position)));
        }

        row.into()
    }

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
