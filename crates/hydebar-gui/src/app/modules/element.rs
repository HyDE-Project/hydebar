//! Island and pill wrapping of the modules drawn in a bar section.

use hydebar_core::{
    config::{AppearanceStyle, ModuleName},
    position_button::position_button,
    style::module_button_style,
    tooltip::{TooltipInfo, tooltip_anchor}
};
use iced::{
    Alignment, Border, Color, Element, Length,
    widget::{Row, container},
    window::Id
};

use super::{ModuleActions, actions::attach_module_actions};
use crate::app::state::{App, Message};

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

    pub(super) fn single_module_wrapper(
        &self,
        module_name: &ModuleName,
        id: Id,
        opacity: f32
    ) -> Option<Element<'_, Message>> {
        let module = self
            .get_module_view(module_name, id, opacity)
            .map(|(content, action)| (content, self.module_actions(module_name, action)));

        module.map(|(content, actions)| {
            let module = self.module_element(content, actions, id, false);

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
        id: Id,
        grouped: bool
    ) -> Element<'a, Message> {
        match actions.is_inert() {
            false => {
                let height = Self::module_height(grouped);

                let button =
                    position_button(container(content).align_y(Alignment::Center).height(height))
                        .padding(self.module_padding())
                        .height(height)
                        .style(module_button_style(
                            self.appearance().style,
                            self.appearance().opacity,
                            self.appearance().pill_radius(),
                            grouped,
                            false
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
                        .style(|theme| container::Style {
                            background: Some(
                                theme
                                    .palette()
                                    .background
                                    .scale_alpha(self.appearance().opacity)
                                    .into()
                            ),
                            border: Border {
                                width:  0.0,
                                radius: self.appearance().pill_radius().into(),
                                color:  Color::TRANSPARENT
                            },
                            ..container::Style::default()
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
    fn module_tooltip(&self, module_name: &ModuleName) -> Option<Option<String>> {
        match module_name {
            ModuleName::Custom(name) => self
                .custom
                .get(name)
                .map(|custom| custom.tooltip().map(str::to_owned)),
            ModuleName::IdleInhibitor => Some(
                self.config
                    .idle_inhibitor
                    .tooltip(self.settings.is_idle_inhibited())
                    .map(str::to_owned)
            ),
            _ => None
        }
    }

    /// Wraps a module in the anchor its tooltip is published from.
    ///
    /// A module that publishes hints stays wrapped even while its own hint is
    /// empty, so leaving one always clears whatever the tooltip surface shows.
    fn with_tooltip<'a>(
        &self,
        module_name: &ModuleName,
        module: Element<'a, Message>,
        id: Id
    ) -> Element<'a, Message> {
        let Some(hint) = self.module_tooltip(module_name) else {
            return module;
        };

        tooltip_anchor(module, move |anchor| {
            Message::ModuleTooltip(
                id,
                anchor.zip(hint.clone()).map(|(anchor, text)| TooltipInfo {
                    text,
                    anchor
                })
            )
        })
        .into()
    }

    pub(super) fn group_module_wrapper(
        &self,
        group: &[ModuleName],
        id: Id,
        opacity: f32
    ) -> Option<Element<'_, Message>> {
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
                            let module = self.module_element(content, actions, id, true);

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
                        .style(|theme| container::Style {
                            background: Some(
                                theme
                                    .palette()
                                    .background
                                    .scale_alpha(self.appearance().opacity)
                                    .into()
                            ),
                            border: Border {
                                width:  0.0,
                                radius: self.appearance().pill_radius().into(),
                                color:  Color::TRANSPARENT
                            },
                            ..container::Style::default()
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
