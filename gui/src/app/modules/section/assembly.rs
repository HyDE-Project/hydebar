//! Assembly of the module strip of one bar section.

use hydebar_core::config::ModuleDef;
use iced::{Element, SurfaceId as Id};

use crate::app::state::{App, Message};

impl App {
    /// Islands the whole layout declares, counted the way the sweep places
    /// them.
    fn island_count(&self) -> usize {
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
        use hydebar_core::components::archipelago::{Archipelago, PillPaint};
        use hydebar_proto::config::AppearanceStyle;

        let appearance = self.appearance();
        let style = appearance.style;
        let opacity_paint = appearance.opacity;
        let finish = hydebar_core::style::IslandFinish::of(appearance);
        let radius = appearance.pill_radius();

        let mut strip = Archipelago::new(
            appearance.island_gap(),
            appearance.island_padding()[1],
            self.relayout.value().clamp(0.0, 1.0),
            &self.flip,
            move |theme: &iced::Theme| match style {
                AppearanceStyle::Islands => Some(PillPaint {
                    background: theme.palette().background.scale_alpha(opacity_paint),
                    border:     finish.border(radius),
                    shadow:     finish.shadow()
                }),
                AppearanceStyle::Solid | AppearanceStyle::Gradient => None
            }
        );

        let total = self.island_count().max(1) as f32;
        let mut island_index = 0usize;
        let mut occurrences: std::collections::HashMap<hydebar_core::config::ModuleName, u64> =
            std::collections::HashMap::new();

        for (index, module_def) in modules_def.iter().enumerate() {
            let names: Vec<&hydebar_core::config::ModuleName> = match module_def {
                ModuleDef::Single(module) => vec![module],
                ModuleDef::Group(group) => group.iter().collect()
            };

            let ordinal = ((island_offset + index) as f32 + 0.5) / total;
            let position = if self.sweep.from_left {
                ordinal
            } else {
                1.0 - ordinal
            };

            let mut seated = false;

            for module_name in names {
                if let Some((content, action)) = self.get_module_view(module_name, id, opacity) {
                    let actions = self.module_actions(module_name, action);
                    let element = self.module_element(content, actions, module_name, id, true);
                    let element = self.with_tooltip(module_name, element, id);
                    let element = self.swept_island(element, position);

                    let occurrence = *occurrences
                        .entry(module_name.clone())
                        .and_modify(|count| *count += 1)
                        .or_insert(0u64);

                    strip = strip.push(
                        self.flip_key(module_name, id).wrapping_add(occurrence),
                        island_index,
                        self.entrance
                            .value()
                            .clamp(0.0, 1.0)
                            .min(hydebar_core::animation::sweep(
                                self.entrance.value().clamp(0.0, 1.0),
                                position,
                                self.sweep.spread
                            )),
                        element
                    );
                    seated = true;
                }
            }

            if seated {
                island_index += 1;
            }
        }

        strip.into()
    }
}
