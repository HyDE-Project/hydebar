//! Unit tests for the iwd D-Bus agents.

use std::convert::TryFrom;

use zbus::zvariant::OwnedObjectPath;

use super::agents::{PWAgent, SignalAgent};

#[tokio::test]
async fn pw_agent_returns_password_when_available() {
    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let mut agent = PWAgent {
        password_rx: rx
    };
    let path = OwnedObjectPath::try_from("/").expect("valid object path");

    assert!(agent.request_passphrase(path.clone()).await.is_err());

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send("secret".to_string()).expect("send password");
    let mut agent = PWAgent {
        password_rx: rx
    };
    let path = OwnedObjectPath::try_from("/").expect("valid object path");
    let value = agent
        .request_passphrase(path)
        .await
        .expect("password available");
    assert_eq!(value, "secret");
}

#[test]
fn signal_agent_emits_levels() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let agent = SignalAgent {
        tx
    };
    agent.changed(42);
    assert_eq!(rx.try_recv().expect("signal level"), 42);
}
