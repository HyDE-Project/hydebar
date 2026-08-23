//! Which network a station is joined to.

use super::super::{IwdDbus, network::NetworkProxy, station::StationProxy};
use crate::services::bus::bus_failure;

/// Answers with nothing, naming the failure first unless it is an absence.
///
/// A station between networks has no joined network to report, and iwd says
/// so by not carrying the property at all — that is the ordinary case and
/// silence is the honest answer to it. A bus that refuses, times out or has
/// gone is a different thing, and reads as an unjoined station unless it is
/// named where it happened.
fn report(err: &zbus::Error, context: &str) -> Option<String> {
    let failure = bus_failure(context, err);

    if failure.kind != masterror::AppErrorKind::NotFound {
        log::warn!("{failure}");
    }

    None
}

impl IwdDbus<'_> {
    /// The name of the network a station is joined to, where it is joined to
    /// one.
    ///
    /// The signal agent reports how strong a link is and says nothing about
    /// which link it is — but an agent is registered against one station, so
    /// the station is the answer. Asked at the moment the reading arrives
    /// rather than remembered from registration, because a station roams and
    /// the reading has to be filed under the network it was taken on.
    ///
    /// A station between networks answers with nothing rather than an empty
    /// name: the bar files strength readings under the network they belong
    /// to, and an empty name belongs to none of them.
    pub async fn connected_network_name(&self, station: &StationProxy<'_>) -> Option<String> {
        let path = match station.connected_network().await {
            Ok(path) => path,
            Err(err) => return report(&err, "asking the station which network it is joined to")
        };

        let network = match NetworkProxy::builder(self.inner().connection())
            .destination("net.connman.iwd")
            .and_then(|builder| builder.path(path))
        {
            Ok(builder) => match builder.build().await {
                Ok(network) => network,
                Err(err) => return report(&err, "addressing the network the station is joined to")
            },
            Err(err) => return report(&err, "addressing the network the station is joined to")
        };

        match network.name().await {
            Ok(name) => Some(name).filter(|name| !name.is_empty()),
            Err(err) => report(&err, "asking the joined network for its name")
        }
    }
}
