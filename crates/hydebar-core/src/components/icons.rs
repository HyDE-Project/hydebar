//! Built in glyph catalogue and the user configurable overrides applied to it.

use std::collections::HashMap;

use hydebar_proto::config::IconsConfig;
use iced::{
    Font,
    widget::{Text, text}
};
use log::warn;

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub enum Icons {
    #[default]
    None,
    AppLauncher,
    Clipboard,
    Refresh,
    NoUpdatesAvailable,
    UpdatesAvailable,
    MenuClosed,
    MenuOpen,
    Cpu,
    Mem,
    Temp,
    Speaker0,
    Speaker1,
    Speaker2,
    Speaker3,
    Headphones0,
    Headphones1,
    Headset,
    Mic0,
    Mic1,
    MonitorSpeaker,
    ScreenShare,
    Battery0,
    Battery1,
    Battery2,
    Battery3,
    Battery4,
    BatteryCharging,
    Wifi0,
    Wifi1,
    Wifi2,
    Wifi3,
    Wifi4,
    Wifi5,
    WifiLock1,
    WifiLock2,
    WifiLock3,
    WifiLock4,
    WifiLock5,
    Ethernet,
    Vpn,
    Bluetooth,
    PowerSaver,
    Balanced,
    Performance,
    EyeOpened,
    EyeClosed,
    Lock,
    Power,
    Reboot,
    Suspend,
    Logout,
    LeftArrow,
    RightArrow,
    LeftChevron,
    RightChevron,
    Brightness,
    Point,
    Close,
    Airplane,
    Webcam,
    SkipPrevious,
    Play,
    Pause,
    SkipNext,
    MusicNote,
    Drive,
    IpAddress,
    DownloadSpeed,
    UploadSpeed,
    Copy
}

impl Icons {
    /// Every icon that can be overridden from the configuration.
    pub const ALL: [Icons; 71] = [
        Icons::None,
        Icons::AppLauncher,
        Icons::Clipboard,
        Icons::Refresh,
        Icons::NoUpdatesAvailable,
        Icons::UpdatesAvailable,
        Icons::MenuClosed,
        Icons::MenuOpen,
        Icons::Cpu,
        Icons::Mem,
        Icons::Temp,
        Icons::Speaker0,
        Icons::Speaker1,
        Icons::Speaker2,
        Icons::Speaker3,
        Icons::Headphones0,
        Icons::Headphones1,
        Icons::Headset,
        Icons::Mic0,
        Icons::Mic1,
        Icons::MonitorSpeaker,
        Icons::ScreenShare,
        Icons::Battery0,
        Icons::Battery1,
        Icons::Battery2,
        Icons::Battery3,
        Icons::Battery4,
        Icons::BatteryCharging,
        Icons::Wifi0,
        Icons::Wifi1,
        Icons::Wifi2,
        Icons::Wifi3,
        Icons::Wifi4,
        Icons::Wifi5,
        Icons::WifiLock1,
        Icons::WifiLock2,
        Icons::WifiLock3,
        Icons::WifiLock4,
        Icons::WifiLock5,
        Icons::Ethernet,
        Icons::Vpn,
        Icons::Bluetooth,
        Icons::PowerSaver,
        Icons::Balanced,
        Icons::Performance,
        Icons::EyeOpened,
        Icons::EyeClosed,
        Icons::Lock,
        Icons::Power,
        Icons::Reboot,
        Icons::Suspend,
        Icons::Logout,
        Icons::LeftArrow,
        Icons::RightArrow,
        Icons::LeftChevron,
        Icons::RightChevron,
        Icons::Brightness,
        Icons::Point,
        Icons::Close,
        Icons::Airplane,
        Icons::Webcam,
        Icons::SkipPrevious,
        Icons::Play,
        Icons::Pause,
        Icons::SkipNext,
        Icons::MusicNote,
        Icons::Drive,
        Icons::IpAddress,
        Icons::DownloadSpeed,
        Icons::UploadSpeed,
        Icons::Copy
    ];

