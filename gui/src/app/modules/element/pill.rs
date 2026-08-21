//! The pill of a single module: box, padding and button styling.

use hydebar_core::{
    config::{AppearanceStyle, ModuleName},
    position_button::position_button,
    style::module_button_style
};
use iced::{Alignment, Element, Length, SurfaceId as Id, widget::container};

use super::super::{ModuleActions, actions::attach_module_actions};
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
    const fn module_height(grouped: bool) -> Length {
        if grouped {
            Length::Shrink
        } else {
            Length::Fill
        }
    }

    /// The seat key of one module on one screen.
    ///
    /// The screen rides along because the same module stands on every
    /// output's bar, and two outputs must not fight over one seat. The
    /// screen, not the surface: the strip and the canvas of one output are
    /// two surfaces and one seat, which is what lets a module leave the strip
    /// and arrive on the canvas as the same block rather than as two.
    pub(crate) fn flip_key(&self, module_name: &ModuleName, id: Id) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        module_name.hash(&mut hasher);
        self.outputs.screen_of(id).flatten().hash(&mut hasher);
        hasher.finish()
    }

    /// Renders the pill of a single module, with or without its button.
    ///
    /// A `grouped` module is drawn inside the island of its group, which
    /// already paints the background and the rounded corners for it.
    ///
    /// An ungrouped button is an island of its own: its sides carry the island
    /// padding, or its content would sit five times closer to the pill edge
    /// than the same module would inside a group.
    pub(crate) fn module_element<'a>(
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
                            let finish = hydebar_core::style::IslandFinish::of(self.appearance());

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
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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
