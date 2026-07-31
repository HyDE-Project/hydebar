//! Unit tests for the HyDE theme watcher.

use std::{
    ffi::{OsStr, OsString},
    fs,
    path::Path,
    sync::Arc
};

use hex_color::HexColor;
use hydebar_proto::{
    config::{AppearanceColor, Config},
    hyde_dirs::HydeDirs
};
use iced::futures::{StreamExt, channel::mpsc};
use inotify::EventMask;
use tempfile::TempDir;

use super::{
    interpret::{handle_theme_event, interpret_theme_event},
    sources::{ThemeRoots, watch_targets, watched_names}
};
use crate::config::{
    ConfigEvent,
    manager::ConfigManager,
    watch::{Event, WatchedEvent}
};

/// Stand-in for an inotify event, so the rules can be exercised without the
/// kernel.
#[derive(Debug)]
struct FakeEvent {
    name: Option<OsString>,
    mask: EventMask
}

impl FakeEvent {
    fn new(name: &str, mask: EventMask) -> Self {
        Self {
            name: Some(OsString::from(name)),
            mask
        }
    }
}

impl WatchedEvent for FakeEvent {
    fn file_name(&self) -> Option<&OsStr> {
        self.name.as_deref()
    }

    fn mask(&self) -> EventMask {
        self.mask
    }
}

/// A HyDE layout rooted in one temporary directory.
///
/// Built explicitly rather than from the environment so a test never reads the
/// theme of the machine it runs on.
fn roots_in(root: &Path) -> ThemeRoots {
    ThemeRoots::new(HydeDirs::new(
        root.join("config"),
        root.join("state"),
        root.join("cache"),
        root.join("data")
    ))
}

const DARK_THEME_CSS: &str = "\
@define-color bar-bg rgba(27,29,28,0.01);
@define-color main-bg rgba(27,29,28,0.8);
@define-color main-fg #AAF0CD;
@define-color wb-act-bg rgb(195,172,118);
@define-color wb-act-fg rgb(255,240,204);
";

const LIGHT_THEME_CSS: &str = "\
@define-color bar-bg rgba(240,240,240,0.5);
@define-color main-bg rgba(250,250,250,0.9);
@define-color main-fg #101010;
@define-color wb-act-bg rgb(10,90,200);
@define-color wb-act-fg rgb(255,255,255);
";

/// Writes a stylesheet as HyDE does, by replacing the file rather than editing
/// it in place.
fn write_theme_css(config_dir: &Path, source: &str) {
    let waybar = config_dir.join("waybar");
    fs::create_dir_all(&waybar).expect("create waybar directory");

    write_replacing(&waybar.join("theme.css"), source);
}

/// Writes a file the way HyDE does, by moving a staged copy over the old one.
///
/// Worth mimicking because that is precisely why the watcher follows
/// directories rather than files.
fn write_replacing(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create directory");

    let staging = path.with_extension("staged");
    fs::write(&staging, contents).expect("write staged file");
    fs::rename(&staging, path).expect("move file into place");
}

/// Lays out a HyDE cache the bar can colour itself from.
///
/// `primary` becomes the dominant colour the islands take, so a test can switch
/// palettes by naming a different one.
fn write_hyde_palette(roots: &ThemeRoots, primary: &str) {
    let mut lines = vec![String::from("dcol_mode=\"dark\"")];

    for group in 1..=4u8 {
        let dominant = if group == 1 {
            primary.to_owned()
        } else {
            format!("30303{group}")
        };

        lines.push(format!("dcol_pry{group}=\"{dominant}\""));
        lines.push(format!("dcol_txt{group}=\"FFFFF{group}\""));

        for step in 1..=9u8 {
            lines.push(format!("dcol_{group}xa{step}=\"1{group}2{step}33\""));
        }
    }

    write_replacing(
        &roots.dirs.staterc(),
        "HYDE_THEME=\"Fixture\"\nenableWallDcol=\"1\"\n"
    );
    write_replacing(&roots.dirs.wall_dcol(), &lines.join("\n"));
}

#[test]
fn interpret_theme_event_reports_a_rewritten_stylesheet_as_changed() {
    let names = watched_names();

    for name in [
        "wall.dcol",
        "staterc",
        "config",
        "hyprland.conf",
        "config.toml",
        "env-theme",
        "theme.css",
        "global.css",
        "border-radius.css"
    ] {
        for mask in [
            EventMask::CREATE,
            EventMask::MODIFY,
            EventMask::MOVED_TO,
            EventMask::CLOSE_WRITE
        ] {
            assert_eq!(
                interpret_theme_event(&FakeEvent::new(name, mask), &names),
                Some(Event::Changed),
                "{name} with {mask:?} should be reported as changed"
            );
        }
    }
}

