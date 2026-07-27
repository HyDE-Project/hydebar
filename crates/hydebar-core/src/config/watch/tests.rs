//! Unit tests for the configuration watcher.

use std::{
    ffi::{OsStr, OsString},
    sync::Arc
};

use hydebar_proto::config::Config;
use iced::futures::{StreamExt, channel::mpsc};
use inotify::EventMask;
use tempfile::TempDir;

use super::{
    Event, WatchedEvent,
    events::ConfigEvent,
    interpret::{handle_watch_event, interpret_event}
};
use crate::config::{ConfigUpdateError, manager::ConfigManager};

#[derive(Debug)]
struct FakeEvent {
    name: Option<OsString>,
    mask: EventMask
}

impl WatchedEvent for FakeEvent {
    fn file_name(&self) -> Option<&OsStr> {
        self.name.as_deref()
    }

    fn mask(&self) -> EventMask {
        self.mask
    }
}

#[test]
fn interpret_event_detects_removed_events() {
    let target = OsStr::new("config.toml");

    let delete_event = FakeEvent {
        name: Some(OsString::from("config.toml")),
        mask: EventMask::DELETE
    };
    assert_eq!(interpret_event(&delete_event, target), Some(Event::Removed));

    let moved_from_event = FakeEvent {
        name: Some(OsString::from("config.toml")),
        mask: EventMask::MOVED_FROM
    };
    assert_eq!(
        interpret_event(&moved_from_event, target),
        Some(Event::Removed)
    );

    let unrelated_name = FakeEvent {
        name: Some(OsString::from("other.toml")),
        mask: EventMask::DELETE
    };
    assert_eq!(interpret_event(&unrelated_name, target), None);
}

#[test]
fn interpret_event_detects_changed_events() {
    let target = OsStr::new("config.toml");

    for mask in [
        EventMask::CREATE,
        EventMask::MODIFY,
        EventMask::MOVED_TO,
        EventMask::CLOSE_WRITE
    ] {
        let event = FakeEvent {
            name: Some(OsString::from("config.toml")),
            mask
        };
        assert_eq!(interpret_event(&event, target), Some(Event::Changed));
    }

    let ignored_event = FakeEvent {
        name: Some(OsString::from("config.toml")),
        mask: EventMask::ACCESS
    };
    assert_eq!(interpret_event(&ignored_event, target), None);
}

#[tokio::test]
async fn emits_applied_event_for_valid_update() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, "").expect("failed to write config");

    let manager = Arc::new(ConfigManager::new(Config::default()));
    let (mut sender, mut receiver) = mpsc::channel(10);

    handle_watch_event(
        &mut sender,
        &config_path,
        Event::Changed,
        Arc::clone(&manager)
    )
    .await
    .expect("sending event should succeed");

    match receiver.next().await {
        Some(ConfigEvent::Applied(_)) => {}
        other => panic!("unexpected event: {other:?}")
    }
}

#[tokio::test]
async fn emits_degraded_event_for_invalid_toml() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, "invalid = [").expect("failed to write invalid config");

    let manager = Arc::new(ConfigManager::new(Config::default()));
    let (mut sender, mut receiver) = mpsc::channel(10);

    handle_watch_event(
        &mut sender,
        &config_path,
        Event::Changed,
        Arc::clone(&manager)
    )
    .await
    .expect("sending event should succeed");

    match receiver.next().await {
        Some(ConfigEvent::Degraded(event)) => {
            assert!(matches!(event.reason, ConfigUpdateError::Parse { .. }));
        }
        other => panic!("unexpected event: {other:?}")
    }
}

#[tokio::test]
async fn emits_degraded_event_when_file_removed() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, "").expect("failed to write config");

    let manager = Arc::new(ConfigManager::new(Config::default()));
    let (mut sender, mut receiver) = mpsc::channel(10);

    handle_watch_event(&mut sender, &config_path, Event::Removed, manager)
        .await
        .expect("sending event should succeed");

    match receiver.next().await {
        Some(ConfigEvent::Degraded(event)) => {
            assert!(matches!(event.reason, ConfigUpdateError::Removed));
        }
        other => panic!("unexpected event: {other:?}")
    }
}
