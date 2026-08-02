//! Proxy for a status notifier item's icon and menu properties.

use zbus::{
    proxy,
    zvariant::{self, OwnedObjectPath}
};

#[derive(Clone, Debug, zvariant::Value)]
pub struct Icon {
    pub width:  i32,
    pub height: i32,
    pub bytes:  Vec<u8>
}

#[proxy(interface = "org.kde.StatusNotifierItem")]
pub trait StatusNotifierItem {
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    #[zbus(property)]
    fn menu(&self) -> zbus::Result<OwnedObjectPath>;
}
