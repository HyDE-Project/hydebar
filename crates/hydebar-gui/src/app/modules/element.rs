//! Island and pill wrapping of the modules drawn in a bar section.

use hydebar_core::{
    config::{AppearanceStyle, ModuleName},
    modules::battery::BatteryData,
    position_button::position_button,
    style::module_button_style,
    tooltip::{TooltipInfo, tooltip_anchor}
};
use iced::{
    Alignment, Element, Length, SurfaceId as Id,
    widget::{Row, container}
};

use super::{ModuleActions, actions::attach_module_actions};
use crate::app::state::{App, Message};

/// States the battery in one line: charge, and whether it is being fed.
fn battery_hint(data: &BatteryData) -> String {
    if data.charging {
        format!("Battery: {}%, charging", data.capacity)
    } else {
        format!("Battery: {}%", data.capacity)
    }
}

impl App {
    /// Padding of a single module, derived from the themed font size.
    fn module_padding(&self) -> [f32; 2] {
        self.appearance().module_padding()
    }

    /// Height the box of a module reserves inside the bar row.
    ///
    /// An ungrouped module is an island of its own, so it fills the row and
    /// paints the pill every other island paints. A grouped module shares the
    /// island of its group and only owns the box its own content needs: were it
    /// to fill the island it would paint its hover over the island padding and
    /// against the rounded corners the island draws.
    fn module_height(grouped: bool) -> Length {
        if grouped {
            Length::Shrink
        } else {
            Length::Fill
        }
    }

