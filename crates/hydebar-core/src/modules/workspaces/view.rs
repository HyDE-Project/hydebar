//! Drawing of the workspace indicators as one row of buttons.

use iced::{
    Element, Length, Padding, SurfaceId as Id, alignment,
    widget::{Row, button, container}
};

use super::{Message, Workspaces};
use crate::{
    components::text::text,
    config::{
        Appearance, MODULE_VERTICAL_PADDING_EM, WORKSPACE_ACTIVE_MARGIN_EM,
        WORKSPACE_ACTIVE_PADDING_EM, WORKSPACE_GAP_EM, WORKSPACE_GLYPH_ADVANCE_EM,
        WORKSPACE_MIN_HEIGHT_EM, WORKSPACE_MIN_WIDTH_EM, WORKSPACE_PADDING_EM,
        WorkspaceVisibilityMode, WorkspacesModuleConfig
    },
    outputs::Outputs,
    style::workspace_button_style
};

/// Returns the width the label box of a workspace indicator reserves.
///
/// The reference waybar theme never lets a workspace button shrink below the
/// minimum width its GTK stylesheet reserves for a button, so a short label
/// sits in a fixed box and the indicators keep the same rhythm whatever they
/// read. A label already wider than that minimum sizes itself instead, which
/// keeps a long special workspace name from being squeezed.
fn label_box_width(label: &str, glyph_advance: f32, min_width: f32) -> Length {
    #[expect(
        clippy::cast_precision_loss,
        reason = "workspace labels are a handful of glyphs, far inside f32's mantissa"
    )]
    let natural = glyph_advance * label.chars().count() as f32;

    if natural < min_width {
        Length::Fixed(min_width)
    } else {
        Length::Shrink
    }
}

impl Workspaces {
    /// The row of indicators for the monitor the surface sits on.
    ///
    /// Colors are looked up by monitor index, safely: a workspace whose
    /// monitor index is unknown, or whose index lies past the configured
    /// palette, simply draws with no color of its own.
    pub(super) fn indicators_row<M>(
        &self,
        outputs: &Outputs,
        id: Id,
        config: &WorkspacesModuleConfig,
        appearance: &Appearance
    ) -> Element<'static, M>
    where
        M: 'static + Clone + From<Message>
    {
        let monitor_name = outputs.get_monitor_name(id);

        let radius = appearance.pill_radius();
        let font_size = appearance.font_size_px();
        let vertical_padding = appearance.spacing(MODULE_VERTICAL_PADDING_EM);
        let idle_padding = appearance.spacing(WORKSPACE_PADDING_EM);
        let active_padding = appearance.spacing(WORKSPACE_ACTIVE_PADDING_EM);
        let active_margin = appearance.spacing(WORKSPACE_ACTIVE_MARGIN_EM);
        let min_label_width = appearance.spacing(WORKSPACE_MIN_WIDTH_EM);
        let min_height = appearance.spacing(WORKSPACE_MIN_HEIGHT_EM);
        let glyph_advance = appearance.spacing(WORKSPACE_GLYPH_ADVANCE_EM);
        let workspace_colors = appearance.workspace_colors.as_slice();
        let special_workspace_colors = appearance.special_workspace_colors.as_deref();

        Row::with_children(
            self.items
                .iter()
                .filter_map(|w| {
                    let on_this_screen = monitor_name.is_none_or(|name| w.monitor == name);

                    if config.visibility_mode == WorkspaceVisibilityMode::All
                        || on_this_screen
                        || !outputs.has_name(&w.monitor)
                    {
                        let empty = w.windows == 0;
                        let monitor = w.monitor_id;

                        let color = monitor.map(|m| {
                            if w.id > 0 {
                                workspace_colors.get(m).copied()
                            } else {
                                special_workspace_colors
                                    .unwrap_or(workspace_colors)
                                    .get(m)
                                    .copied()
                            }
                        });

                        let w_id = w.id;
                        let w_name = w.name.clone();
                        let w_active = w.active;

                        let side_padding = if w_active {
                            active_padding
                        } else {
                            idle_padding
                        };

                        let label = if w_id < 0 { w_name } else { w_id.to_string() };
                        let label_width = label_box_width(&label, glyph_advance, min_label_width);

                        let indicator = button(
                            container(text(label).size(font_size))
                                .width(label_width)
                                .align_x(alignment::Horizontal::Center)
                                .align_y(alignment::Vertical::Center)
                        )
                        .style(workspace_button_style(
                            empty, w_active, w.urgent, radius, color
                        ))
                        .padding([vertical_padding, side_padding])
                        .on_press(if w_id > 0 {
                            Message::ChangeWorkspace(w_id)
                        } else {
                            Message::ToggleSpecialWorkspace(w_id)
                        })
                        .width(Length::Shrink)
                        .height(Length::Fixed(min_height));

                        Some(if w_active {
                            container(indicator)
                                .padding(Padding::ZERO.left(active_margin).right(active_margin))
                                .into()
                        } else {
                            Element::from(indicator)
                        })
                    } else {
                        None
                    }
                })
                .map(|elem: Element<'_, Message>| elem.map(M::from))
        )
        .spacing(appearance.spacing(WORKSPACE_GAP_EM))
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_labels_fill_the_minimum_box() {
        // The reference theme resolves to a 6px glyph advance and a 16px minimum.
        assert_eq!(label_box_width("1", 6.0, 16.0), Length::Fixed(16.0));
        assert_eq!(label_box_width("10", 6.0, 16.0), Length::Fixed(16.0));
        assert_eq!(label_box_width("100", 6.0, 16.0), Length::Shrink);
        assert_eq!(label_box_width("scratch", 6.0, 16.0), Length::Shrink);
    }
}
