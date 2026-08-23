//! Menu layout model and proxy for the canonical dbusmenu interface.

#![expect(
    missing_docs,
    reason = "the proxy macro writes the proxy beside this one, and a generated item \
              takes no doc comment of ours"
)]

use zbus::{
    proxy,
    zvariant::{self, OwnedValue, Type}
};

/// One menu entry: its identifier, what it says, and what opens under it.
#[derive(Clone, Debug, Type)]
#[zvariant(signature = "(ia{sv}av)")]
pub struct Layout(pub i32, pub LayoutProps, pub Vec<Self>);

impl<'a> serde::Deserialize<'a> for Layout {
    fn deserialize<D: serde::Deserializer<'a>>(
        deserializer: D
    ) -> std::result::Result<Self, D::Error> {
        let (id, props, children) =
            <(i32, LayoutProps, Vec<(zvariant::Signature, Self)>)>::deserialize(deserializer)?;
        Ok(Self(id, props, children.into_iter().map(|x| x.1).collect()))
    }
}

/// What one menu entry says about itself.
#[derive(Clone, Debug, Type, zvariant::DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct LayoutProps {
    /// Whether the entry opens a submenu.
    #[zvariant(rename = "children-display")]
    pub children_display: Option<String>,
    /// What the entry reads as.
    pub label:            Option<String>,
    /// What kind of entry it is, a separator among them.
    #[zvariant(rename = "type")]
    pub type_:            Option<String>,
    /// Whether it carries a mark, and of what kind.
    #[zvariant(rename = "toggle-type")]
    pub toggle_type:      Option<String>,
    /// Whether that mark is currently set.
    #[zvariant(rename = "toggle-state")]
    pub toggle_state:     Option<i32>
}

/// The interface an application publishes its tray menu on.
#[proxy(interface = "com.canonical.dbusmenu")]
pub trait DBusMenu {
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: &[&str]
    ) -> zbus::Result<(u32, Layout)>;

    fn event(
        &self,
        id: i32,
        event_id: &str,
        data: &OwnedValue,
        timestamp: u32
    ) -> zbus::Result<()>;

    fn about_to_show(&self, id: i32) -> zbus::Result<bool>;

    #[zbus(signal)]
    fn layout_updated(&self, revision: u32, parent: i32) -> zbus::Result<()>;
}