    /// Key used to address the icon inside the `[icons]` configuration table.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Icons::None => "none",
            Icons::AppLauncher => "app_launcher",
            Icons::Clipboard => "clipboard",
            Icons::Refresh => "refresh",
            Icons::NoUpdatesAvailable => "no_updates_available",
            Icons::UpdatesAvailable => "updates_available",
            Icons::MenuClosed => "menu_closed",
            Icons::MenuOpen => "menu_open",
            Icons::Cpu => "cpu",
            Icons::Mem => "mem",
            Icons::Temp => "temp",
            Icons::Speaker0 => "speaker0",
            Icons::Speaker1 => "speaker1",
            Icons::Speaker2 => "speaker2",
            Icons::Speaker3 => "speaker3",
            Icons::Headphones0 => "headphones0",
            Icons::Headphones1 => "headphones1",
            Icons::Headset => "headset",
            Icons::Mic0 => "mic0",
            Icons::Mic1 => "mic1",
            Icons::MonitorSpeaker => "monitor_speaker",
            Icons::ScreenShare => "screen_share",
            Icons::Battery0 => "battery0",
            Icons::Battery1 => "battery1",
            Icons::Battery2 => "battery2",
            Icons::Battery3 => "battery3",
            Icons::Battery4 => "battery4",
            Icons::BatteryCharging => "battery_charging",
            Icons::Wifi0 => "wifi0",
            Icons::Wifi1 => "wifi1",
            Icons::Wifi2 => "wifi2",
            Icons::Wifi3 => "wifi3",
            Icons::Wifi4 => "wifi4",
            Icons::Wifi5 => "wifi5",
            Icons::WifiLock1 => "wifi_lock1",
            Icons::WifiLock2 => "wifi_lock2",
            Icons::WifiLock3 => "wifi_lock3",
            Icons::WifiLock4 => "wifi_lock4",
            Icons::WifiLock5 => "wifi_lock5",
            Icons::Ethernet => "ethernet",
            Icons::Vpn => "vpn",
            Icons::Bluetooth => "bluetooth",
            Icons::PowerSaver => "power_saver",
            Icons::Balanced => "balanced",
            Icons::Performance => "performance",
            Icons::EyeOpened => "eye_opened",
            Icons::EyeClosed => "eye_closed",
            Icons::Lock => "lock",
            Icons::Power => "power",
            Icons::Reboot => "reboot",
            Icons::Suspend => "suspend",
            Icons::Logout => "logout",
            Icons::LeftArrow => "left_arrow",
            Icons::RightArrow => "right_arrow",
            Icons::LeftChevron => "left_chevron",
            Icons::RightChevron => "right_chevron",
            Icons::Brightness => "brightness",
            Icons::Point => "point",
            Icons::Close => "close",
            Icons::Airplane => "airplane",
            Icons::Webcam => "webcam",
            Icons::SkipPrevious => "skip_previous",
            Icons::Play => "play",
            Icons::Pause => "pause",
            Icons::SkipNext => "skip_next",
            Icons::MusicNote => "music_note",
            Icons::Drive => "drive",
            Icons::IpAddress => "ip_address",
            Icons::DownloadSpeed => "download_speed",
            Icons::UploadSpeed => "upload_speed",
            Icons::Copy => "copy"
        }
    }

    /// Resolves a configuration key back to the icon it addresses.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|icon| icon.name() == name)
    }

    /// Glyph compiled into the binary, used when no override applies.
    #[must_use]
    pub const fn default_glyph(self) -> &'static str {
        match self {
            Icons::None => "",
            Icons::AppLauncher => "󱗼",
            Icons::Clipboard => "󰅌",
            Icons::Refresh => "󰑐",
            Icons::NoUpdatesAvailable => "󰗠",
            Icons::UpdatesAvailable => "󰳛",
            Icons::MenuClosed => "",
            Icons::MenuOpen => "",
            Icons::Cpu => "󰔂",
            Icons::Mem => "",
            Icons::Temp => "󰔏",
            Icons::Speaker0 => "󰸈",
            Icons::Speaker1 => "󰕿",
            Icons::Speaker2 => "󰖀",
            Icons::Speaker3 => "󰕾",
            Icons::Headphones0 => "󰟎",
            Icons::Headphones1 => "󰋋",
            Icons::Headset => "󰋎",
            Icons::Mic0 => "󰍭",
            Icons::Mic1 => "󰍬",
            Icons::ScreenShare => "󱒃",
            Icons::MonitorSpeaker => "󰽟",
            Icons::Battery0 => "󰂃",
            Icons::Battery1 => "󰁼",
            Icons::Battery2 => "󰁾",
            Icons::Battery3 => "󰂀",
            Icons::Battery4 => "󰁹",
            Icons::BatteryCharging => "󰂄",
            Icons::Wifi0 => "󰤭",
            Icons::Wifi1 => "󰤯",
            Icons::Wifi2 => "󰤟",
            Icons::Wifi3 => "󰤢",
            Icons::Wifi4 => "󰤥",
            Icons::Wifi5 => "󰤨",
            Icons::WifiLock1 => "󰤬",
            Icons::WifiLock2 => "󰤡",
            Icons::WifiLock3 => "󰤤",
            Icons::WifiLock4 => "󰤧",
            Icons::WifiLock5 => "󰤪",
            Icons::Ethernet => "󰈀",
            Icons::Vpn => "󰖂",
            Icons::Bluetooth => "󰂯",
            Icons::PowerSaver => "󰾆",
            Icons::Balanced => "󰾅",
            Icons::Performance => "󰓅",
            Icons::EyeOpened => "󰈈",
            Icons::EyeClosed => "󰈉",
            Icons::Lock => "󰌾",
            Icons::Power => "󰐥",
            Icons::Reboot => "󰑐",
            Icons::Suspend => "󰤄",
            Icons::Logout => "󰗽",
            Icons::LeftArrow => "󰁍",
            Icons::RightArrow => "󰁔",
            Icons::LeftChevron => "󰅁",
            Icons::RightChevron => "󰅂",
            Icons::Brightness => "󰃠",
            Icons::Point => "",
            Icons::Close => "󰅖",
            Icons::Airplane => "󰀝",
            Icons::Webcam => "",
            Icons::SkipPrevious => "󰒮",
            Icons::Play => "󰐊",
            Icons::Pause => "󰏤",
            Icons::SkipNext => "󰒭",
            Icons::MusicNote => "󰎇",
            Icons::Drive => "󰋊",
            Icons::IpAddress => "󰩠",
            Icons::DownloadSpeed => "󰛴",
            Icons::UploadSpeed => "󰛶",
            Icons::Copy => "󰆏"
        }
    }
}