#[test]
fn interpret_theme_event_reports_a_replaced_stylesheet_as_removed() {
    let names = watched_names();

    assert_eq!(
        interpret_theme_event(&FakeEvent::new("wall.dcol", EventMask::DELETE), &names),
        Some(Event::Removed)
    );
    assert_eq!(
        interpret_theme_event(&FakeEvent::new("staterc", EventMask::MOVED_FROM), &names),
        Some(Event::Removed)
    );
}

#[test]
fn a_replaced_palette_symlink_is_seen_through_the_directory_it_sits_in() {
    let names = watched_names();

    for mask in [EventMask::CREATE, EventMask::MOVED_TO] {
        assert_eq!(
            interpret_theme_event(&FakeEvent::new("wall.dcol", mask), &names),
            Some(Event::Changed),
            "a palette re-linked with {mask:?} should be reported as changed"
        );
    }
}

#[test]
fn interpret_theme_event_ignores_files_the_theme_is_not_read_from() {
    let names = watched_names();

    assert_eq!(
        interpret_theme_event(&FakeEvent::new("style.css", EventMask::MODIFY), &names),
        None
    );
    assert_eq!(
        interpret_theme_event(&FakeEvent::new("theme.css", EventMask::ACCESS), &names),
        None
    );
}

#[test]
fn watch_targets_are_empty_when_the_bar_does_not_follow_hyde() {
    assert!(watch_targets(false).is_empty());
}

#[test]
fn watch_targets_cover_every_file_the_theme_is_read_from() {
    let targets = roots_in(Path::new("/tmp/hydebar-fixture")).targets();

    let directories: Vec<_> = targets
        .iter()
        .map(|target| target.directory.clone())
        .collect();
    assert_eq!(
        directories,
        vec![
            Path::new("/tmp/hydebar-fixture/cache/hyde").to_path_buf(),
            Path::new("/tmp/hydebar-fixture/state/hyde").to_path_buf(),
            Path::new("/tmp/hydebar-fixture/config/hyde").to_path_buf(),
            Path::new("/tmp/hydebar-fixture/data/hyde").to_path_buf(),
            Path::new("/tmp/hydebar-fixture/config/waybar").to_path_buf(),
            Path::new("/tmp/hydebar-fixture/config/waybar/includes").to_path_buf(),
        ]
    );

    let names: Vec<_> = targets
        .iter()
        .flat_map(|target| target.file_names.clone())
        .collect();
    assert_eq!(
        names,
        vec![
            OsString::from("wall.dcol"),
            OsString::from("staterc"),
            OsString::from("config"),
            OsString::from("hyprland.conf"),
            OsString::from("config.toml"),
            OsString::from("env-theme"),
            OsString::from("theme.css"),
            OsString::from("global.css"),
            OsString::from("border-radius.css"),
        ]
    );
}

#[test]
fn the_hyde_directories_are_watched_before_the_fallback_stylesheets() {
    let targets = roots_in(Path::new("/tmp/hydebar-fixture")).targets();

    assert!(
        targets[0].directory.ends_with("cache/hyde"),
        "the palette directory has to be watched first"
    );
}

#[tokio::test]
async fn a_palette_switch_repaints_the_bar_from_the_hyde_cache() {
    let root = TempDir::new().expect("failed to create temp dir");
    let roots = roots_in(root.path());
    let config_path = root.path().join("config.toml");
    fs::write(&config_path, "").expect("failed to write config");

    write_hyde_palette(&roots, "483828");

    let manager = Arc::new(ConfigManager::new(Config::default()));
    let (mut sender, mut receiver) = mpsc::channel(10);

    handle_theme_event(
        &mut sender,
        &config_path,
        &roots,
        Event::Changed,
        Arc::clone(&manager)
    )
    .await
    .expect("sending the first event should succeed");

    let first = match receiver.next().await {
        Some(ConfigEvent::Applied(applied)) => applied,
        other => panic!("unexpected event: {other:?}")
    };
    assert_eq!(
        first.config.appearance.background_color.get_base(),
        iced::Color::from_rgb8(0x48, 0x38, 0x28)
    );
    assert_eq!(first.config.appearance.opacity, 0.8);

    write_hyde_palette(&roots, "1B1D1C");

    handle_theme_event(
        &mut sender,
        &config_path,
        &roots,
        Event::Changed,
        Arc::clone(&manager)
    )
    .await
    .expect("sending the second event should succeed");

    let second = match receiver.next().await {
        Some(ConfigEvent::Applied(applied)) => applied,
        other => panic!("unexpected event: {other:?}")
    };

    assert!(
        second.impact.appearance_changed,
        "a palette switch should be reported as an appearance change"
    );
    assert_eq!(
        second.config.appearance.background_color.get_base(),
        iced::Color::from_rgb8(0x1B, 0x1D, 0x1C)
    );
}

