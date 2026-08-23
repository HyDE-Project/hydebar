//! Proxy for a status notifier item's icon and menu properties.

#![expect(
    missing_docs,
    reason = "the proxy macro writes the proxy beside this one, and a generated item \
              takes no doc comment of ours"
)]

use zbus::{
    proxy,
    zvariant::{self, OwnedObjectPath}
};

/// One picture as the tray protocol carries it.
#[derive(Clone, Debug, zvariant::Value)]
pub struct Icon {
    /// How wide it is, in pixels.
    pub width:  i32,
    /// How tall it is, in pixels.
    pub height: i32,
    /// The pixels themselves.
    pub bytes:  Vec<u8>
}

/// The interface an application registers its tray entry on.
#[proxy(interface = "org.kde.StatusNotifierItem")]
pub trait StatusNotifierItem {
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    #[zbus(property)]
    fn menu(&self) -> zbus::Result<OwnedObjectPath>;
}
