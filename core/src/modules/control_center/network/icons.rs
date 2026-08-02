//! Mapping of connection kinds and signal strengths onto bar icons.

use crate::{
    components::icons::Icons, services::network::ActiveConnectionInfo, utils::IndicatorState
};

static WIFI_SIGNAL_ICONS: [Icons; 6] = [
    Icons::Wifi0,
    Icons::Wifi1,
    Icons::Wifi2,
    Icons::Wifi3,
    Icons::Wifi4,
    Icons::Wifi5
];

static WIFI_LOCK_SIGNAL_ICONS: [Icons; 5] = [
    Icons::WifiLock1,
    Icons::WifiLock2,
    Icons::WifiLock3,
    Icons::WifiLock4,
    Icons::WifiLock5
];

impl ActiveConnectionInfo {
    /// Maps a signal strength to its icon bucket, whatever the backend
    /// sends.
    ///
    /// The strength is clamped first: a backend can report a value past one
    /// hundred — a wrapped negative RSSI does exactly that — and an index
    /// computed from it unclamped walked off the end of the icon tables.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the rounded value stays within 0..=4 after clamping, so the cast is exact"
    )]
    fn signal_bucket(signal: u8) -> usize {
        f32::round(f32::from(signal.min(100)) / 100. * 4.) as usize
    }

    #[must_use]
    pub fn get_wifi_icon(signal: u8) -> Icons {
        WIFI_SIGNAL_ICONS[1 + Self::signal_bucket(signal)]
    }

    #[must_use]
    pub fn get_wifi_lock_icon(signal: u8) -> Icons {
        WIFI_LOCK_SIGNAL_ICONS[Self::signal_bucket(signal)]
    }

    #[must_use]
    pub fn get_icon(&self) -> Icons {
        match self {
            Self::WiFi {
                strength, ..
            } => Self::get_wifi_icon(*strength),
            Self::Wired {
                ..
            } => Icons::Ethernet,
            Self::Vpn {
                ..
            } => Icons::Vpn
        }
    }

    #[must_use]
    pub const fn get_indicator_state(&self) -> IndicatorState {
        match self {
            Self::WiFi {
                strength: 0 | 1, ..
            } => IndicatorState::Warning,
            _ => IndicatorState::Normal
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn every_possible_signal_yields_a_wifi_icon_without_panicking() {
        for signal in u8::MIN..=u8::MAX {
            let _ = ActiveConnectionInfo::get_wifi_icon(signal);
        }
    }

    #[test]
    fn every_possible_signal_yields_a_wifi_lock_icon_without_panicking() {
        for signal in u8::MIN..=u8::MAX {
            let _ = ActiveConnectionInfo::get_wifi_lock_icon(signal);
        }
    }

    #[test]
    fn signal_quartiles_pick_ascending_wifi_icons() {
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(0), Icons::Wifi1);
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(25), Icons::Wifi2);
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(50), Icons::Wifi3);
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(75), Icons::Wifi4);
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(100), Icons::Wifi5);
    }

    #[test]
    fn signal_quartiles_pick_ascending_wifi_lock_icons() {
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(0),
            Icons::WifiLock1
        );
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(25),
            Icons::WifiLock2
        );
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(50),
            Icons::WifiLock3
        );
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(75),
            Icons::WifiLock4
        );
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(100),
            Icons::WifiLock5
        );
    }

    #[test]
    fn a_signal_past_one_hundred_stays_in_the_top_bucket() {
        assert_eq!(ActiveConnectionInfo::get_wifi_icon(u8::MAX), Icons::Wifi5);
        assert_eq!(
            ActiveConnectionInfo::get_wifi_lock_icon(u8::MAX),
            Icons::WifiLock5
        );
    }
}
