//! Drawing of the tree: rows, unfolding branches and dividing lines.

use iced::{
    Alignment, Element, Length, SurfaceId,
    widget::{Column, button, container, row, rule}
};

use super::{Entry, HydeMenu, Message};
use crate::{
    components::{
        icons::{IconTheme, Icons, icon, icon_raw},
        scale,
        text::text
    },
    menu::MenuType,
    modules::OnModulePress,
    style::ghost_button_style
};

impl HydeMenu {
    /// The bar entry: the glyph the desktop's own module states, or the
    /// bar's menu icon where it states none.
    ///
    /// Rendered by the module itself, so the bar layer holds no menu drawing
    /// of its own.
    #[must_use]
    pub fn bar_view<M>(
        &self,
        icons: &IconTheme
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static
    {
        let entry = match self.glyph.as_deref().map(str::trim) {
            Some(glyph) if !glyph.is_empty() => icon_raw(glyph.to_owned()),
            _ => icon(icons, Icons::MenuOpen)
        };

        Some((
            entry.into(),
            Some(OnModulePress::ToggleMenu(MenuType::HydeMenu))
        ))
    }
}

/// Indent each nesting level adds, in pixels of the reference theme.
const INDENT: f32 = 16.0;

/// Vertical padding of one row, in pixels of the reference theme.
const ROW_PADDING: f32 = 6.0;

/// The whole tree.
pub(super) fn tree_view<'a>(
    tree: &'a [Entry],
    expanded: &std::collections::HashSet<String>,
    surface: SurfaceId,
    opacity: f32
) -> Element<'a, Message> {
    let mut column = Column::new().spacing(scale::scaled(2.0));

    for entry in tree {
        column = column.push(entry_view(entry, expanded, surface, opacity, 0));
    }

    column.into()
}

/// One entry at `depth` levels of indent.
fn entry_view<'a>(
    entry: &'a Entry,
    expanded: &std::collections::HashSet<String>,
    surface: SurfaceId,
    opacity: f32,
    depth: u16
) -> Element<'a, Message> {
    let indent = scale::scaled(INDENT) * f32::from(depth);

    match entry {
        Entry::Separator => container(rule::horizontal(1))
            .padding([scale::scaled(2.0), indent])
            .into(),
        Entry::Item {
            id,
            label
        } => row_button(
            label,
            None,
            Message::Run(surface, id.clone()),
            opacity,
            indent
        ),
        Entry::Submenu {
            id,
            label,
            children
        } => {
            let unfolded = expanded.contains(id);
            let head = row_button(
                label,
                Some(if unfolded {
                    Icons::MenuClosed
                } else {
                    Icons::RightChevron
                }),
                Message::Toggle(id.clone()),
                opacity,
                indent
            );

            if !unfolded {
                return head;
            }

            let mut branch = Column::new().push(head);

            for child in children {
                branch = branch.push(entry_view(child, expanded, surface, opacity, depth + 1));
            }

            branch.into()
        }
    }
}

/// One row of the menu: label left, an optional chevron right.
fn row_button(
    label: &str,
    chevron: Option<Icons>,
    message: Message,
    opacity: f32,
    indent: f32
) -> Element<'_, Message> {
    let mut content = row![text(label).width(Length::Fill)].align_y(Alignment::Center);

    if let Some(glyph) = chevron {
        content = content.push(icon_raw(glyph.default_glyph().to_owned()));
    }

    button(content)
        .style(ghost_button_style(opacity))
        .padding([scale::scaled(ROW_PADDING), scale::scaled(8.0) + indent])
        .width(Length::Fill)
        .on_press(message)
        .into()
}
