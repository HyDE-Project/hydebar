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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::io::Write;

    use super::*;

    /// Writes `xml` to a scratch file and reads the tree back out of it.
    fn tree_of(xml: &str) -> Option<Vec<Entry>> {
        let mut file = tempfile::NamedTempFile::new().expect("a scratch file");
        file.write_all(xml.as_bytes()).expect("the menu is written");

        read_tree(file.path().to_str().expect("a printable path"))
    }

    fn item(id: &str, label: &str) -> String {
        format!(
            r#"<child><object class="GtkMenuItem" id="{id}">
                 <property name="label">{label}</property>
               </object></child>"#
        )
    }

    fn menu(body: &str) -> String {
        format!(r#"<interface><object class="GtkMenu" id="root">{body}</object></interface>"#)
    }

    #[test]
    fn a_menu_file_nobody_wrote_yields_no_tree() {
        assert!(read_tree("/nonexistent/menu.ui").is_none());
    }

    #[test]
    fn a_file_that_is_not_xml_yields_no_tree() {
        assert!(tree_of("<interface><object").is_none());
    }

    #[test]
    fn an_empty_menu_yields_an_empty_tree() {
        let tree = tree_of(&menu("")).expect("the menu is read");

        assert!(tree.is_empty());
    }

    #[test]
    fn every_item_carries_its_identifier_and_label() {
        let tree = tree_of(&menu(&format!(
            "{}{}",
            item("lock", "Lock"),
            item("quit", "Quit")
        )))
        .expect("the menu is read");

        assert_eq!(
            tree,
            vec![
                Entry::Item {
                    id:    "lock".to_owned(),
                    label: "Lock".to_owned()
                },
                Entry::Item {
                    id:    "quit".to_owned(),
                    label: "Quit".to_owned()
                },
            ]
        );
    }

    #[test]
    fn a_separator_becomes_a_dividing_line() {
        let tree = tree_of(&menu(&format!(
            r#"{}<child><object class="GtkSeparatorMenuItem"/></child>{}"#,
            item("lock", "Lock"),
            item("quit", "Quit")
        )))
        .expect("the menu is read");

        assert_eq!(tree.len(), 3);
        assert_eq!(tree[1], Entry::Separator);
    }

    #[test]
    fn a_nested_menu_becomes_a_branch_that_unfolds() {
        let tree = tree_of(&menu(&format!(
            r#"<child><object class="GtkMenuItem" id="power">
                 <property name="label">Power</property>
                 <child><object class="GtkMenu" id="power-menu">{}</object></child>
               </object></child>"#,
            item("off", "Shut down")
        )))
        .expect("the menu is read");

        assert_eq!(
            tree,
            vec![Entry::Submenu {
                id:       "power".to_owned(),
                label:    "Power".to_owned(),
                children: vec![Entry::Item {
                    id:    "off".to_owned(),
                    label: "Shut down".to_owned()
                }]
            }]
        );
    }

    #[test]
    fn a_label_is_trimmed_of_the_whitespace_the_file_lays_it_out_with() {
        let tree = tree_of(&menu(
            r#"<child><object class="GtkMenuItem" id="lock">
                 <property name="label">  Lock  </property>
               </object></child>"#
        ))
        .expect("the menu is read");

        assert_eq!(
            tree,
            vec![Entry::Item {
                id:    "lock".to_owned(),
                label: "Lock".to_owned()
            }]
        );
    }

    #[test]
    fn a_property_that_is_not_a_label_is_not_read_as_one() {
        let tree = tree_of(&menu(
            r#"<child><object class="GtkMenuItem" id="lock">
                 <property name="tooltip">ignored</property>
                 <property name="label">Lock</property>
               </object></child>"#
        ))
        .expect("the menu is read");

        assert_eq!(
            tree,
            vec![Entry::Item {
                id:    "lock".to_owned(),
                label: "Lock".to_owned()
            }]
        );
    }

    /// A self-closed item is the one empty object the reader answers to
    /// besides a separator, and it answers by passing over it: an item that
    /// opens and closes in one tag carries neither identifier nor label, so
    /// there is nothing to press.
    #[test]
    fn a_self_closed_item_is_passed_over() {
        let tree = tree_of(&menu(&format!(
            r#"<child><object class="GtkMenuItem"/></child>{}"#,
            item("lock", "Lock")
        )))
        .expect("the menu is read");

        assert_eq!(
            tree,
            vec![Entry::Item {
                id:    "lock".to_owned(),
                label: "Lock".to_owned()
            }]
        );
    }

    #[test]
    fn an_item_the_file_gives_no_label_is_still_pressable_by_its_identifier() {
        let tree = tree_of(&menu(
            r#"<child><object class="GtkMenuItem" id="lock"></object></child>"#
        ))
        .expect("the menu is read");

        assert_eq!(
            tree,
            vec![Entry::Item {
                id:    "lock".to_owned(),
                label: String::new()
            }]
        );
    }

    #[test]
    fn objects_of_a_kind_the_menu_does_not_know_are_passed_over() {
        let tree = tree_of(&menu(&format!(
            r#"<child><object class="GtkBox" id="box"/></child>{}"#,
            item("lock", "Lock")
        )))
        .expect("the menu is read");

        assert_eq!(
            tree,
            vec![Entry::Item {
                id:    "lock".to_owned(),
                label: "Lock".to_owned()
            }]
        );
    }

    #[test]
    fn a_file_holding_no_menu_at_all_yields_no_tree() {
        let tree = tree_of(r#"<interface><object class="GtkBox" id="box"/></interface>"#);

        assert!(tree.is_none());
    }

    #[test]
    fn a_branch_of_a_branch_is_read_to_its_depth() {
        let tree = tree_of(&menu(&format!(
            r#"<child><object class="GtkMenuItem" id="a">
                 <property name="label">A</property>
                 <child><object class="GtkMenu" id="a-menu">
                   <child><object class="GtkMenuItem" id="b">
                     <property name="label">B</property>
                     <child><object class="GtkMenu" id="b-menu">{}</object></child>
                   </object></child>
                 </object></child>
               </object></child>"#,
            item("c", "C")
        )))
        .expect("the menu is read");

        let Some(Entry::Submenu {
            children, ..
        }) = tree.first()
        else {
            panic!("the outer branch is a submenu")
        };
        let Some(Entry::Submenu {
            children, ..
        }) = children.first()
        else {
            panic!("the inner branch is a submenu")
        };

        assert_eq!(
            children,
            &vec![Entry::Item {
                id:    "c".to_owned(),
                label: "C".to_owned()
            }]
        );
    }
}
