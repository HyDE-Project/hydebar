//! One registered status notifier item and its proxies.

use log::debug;
use masterror::{AppError, AppResult};

use super::{
    TrayIcon,
    dbus::{self, DBusMenuProxy, Layout, StatusNotifierItemProxy},
    icon
};

#[derive(Debug, Clone)]
pub struct StatusNotifierItem {
    pub name:              String,
    pub icon:              Option<TrayIcon>,
    pub menu:              Layout,
    pub(super) item_proxy: StatusNotifierItemProxy<'static>,
    pub(super) menu_proxy: DBusMenuProxy<'static>
}

impl StatusNotifierItem {
    /// Builds the item and menu proxies for a registered tray item.
    ///
    /// # Errors
    ///
    /// Returns an error when the item or menu proxy cannot be configured or
    /// built, or when the item refuses to report its menu path.
    pub async fn new(conn: &zbus::Connection, name: String) -> AppResult<Self> {
        let (dest, path) = name.find('/').map_or_else(
            || (name.as_ref(), "/StatusNotifierItem"),
            |idx| (&name[..idx], &name[idx..])
        );

        let item_proxy = StatusNotifierItemProxy::builder(conn)
            .destination(dest.to_owned())
            .map_err(|e| {
                AppError::internal(format!(
                    "Failed to set StatusNotifierItemProxy destination: {e}"
                ))
            })?
            .path(path.to_owned())
            .map_err(|e| {
                AppError::internal(format!("Failed to set StatusNotifierItemProxy path: {e}"))
            })?
            .build()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to build StatusNotifierItemProxy: {e}"))
            })?;

        debug!("item_proxy {item_proxy:?}");

        let icon_pixmap = item_proxy.icon_pixmap().await;

        let icon = match icon_pixmap {
            Ok(icons) => {
                debug!("icon_pixmap {icons:?}");
                tokio::task::spawn_blocking(move || icon::icon_from_pixmaps(icons))
                    .await
                    .ok()
                    .flatten()
            }
            Err(_) => match item_proxy.icon_name().await.ok() {
                Some(icon_name) => {
                    tokio::task::spawn_blocking(move || icon::icon_from_name(&icon_name))
                        .await
                        .ok()
                        .flatten()
                }
                None => None
            }
        };

        let menu_path = item_proxy
            .menu()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get menu path: {e}")))?;
        let menu_proxy = dbus::DBusMenuProxy::builder(conn)
            .destination(dest.to_owned())
            .map_err(|e| {
                AppError::internal(format!("Failed to set DBusMenuProxy destination: {e}"))
            })?
            .path(menu_path.clone())
            .map_err(|e| AppError::internal(format!("Failed to set DBusMenuProxy path: {e}")))?
            .build()
            .await
            .map_err(|e| AppError::internal(format!("Failed to build DBusMenuProxy: {e}")))?;

        let (_, menu) = menu_proxy
            .get_layout(0, -1, &[])
            .await
            .map_err(|e| AppError::internal(format!("Failed to get menu layout: {e}")))?;

        Ok(Self {
            name,
            icon,
            menu,
            item_proxy,
            menu_proxy
        })
    }
}
