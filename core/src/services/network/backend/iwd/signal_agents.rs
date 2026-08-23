//! The signal agents a listening session owns, and giving them back.
//!
//! iwd reports how strong a link is by calling back into an agent the bar
//! exports and registers against a station. The registration outlives the
//! call that made it: it stands on the connection until somebody takes it
//! down, and the listening session is re-established whenever the event
//! stream ends. Left to itself the session would register another agent per
//! station every time, and iwd would call every one of them — so a reading
//! that should arrive once would arrive as many times as the bar had ever
//! reconnected.
//!
//! [`SignalAgents`] is what the session holds instead: it carries the
//! registrations it made, and giving it back gives them back.

use std::{
    pin::Pin,
    task::{Context, Poll}
};

use iced::futures::Stream;
use log::debug;
use zbus::{Connection, zvariant::OwnedObjectPath};

use super::{agents::SignalAgent, station::StationProxy};

/// The signal agents one listening session registered.
pub(super) struct SignalAgents {
    /// The connection they are exported on.
    conn:   Connection,
    /// The station an agent was registered against, and the agent's own path.
    agents: Vec<(OwnedObjectPath, OwnedObjectPath)>
}

impl SignalAgents {
    /// An empty set, to be filled as each station's agent is registered.
    pub(super) const fn new(conn: Connection) -> Self {
        Self {
            conn,
            agents: Vec::new()
        }
    }

    /// Records an agent registered against the station at `station`.
    ///
    /// Kept as a path rather than as the proxy it was registered through:
    /// that proxy borrows the daemon handle this session was built from, and
    /// the registration has to outlive the call that made it. The proxy is
    /// built again from the connection when the agent is given back.
    pub(super) fn keep(&mut self, station: OwnedObjectPath, agent: OwnedObjectPath) {
        self.agents.push((station, agent));
    }

    /// Guards `stream`, so the registrations last exactly as long as it.
    pub(super) const fn guarding<S>(self, stream: S) -> Guarded<S> {
        Guarded {
            inner:   stream,
            _agents: self
        }
    }
}

impl Drop for SignalAgents {
    /// Unregisters every agent and takes it off the connection.
    ///
    /// Dropping cannot wait, so the work is handed to the runtime the stream
    /// was polled on. A station that has gone in the meantime refuses the
    /// call, which is the outcome wanted anyway.
    fn drop(&mut self) {
        let conn = self.conn.clone();
        let agents = std::mem::take(&mut self.agents);

        if agents.is_empty() {
            return;
        }

        tokio::spawn(async move {
            for (station, agent) in agents {
                if let Ok(builder) = StationProxy::builder(&conn)
                    .destination("net.connman.iwd")
                    .and_then(|builder| builder.path(station))
                    && let Ok(proxy) = builder.build().await
                {
                    let _ = proxy.unregister_signal_level_agent(&agent).await;
                }

                let _ = conn.object_server().remove::<SignalAgent, _>(&agent).await;

                debug!("gave back the signal agent at {agent}");
            }
        });
    }
}

/// A stream that owns the registrations feeding it.
pub(super) struct Guarded<S> {
    /// The events themselves.
    inner:   S,
    /// Held so the agents stand while the stream is read, and no longer.
    _agents: SignalAgents
}

impl<S> Stream for Guarded<S>
where
    S: Stream + Unpin
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::convert::TryFrom;

    use iced::futures::{StreamExt, stream};

    use super::*;

    fn path(at: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(at).expect("a valid object path")
    }

    #[tokio::test]
    async fn a_guarded_stream_still_carries_everything_the_inner_one_does() {
        let conn = Connection::session().await;
        let Ok(conn) = conn else {
            return;
        };

        let guarded = SignalAgents::new(conn).guarding(stream::iter([1, 2, 3]).boxed());

        assert_eq!(guarded.collect::<Vec<i32>>().await, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn a_session_that_registered_nothing_has_nothing_to_give_back() {
        let Ok(conn) = Connection::session().await else {
            return;
        };

        drop(SignalAgents::new(conn));
    }

    #[tokio::test]
    async fn every_registration_a_session_made_is_kept_to_be_given_back() {
        let Ok(conn) = Connection::session().await else {
            return;
        };

        let mut agents = SignalAgents::new(conn);
        agents.keep(
            path("/net/connman/iwd/0/4"),
            path("/com/hydebar/signalagent/one")
        );
        agents.keep(
            path("/net/connman/iwd/0/5"),
            path("/com/hydebar/signalagent/two")
        );

        assert_eq!(
            agents.agents.len(),
            2,
            "a session hands back every agent it stood up, not just the last"
        );
    }
}
