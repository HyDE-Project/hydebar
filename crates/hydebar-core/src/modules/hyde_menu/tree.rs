//! Parsing of the desktop's GTK menu XML into the tree the window renders.

use log::warn;
use quick_xml::{Reader, events::Event};

use super::Entry;

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
                            .map(std::borrow::Cow::into_owned)
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