#[tokio::test]
async fn a_colour_written_in_the_bar_configuration_survives_a_palette_switch() {
    let root = TempDir::new().expect("failed to create temp dir");
    let roots = roots_in(root.path());
    let config_path = root.path().join("config.toml");
    fs::write(
        &config_path,
        "[appearance]\nbackground_color = \"#010203\"\nfont_size = 17.0\n"
    )
    .expect("failed to write config");

    write_hyde_palette(&roots, "483828");

    let manager = Arc::new(ConfigManager::new(Config::default()));
    let (mut sender, mut receiver) = mpsc::channel(10);

    handle_theme_event(&mut sender, &config_path, &roots, Event::Changed, manager)
        .await
        .expect("sending the event should succeed");

    match receiver.next().await {
        Some(ConfigEvent::Applied(applied)) => {
            assert_eq!(
                applied.config.appearance.background_color,
                AppearanceColor::Simple(HexColor::rgb(1, 2, 3))
            );
            assert_eq!(applied.config.appearance.font_size, Some(17.0));
        }
        other => panic!("unexpected event: {other:?}")
    }
}

#[tokio::test]
async fn a_stylesheet_switch_still_applies_where_hyde_answers_for_nothing() {
    let root = TempDir::new().expect("failed to create temp dir");
    let roots = roots_in(root.path());
    let config_dir = roots.dirs.config.clone();
    let config_path = root.path().join("config.toml");
    fs::write(&config_path, "").expect("failed to write config");

    write_theme_css(&config_dir, DARK_THEME_CSS);

    let manager = Arc::new(ConfigManager::new(Config::default()));
    let (mut sender, mut receiver) = mpsc::channel(10);

    handle_theme_event(
        &mut sender,
        &config_path,
        &roots,
        Event::Changed,
        Arc::clone(&manager)
    )
    .await
    .expect("sending the first event should succeed");

    let dark = match receiver.next().await {
        Some(ConfigEvent::Applied(applied)) => applied,
        other => panic!("unexpected event: {other:?}")
    };
    assert_eq!(dark.config.appearance.opacity, 0.8);
    assert_eq!(
        dark.config.appearance.text_color,
        AppearanceColor::Simple(HexColor::rgb(170, 240, 205))
    );

    write_theme_css(&config_dir, LIGHT_THEME_CSS);

    handle_theme_event(
        &mut sender,
        &config_path,
        &roots,
        Event::Changed,
        Arc::clone(&manager)
    )
    .await
    .expect("sending the second event should succeed");

    let light = match receiver.next().await {
        Some(ConfigEvent::Applied(applied)) => applied,
        other => panic!("unexpected event: {other:?}")
    };

    assert!(
        light.impact.appearance_changed,
        "a theme switch should be reported as an appearance change"
    );
    assert_ne!(
        light.config.appearance.background_color,
        dark.config.appearance.background_color
    );
    assert_ne!(
        light.config.appearance.text_color,
        dark.config.appearance.text_color
    );
    assert_eq!(light.config.appearance.opacity, 0.9);
}

#[tokio::test]
async fn a_theme_switch_leaves_the_appearance_alone_when_follow_hyde_is_disabled() {
    let dirs = TempDir::new().expect("failed to create temp dir");
    let config_dir = dirs.path().join("config");
    let state_dir = dirs.path().join("state");
    let config_path = dirs.path().join("config.toml");
    fs::write(&config_path, "[appearance]\nfollow_hyde = false\n")
        .expect("failed to write config");

    write_theme_css(&config_dir, LIGHT_THEME_CSS);

    let roots = ThemeRoots::new(HydeDirs::new(
        config_dir,
        state_dir,
        dirs.path().join("cache"),
        dirs.path().join("data")
    ));
    let manager = Arc::new(ConfigManager::new(Config::default()));
    let (mut sender, mut receiver) = mpsc::channel(10);

    handle_theme_event(&mut sender, &config_path, &roots, Event::Changed, manager)
        .await
        .expect("sending the event should succeed");

    match receiver.next().await {
        Some(ConfigEvent::Applied(applied)) => {
            let default = Config::default();
            assert_eq!(
                applied.config.appearance.background_color,
                default.appearance.background_color
            );
            assert_eq!(
                applied.config.appearance.opacity,
                default.appearance.opacity
            );
        }
        other => panic!("unexpected event: {other:?}")
    }
}
