//! The HyDE menu: the desktop's own menu tree, drawn by the bar.
//!
//! The reference waybar opens a GTK menu built from an XML file the HyDE
//! Project ships, and every item runs a command from the module's action
//! table. This module reads the very same two files — nothing about the menu
//! is stated here — and renders the tree in the bar's own window: submenus
//! unfold in place, a leaf runs its command and the window closes.
//!
//! One file, three rooms: [`source`] reads the desktop's files, [`view`]
//! draws the tree, the root holds the state and the messages.

use iced::Element;

use crate::{
    components::icons::IconTheme,
    menu::MenuType,
    modules::{Module, OnModulePress}
};

/// What the user asks of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Unfold or fold the submenu with this identifier.
    Toggle(String),
    /// Run the action with this identifier and close the menu.
    Run(iced::SurfaceId, String)
}

/// One entry of the menu tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A leaf: label shown, action run on press.
    Item { id: String, label: String },
    /// A dividing line between groups.
    Separator,
    /// A named branch that unfolds in place.
    Submenu {
        id:       String,
        label:    String,
        children: Vec<Entry>
    }
}

/// The HyDE menu module.
#[derive(Debug, Default)]
pub struct HydeMenu {
    /// The tree the desktop's menu file describes.
    tree:     Vec<Entry>,
    /// Command per leaf identifier.
    actions:  std::collections::HashMap<String, String>,
    /// Glyph the bar entry shows, as the desktop's module states it.
    glyph:    Option<String>,
    /// Identifiers of the branches currently unfolded.
    expanded: std::collections::HashSet<String>
}

impl HydeMenu {
    /// Re-reads the desktop's files, folding every branch.
    ///
    /// Called when the menu opens, so an edited menu file is picked up
    /// without restarting the bar; the two files are small.
    pub fn reload(&mut self) {
        self.expanded.clear();

        let Some(definition) = source::read_definition() else {
            return;
        };

        self.glyph = definition.glyph;
        self.actions = definition.actions;
        self.tree = definition
            .menu_file
            .as_deref()
            .and_then(source::read_tree)
            .unwrap_or_default();
    }

    /// Applies what the user asked, reporting the command to run, if any.
    pub fn update(&mut self, message: Message) -> Option<(iced::SurfaceId, String)> {
        match message {
            Message::Toggle(id) => {
                if !self.expanded.remove(&id) {
                    self.expanded.insert(id);
                }

                None
            }
            Message::Run(surface, id) => {
                let command = self.actions.get(&id)?.clone();

                Some((surface, command))
            }
        }
    }

    /// The menu tree as the window shows it.
    pub fn menu_view(&self, id: iced::SurfaceId, opacity: f32) -> Element<'_, Message> {
        view::tree_view(&self.tree, &self.expanded, id, opacity)
    }
}

impl<M> Module<M> for HydeMenu
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        _ctx: &crate::ModuleContext,
        _data: Self::RegistrationData<'_>
    ) -> Result<(), super::ModuleError> {
        self.reload();

        Ok(())
    }

    /// Renders the bar entry with the glyph the desktop's module states.
    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        use crate::components::icons::{Icons, icon, icon_raw};

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

/// Reading of the desktop's own files: the module table and the menu tree.
mod source {
    use std::collections::HashMap;

    use hydebar_proto::bar_layout::plain_json;
    use log::warn;
    use quick_xml::{Reader, events::Event};

    use super::Entry;

    /// Everything the waybar module definition states about the menu.
    pub(super) struct Definition {
        pub(super) glyph:     Option<String>,
        pub(super) menu_file: Option<String>,
        pub(super) actions:   HashMap<String, String>
    }

