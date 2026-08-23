//! Walks of the declared layout: actions by index and subscriptions.

use hydebar_core::{config::ModuleDef, modules::OnModulePress};
use iced::{Subscription, SurfaceId as Id};

use crate::app::state::{App, Message};

impl App {
    #[must_use]
    /// The entry standing at `index` in the whole bar, drawn for the keyboard.
    ///
    /// Counted across the three sections in the order they are written, which
    /// is the order the keyboard selection walks them in.
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

    #[must_use]
    /// Every stream the entries of one section produce on their own.
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
