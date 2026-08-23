//! Built in glyph catalogue: every icon the bar knows and its default glyph.
//!
//! The catalogue is split along its three tables: [`glyphs`] holds the
//! default glyph of every icon, [`names`] the configuration keys they
//! answer to, and [`roster`] the full list the overrides walk. The enum
//! itself and its wiring stay here.

mod glyphs;
mod names;
mod roster;

/// Every icon the bar knows how to draw.
///
/// One variant per glyph, each named after what it draws; the default glyph
/// of each lives in [`glyphs`], the configuration key it answers to in
/// [`names`].
#[expect(
    missing_docs,
    reason = "a variant here is one icon, named after the thing it draws, and \
              a line of prose restating the name would say less than the name"
)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub enum Icons {
    #[default]
    None,
    AppLauncher,
    Clipboard,
    Refresh,
    Trash,
    Download,
    ViewGrid,
    ViewList,
    NoUpdatesAvailable,
    UpdatesAvailable,
    MenuClosed,
    MenuOpen,
    Record,
    Stop,
    WindowCapture,
    Cpu,
    Gpu,
    IntegratedGpu,
    Accelerator,
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
    AreaSelect,
    Bell,
    Camera,
    Close,
    Fullscreen,
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
    Copy,
    IdleInhibitorActive,
    IdleInhibitorInactive,
    Settings,
    Themes,
    Wallpaper,
    BarLayout,
    KeybindHint,
    NightLight,
    GameMode,
    Weather,
    /// Marks work signed by the person at the keyboard.
    Authored
}

impl From<Icons> for &'static str {
    fn from(icon: Icons) -> &'static str {
        icon.default_glyph()
    }
}