    /// Directory the desktop keeps its waybar assets in.
    fn data_dir() -> Option<std::path::PathBuf> {
        std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".local/share")))
    }

    /// Reads the module definition the desktop ships for its menu.
    pub(super) fn read_definition() -> Option<Definition> {
        let path = data_dir()?.join("waybar/modules/custom-hyde-menu.jsonc");
        let source = std::fs::read_to_string(&path)
            .inspect_err(|err| warn!("cannot read {}: {err}", path.display()))
            .ok()?;
        let parsed: serde_json::Value = serde_json::from_str(&plain_json(&source))
            .inspect_err(|err| warn!("cannot parse {}: {err}", path.display()))
            .ok()?;
        let module = parsed.get("custom/hyde-menu")?;

        let actions = module
            .get("menu-actions")
            .and_then(|value| value.as_object())
            .map(|table| {
                table
                    .iter()
                    .filter_map(|(id, command)| Some((id.clone(), command.as_str()?.to_owned())))
                    .collect()
            })
            .unwrap_or_default();

        Some(Definition {
            glyph: module
                .get("format")
                .and_then(|value| value.as_str())
                .map(str::to_owned),
            menu_file: module
                .get("menu-file")
                .and_then(|value| value.as_str())
                .map(expand_path),
            actions
        })
    }

    /// Expands the `${VAR:-fallback}` and `$VAR` forms the desktop uses.
    ///
    /// Only what the shipped files actually contain: one substitution at the
    /// front of the path is the whole grammar.
    fn expand_path(raw: &str) -> String {
        let expand_var = |name: &str, fallback: Option<&str>| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.is_empty())
                .or_else(|| fallback.map(str::to_owned))
                .unwrap_or_default()
        };

        if let Some(start) = raw.find("${")
            && let Some(end) = raw[start..].find('}')
        {
            let inner = &raw[start + 2..start + end];
            let (name, fallback) = match inner.split_once(":-") {
                Some((name, fallback)) => (name, Some(fallback)),
                None => (inner, None)
            };
            let fallback = fallback.map(|fallback| expand_plain(fallback));

            return format!(
                "{}{}{}",
                &raw[..start],
                expand_var(name, fallback.as_deref()),
                &raw[start + end + 1..]
            );
        }

        expand_plain(raw)
    }

    /// Expands a leading `$VAR` with no braces.
    fn expand_plain(raw: &str) -> String {
        let Some(name) = raw.strip_prefix('$') else {
            return raw.to_owned();
        };

        let (name, rest) = name.split_at(name.find('/').unwrap_or(name.len()));

        match std::env::var(name) {
            Ok(value) if !value.is_empty() => format!("{value}{rest}"),
            _ => data_dir()
                .map(|dir| format!("{}{rest}", dir.display()))
                .unwrap_or_else(|| raw.to_owned())
        }
    }

    /// Kind of `<object>` a closing tag ends, tracked in document order.
    enum ObjectKind {
        Menu,
        Item,
        Other
    }

    /// An item still being assembled: identifier, label, unfolded children.
    struct PendingItem {
        id:       String,
        label:    String,
        children: Option<Vec<Entry>>
    }

    /// Parses the GTK menu XML into the tree the window renders.
    ///
    /// The file is a `GtkMenu` of `GtkMenuItem` objects: an item carries an
    /// `id` and a `label` property, a nested `GtkMenu` inside an item is its
    /// submenu, and `GtkSeparatorMenuItem` draws a line. Nothing else is
    /// read; the `<child>` wrappers carry no information of their own.
    pub(super) fn read_tree(path: &str) -> Option<Vec<Entry>> {
        let source = std::fs::read_to_string(path)
            .inspect_err(|err| warn!("cannot read menu file {path}: {err}"))
            .ok()?;

        let mut reader = Reader::from_str(&source);
        reader.config_mut().trim_text(true);

        let attribute = |tag: &quick_xml::events::BytesStart<'_>, name: &str| {
            tag.try_get_attribute(name)
                .ok()
                .flatten()
                .map(|attribute| String::from_utf8_lossy(&attribute.value).into_owned())
                .unwrap_or_default()
        };

        let mut menus: Vec<Vec<Entry>> = Vec::new();
        let mut items: Vec<PendingItem> = Vec::new();
        let mut objects: Vec<ObjectKind> = Vec::new();
        let mut result: Option<Vec<Entry>> = None;
        let mut in_label = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(tag)) => match tag.name().as_ref() {
                    b"object" => match attribute(&tag, "class").as_str() {
                        "GtkMenu" => {
                            menus.push(Vec::new());
                            objects.push(ObjectKind::Menu);
                        }
                        "GtkMenuItem" => {
                            items.push(PendingItem {
                                id:       attribute(&tag, "id"),
                                label:    String::new(),
                                children: None
                            });
                            objects.push(ObjectKind::Item);
                        }
                        _ => objects.push(ObjectKind::Other)
                    },
                    b"property" => {
                        in_label = attribute(&tag, "name") == "label";
                    }
                    _ => {}
                },
                Ok(Event::Empty(tag))
                    if tag.name().as_ref() == b"object"
                        && attribute(&tag, "class") == "GtkSeparatorMenuItem" =>
                {
                    if let Some(menu) = menus.last_mut() {
                        menu.push(Entry::Separator);
                    }
                }
                Ok(Event::Text(content)) if in_label => {
                    if let Some(item) = items.last_mut() {
                        item.label.push_str(
                            &content
                                .decode()
                                .map(|value| value.into_owned())
                                .unwrap_or_default()
                        );
                    }
                }
                Ok(Event::End(tag)) => match tag.name().as_ref() {
                    b"property" => in_label = false,
                    b"object" => match objects.pop() {
                        Some(ObjectKind::Menu) => {
                            let entries = menus.pop().unwrap_or_default();

                            match items.last_mut() {
                                Some(item) => item.children = Some(entries),
                                None => result = Some(entries)
                            }
                        }
                        Some(ObjectKind::Item) => {
                            if let Some(item) = items.pop()
                                && let Some(menu) = menus.last_mut()
                            {
                                let label = item.label.trim().to_owned();

                                menu.push(match item.children {
                                    Some(children) => Entry::Submenu {
                                        id: item.id,
                                        label,
                                        children
                                    },
                                    None => Entry::Item {
                                        id: item.id,
                                        label
                                    }
                                });
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Err(err) => {
                    warn!("menu file {path} is not readable as XML: {err}");
                    return None;
                }
                _ => {}
            }
        }

        result
    }
}

