//! Assembly of the module rows and subscriptions of a bar section.

use hydebar_core::{components::push_maybe::PushMaybe, config::ModuleDef, modules::OnModulePress};
use iced::{Alignment, Element, Length, Subscription, SurfaceId as Id, widget::row};

use crate::app::state::{App, Message};

impl App {
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

    pub fn modules_section(
        &self,
        modules_def: &[ModuleDef],
        id: Id,
        opacity: f32
    ) -> Element<'_, Message> {
        let mut row = row!()
            .height(Length::Shrink)
            .align_y(Alignment::Center)
            .spacing(self.appearance().island_gap());

        for module_def in modules_def {
            row = row.push_maybe(match module_def {
                ModuleDef::Single(module) => self.single_module_wrapper(module, id, opacity),
                ModuleDef::Group(group) => self.group_module_wrapper(group, id, opacity)
            });
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