    pub(super) fn single_module_wrapper<'a>(
        &'a self,
        module_name: &'a ModuleName,
        id: Id,
        opacity: f32
    ) -> Option<Element<'a, Message>> {
        let module = self
            .get_module_view(module_name, id, opacity)
            .map(|(content, action)| (content, self.module_actions(module_name, action)));

        module.map(|(content, actions)| {
            let module = self.module_element(content, actions, module_name, id, false);

            self.with_tooltip(module_name, module, id)
        })
    }

    /// Renders the pill of a single module, with or without its button.
    ///
    /// A `grouped` module is drawn inside the island of its group, which
    /// already paints the background and the rounded corners for it.
    fn module_element<'a>(
        &'a self,
        content: Element<'a, Message>,
        actions: ModuleActions,
        module_name: &ModuleName,
        id: Id,
        grouped: bool
    ) -> Element<'a, Message> {
        match actions.is_inert() {
            false => {
                let height = Self::module_height(grouped);

                // an ungrouped button is an island of its own: its sides carry
                // the island padding, or its content sits five times closer
                // to the pill edge than the same module would inside a group
                let padding = if grouped {
                    self.module_padding()
                } else if self.appearance().style == AppearanceStyle::Islands {
                    [
                        self.module_padding()[0],
                        self.appearance().island_padding()[1]
                    ]
                } else {
                    self.module_padding()
                };

                let button =
                    position_button(container(content).align_y(Alignment::Center).height(height))
                        .padding(padding)
                        .height(height)
                        .style(module_button_style(
                            self.appearance().style,
                            self.appearance().opacity,
                            self.appearance().pill_radius(),
                            grouped,
                            false,
                            self.hover.progress(module_name),
                            hydebar_core::style::IslandFinish::of(self.appearance())
                        ));

                attach_module_actions(button, actions, id).into()
            }
            _ if grouped => container(content)
                .padding(self.module_padding())
                .height(Self::module_height(grouped))
                .align_y(Alignment::Center)
                .into(),
            _ => {
                let padding = match self.appearance().style {
                    AppearanceStyle::Islands => self.appearance().island_padding(),
                    _ => self.module_padding()
                };

                let container = container(content)
                    .padding(padding)
                    .height(Length::Fill)
                    .align_y(Alignment::Center);

                match self.appearance().style {
                    AppearanceStyle::Solid | AppearanceStyle::Gradient => container.into(),
                    AppearanceStyle::Islands => container
                        .style(|theme| {
                            let finish =
                                hydebar_core::style::IslandFinish::of(self.appearance());

                            container::Style {
                                background: Some(
                                    theme
                                        .palette()
                                        .background
                                        .scale_alpha(self.appearance().opacity)
                                        .into()
                                ),
                                border: finish.border(self.appearance().pill_radius()),
                                shadow: finish.shadow(),
                                ..container::Style::default()
                            }
                        })
                        .into()
                }
            }
        }
    }

    /// Hint a module publishes while the pointer rests on it.
    ///
    /// The outer [`Option`] separates a module that never shows a hint, and is
    /// left unwrapped, from one that shows a hint only in some of its states.
    ///
    /// Modules whose facts fit a line state them; the rest state their name,
    /// because a strip of glyphs nobody can name is a bar nobody can learn.
    /// A custom module with no hint of its own states the name it was
    /// configured under, for the same reason. The ones that stay silent
    /// already say everything on the bar itself: workspaces and the window
    /// title are their own text, the tray draws icons the bar does not own,
    /// and the system readouts stand beside their values.
    fn module_tooltip(&self, module_name: &ModuleName) -> Option<Option<String>> {
        match module_name {
            ModuleName::Custom(name) => self.custom.get(name).map(|custom| {
                Some(
                    custom
                        .tooltip()
                        .map(str::to_owned)
                        .unwrap_or_else(|| hydebar_proto::bar_layout::display_label(name))
                )
            }),
            ModuleName::IdleInhibitor => Some(
                self.config
                    .idle_inhibitor
                    .tooltip(self.control_center.is_idle_inhibited())
                    .map(str::to_owned)
            ),
            ModuleName::Battery => Some(self.battery.data().map(battery_hint)),
            ModuleName::Network => Some(Some(
                self.control_center
                    .network_hint()
                    .unwrap_or_else(|| module_name.label().to_owned())
            )),
            ModuleName::Cpu => Some(Some(hydebar_core::modules::cpu::hint(
                self.system_info.data()
            ))),
            ModuleName::Memory => Some(Some(hydebar_core::modules::memory::hint(
                self.system_info.data()
            ))),
            ModuleName::CpuTemp => Some(Some(hydebar_core::modules::cpu_temp::hint(
                self.system_info.data()
            ))),
            ModuleName::GpuTemp => Some(Some(hydebar_core::modules::gpu_temp::hint(
                self.system_info.data()
            ))),
            ModuleName::Clock => Some(Some(self.clock.data().format("%A, %-d %B %Y"))),
            ModuleName::Updates => Some(self.updates.tooltip()),
            ModuleName::KeyboardLayout => Some(Some(format!(
                "{}: {}",
                module_name.label(),
                self.keyboard_layout.active_layout()
            ))),
            ModuleName::Workspaces
            | ModuleName::WindowTitle
            | ModuleName::Tray
            | ModuleName::SystemInfo => None,
            named => Some(Some(named.label().to_owned()))
        }
    }

    /// Wraps a module in the anchor its hover is published from.
    ///
    /// Every module is wrapped, hint or no hint: the pointer resting on a
    /// module is what the bar reads its attention out of, and a module that
    /// shows no tooltip is still something the user can look at. The hint, when
    /// there is one, rides along on the same message rather than on a second
    /// one, because there is only ever one thing being looked at.
    ///
    /// The hint is composed inside the closure on purpose: the closure runs
    /// when the pointer actually enters or leaves, while the wrapper itself
    /// runs for every module on every frame — and a date formatted for a
    /// tooltip nobody hovers is a frame budget spent on nothing.
    fn with_tooltip<'a>(
        &'a self,
        module_name: &'a ModuleName,
        module: Element<'a, Message>,
        id: Id
    ) -> Element<'a, Message> {
        tooltip_anchor(module, move |anchor| Message::ModuleHover {
            surface: id,
            module:  module_name.clone(),
            entered: anchor.is_some(),
            tooltip: anchor
                .zip(anchor.and(self.module_tooltip(module_name).flatten()))
                .map(|(anchor, text)| TooltipInfo {
                    text,
                    anchor
                })
        })
        .into()
    }

    pub(super) fn group_module_wrapper<'a>(
        &'a self,
        group: &'a [ModuleName],
        id: Id,
        opacity: f32
    ) -> Option<Element<'a, Message>> {
        let modules = group
            .iter()
            .filter_map(|module| {
                self.get_module_view(module, id, opacity)
                    .map(|(content, action)| {
                        (module, content, self.module_actions(module, action))
                    })
            })
            .collect::<Vec<_>>();

        if modules.is_empty() {
            None
        } else {
            Some({
                let group = Row::with_children(
                    modules
                        .into_iter()
                        .map(|(module_name, content, actions)| {
                            let module =
                                self.module_element(content, actions, module_name, id, true);

                            self.with_tooltip(module_name, module, id)
                        })
                        .collect::<Vec<_>>()
                )
                .align_y(Alignment::Center);

                match self.appearance().style {
                    AppearanceStyle::Solid | AppearanceStyle::Gradient => group.into(),
                    AppearanceStyle::Islands => container(group)
                        .padding(self.appearance().island_padding())
                        .height(Length::Fill)
                        .align_y(Alignment::Center)
                        .style(|theme| {
                            let finish =
                                hydebar_core::style::IslandFinish::of(self.appearance());

                            container::Style {
                                background: Some(
                                    theme
                                        .palette()
                                        .background
                                        .scale_alpha(self.appearance().opacity)
                                        .into()
                                ),
                                border: finish.border(self.appearance().pill_radius()),
                                shadow: finish.shadow(),
                                ..container::Style::default()
                            }
                        })
                        .into()
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grouped_module_owns_the_box_of_its_own_content() {
        // a grouped module stretched over the island height would hover-paint
        // over the island padding and its rounded corners
        assert_eq!(App::module_height(true), Length::Shrink);
        assert_eq!(App::module_height(false), Length::Fill);
    }
}