impl From<Icons> for &'static str {
    fn from(icon: Icons) -> &'static str {
        icon.default_glyph()
    }
}

/// Glyph table handed to the rendering helpers.
///
/// Holds only the icons the user redefined; everything else falls back to
/// [`Icons::default_glyph`], so the default table costs nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IconTheme {
    overrides: HashMap<Icons, String>
}

impl IconTheme {
    /// Builds the table from the `[icons]` configuration section.
    ///
    /// Unknown names are reported and skipped so a typo cannot take the bar
    /// down.
    #[must_use]
    pub fn from_config(config: &IconsConfig) -> Self {
        let mut overrides = HashMap::new();

        for (name, glyph) in config.iter() {
            match Icons::from_name(name) {
                Some(icon) => {
                    overrides.insert(icon, glyph.to_owned());
                }
                None => warn!("Unknown icon override `{name}`, ignoring it")
            }
        }

        Self {
            overrides
        }
    }

    /// Glyph to render for `icon`, honouring any configured override.
    #[must_use]
    pub fn glyph(&self, icon: Icons) -> &str {
        self.overrides
            .get(&icon)
            .map_or_else(|| icon.default_glyph(), String::as_str)
    }

    /// Declares an override for `icon`, replacing any previous value.
    pub fn set(&mut self, icon: Icons, glyph: impl Into<String>) {
        self.overrides.insert(icon, glyph.into());
    }

    /// Returns `true` when every icon resolves to its built in glyph.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.overrides.is_empty()
    }
}

impl From<&IconsConfig> for IconTheme {
    fn from(config: &IconsConfig) -> Self {
        Self::from_config(config)
    }
}

pub fn icon<'a>(theme: &IconTheme, r#type: Icons) -> Text<'a> {
    icon_raw(theme.glyph(r#type).to_owned())
}

pub fn icon_raw<'a>(s: String) -> Text<'a> {
    text(s).font(Font::with_name("Symbols Nerd Font"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_keeps_built_in_glyphs() {
        let theme = IconTheme::default();

        assert!(theme.is_default());
        for icon in Icons::ALL {
            assert_eq!(theme.glyph(icon), icon.default_glyph());
        }
    }

    #[test]
    fn absent_override_keeps_built_in_glyph() {
        let config = IconsConfig::from_iter([("cpu", "\u{f035b}")]);
        let theme = IconTheme::from_config(&config);

        assert_eq!(theme.glyph(Icons::Mem), Icons::Mem.default_glyph());
        assert_eq!(theme.glyph(Icons::Temp), Icons::Temp.default_glyph());
    }

    #[test]
    fn override_replaces_glyph() {
        let config = IconsConfig::from_iter([
            ("cpu", "\u{f035b}"),
            ("mem", "\u{f0f86}"),
            ("battery4", "\u{f0079}")
        ]);
        let theme = IconTheme::from_config(&config);

        assert_eq!(theme.glyph(Icons::Cpu), "\u{f035b}");
        assert_eq!(theme.glyph(Icons::Mem), "\u{f0f86}");
        assert_eq!(theme.glyph(Icons::Battery4), "\u{f0079}");
        assert!(!theme.is_default());
    }

    #[test]
    fn unknown_override_is_ignored() {
        let config = IconsConfig::from_iter([("not_an_icon", "X"), ("cpu", "Y")]);
        let theme = IconTheme::from_config(&config);

        assert_eq!(theme.glyph(Icons::Cpu), "Y");
        assert_eq!(Icons::from_name("not_an_icon"), None);
    }

    #[test]
    fn every_icon_has_a_unique_name() {
        let mut names = std::collections::HashSet::new();

        for icon in Icons::ALL {
            assert!(names.insert(icon.name()), "duplicate name {}", icon.name());
            assert_eq!(Icons::from_name(icon.name()), Some(icon));
        }

        assert_eq!(names.len(), Icons::ALL.len());
    }

    #[test]
    fn set_overrides_a_single_icon() {
        let mut theme = IconTheme::default();
        theme.set(Icons::Bluetooth, "Z");

        assert_eq!(theme.glyph(Icons::Bluetooth), "Z");
        assert_eq!(theme.glyph(Icons::Vpn), Icons::Vpn.default_glyph());
    }
}
