//! Lookup helpers returning typed proxies for iwd objects.

use masterror::{AppError, AppResult};
use zbus::fdo::ObjectManagerProxy;

/// Macro to simplify listing proxies based on their interface name.
macro_rules! list_proxies {
    ($manager:expr, $interface:expr, $proxy_type:ty) => {
        async {
            let objects = $manager
                .get_managed_objects()
                .await
                .map_err(|e| fdo_failure("Failed to get managed objects", &e))?;
            let mut proxies = Vec::new();
            for (path, ifs) in objects {
                if ifs.contains_key($interface) {
                    proxies.push(
                        <$proxy_type>::builder($manager.inner().connection())
                            .destination("net.connman.iwd")
                            .map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to set proxy destination: {}",
                                    e
                                ))
                            })?
                            .path(path.clone())
                            .map_err(|e| bus_failure("Failed to set proxy path", &e))?
                            .build()
                            .await
                            .map_err(|e| bus_failure("Failed to build proxy", &e))?
                    );
                }
            }
            Ok::<_, AppError>(proxies)
        }
    };
}

use super::{
    IwdDbus, access_point::AccessPointProxy, adapter::AdapterProxy,
    agent_manager::AgentManagerProxy, device::DeviceProxy, known_network::KnownNetworkProxy,
    network::NetworkProxy, station::StationProxy
};
use crate::services::bus::{bus_failure, fdo_failure};

impl IwdDbus<'_> {
    /// Connect to the system bus and the IWD service
    ///
    /// # Errors
    ///
    /// Returns an error when the `ObjectManager` proxy destination or path
    /// cannot be set, or when the proxy cannot be built on the connection.
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let manager = ObjectManagerProxy::builder(conn)
            .destination("net.connman.iwd")
            .map_err(|e| bus_failure("Failed to set ObjectManagerProxy destination", &e))?
            .path("/")
            .map_err(|e| bus_failure("Failed to set ObjectManagerProxy path", &e))?
            .build()
            .await
            .map_err(|e| bus_failure("Failed to build ObjectManagerProxy for IWD", &e))?;

        Ok(Self {
            inner: manager
        })
    }

    /// Lists a proxy for every iwd station object.
    ///
    /// iwd nests them: an adapter carries devices, and a device in station
    /// mode carries the station. The station is what answers for the link.
    ///
    /// # Errors
    ///
    /// Returns an error when the managed objects cannot be listed or a station
    /// proxy cannot be built.
    pub async fn stations(&self) -> AppResult<Vec<StationProxy>> {
        list_proxies!(&self.inner, "net.connman.iwd.Station", StationProxy).await
    }

    /// Lists a proxy for every iwd adapter object.
    ///
    /// # Errors
    ///
    /// Returns an error when the managed objects cannot be listed or an
    /// adapter proxy cannot be built.
    pub async fn adapters(&self) -> AppResult<Vec<AdapterProxy>> {
        list_proxies!(&self.inner, "net.connman.iwd.Adapter", AdapterProxy).await
    }

    /// Lists a proxy for every iwd device object.
    ///
    /// # Errors
    ///
    /// Returns an error when the managed objects cannot be listed or a device
    /// proxy cannot be built.
    pub async fn devices(&self) -> AppResult<Vec<DeviceProxy>> {
        list_proxies!(&self.inner, "net.connman.iwd.Device", DeviceProxy).await
    }

    /// Returns the proxy for the iwd agent manager object.
    ///
    /// # Errors
    ///
    /// Returns an error when the managed objects cannot be listed, a proxy
    /// cannot be built, or no agent manager object is present on the bus.
    pub async fn agent_manager(&self) -> AppResult<AgentManagerProxy> {
        list_proxies!(
            &self.inner,
            "net.connman.iwd.AgentManager",
            AgentManagerProxy
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| AppError::not_found("No AgentManagerProxy found"))
    }

    /// Lists a proxy for every network iwd already knows.
    ///
    /// # Errors
    ///
    /// Returns an error when the managed objects cannot be listed or a known
    /// network proxy cannot be built.
    pub async fn known_networks_proxies(&self) -> AppResult<Vec<KnownNetworkProxy>> {
        list_proxies!(
            &self.inner,
            "net.connman.iwd.KnownNetwork",
            KnownNetworkProxy
        )
        .await
    }

    /// Lists a proxy for every network object currently exposed.
    ///
    /// # Errors
    ///
    /// Returns an error when the managed objects cannot be listed or a network
    /// proxy cannot be built.
    pub async fn networks_proxies(&self) -> AppResult<Vec<NetworkProxy>> {
        list_proxies!(&self.inner, "net.connman.iwd.Network", NetworkProxy).await
    }

    /// Lists a proxy for every access point object currently exposed.
    ///
    /// Asked of the root object manager, where iwd exposes an access point
    /// only while the card is running one. A card in station mode answers
    /// with nothing, which is the ordinary case and not a failure.
    ///
    /// # Errors
    ///
    /// Returns an error when the managed objects cannot be listed or an access
    /// point proxy cannot be built.
    pub async fn access_points_proxies(&self) -> AppResult<Vec<AccessPointProxy>> {
        list_proxies!(&self.inner, "net.connman.iwd.AccessPoint", AccessPointProxy).await
    }

    /// Lists every network visible from a station together with its signal
    /// strength.
    ///
    /// # Errors
    ///
    /// Returns an error when the stations cannot be listed, a station refuses
    /// the ordered networks query, or a network proxy cannot be built.
    pub async fn reachable_networks(&self) -> AppResult<Vec<(NetworkProxy, i16)>> {
        let stations = self.stations().await?;
        let mut networks = Vec::new();

        for station in stations {
            let networks_proxies = station
                .get_ordered_networks()
                .await
                .map_err(|e| bus_failure("Failed to get ordered networks from station", &e))?;
            for (path, strength) in networks_proxies {
                let network = NetworkProxy::builder(self.inner().connection())
                    .destination("net.connman.iwd")
                    .map_err(|e| bus_failure("Failed to set NetworkProxy destination", &e))?
                    .path(path.clone())
                    .map_err(|e| bus_failure("Failed to set NetworkProxy path", &e))?
                    .build()
                    .await
                    .map_err(|e| bus_failure("Failed to build NetworkProxy", &e))?;
                networks.push((network, strength));
            }
        }
        Ok(networks)
    }
}