/// Drawing of the tree: rows, unfolding branches and dividing lines.
mod view {
    use iced::{
        Alignment, Element, Length, SurfaceId,
        widget::{Column, button, container, row, rule}
    };

    use super::{Entry, Message};
    use crate::{
        components::{
            icons::{Icons, icon_raw},
            scale,
            text::text
        },
        style::ghost_button_style
    };

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
    fn row_button<'a>(
        label: &'a str,
        chevron: Option<Icons>,
        message: Message,
        opacity: f32,
        indent: f32
    ) -> Element<'a, Message> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toggle_unfolds_and_folds_the_same_branch() {
        let mut menu = HydeMenu::default();

        assert!(menu.update(Message::Toggle("a".into())).is_none());
        assert!(menu.expanded.contains("a"));

        assert!(menu.update(Message::Toggle("a".into())).is_none());
        assert!(!menu.expanded.contains("a"));
    }

    #[test]
    fn a_run_reports_the_command_of_the_action() {
        let mut menu = HydeMenu::default();
        menu.actions
            .insert("go".into(), "hyde-shell wallpaper --next".into());

        let run = menu.update(Message::Run(iced::SurfaceId::MAIN, "go".into()));

        assert_eq!(
            run.map(|(_, command)| command),
            Some(String::from("hyde-shell wallpaper --next"))
        );
    }

    #[test]
    fn an_unknown_action_runs_nothing() {
        let mut menu = HydeMenu::default();

        assert!(
            menu.update(Message::Run(iced::SurfaceId::MAIN, "gone".into()))
                .is_none()
        );
    }
}
