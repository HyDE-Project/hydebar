use crate::services::network::{ConnectivityState, DeviceState};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Ethernet,
    Wifi,
    Bluetooth,
    TunTap,
    WireGuard,
    Generic,
    Other,
    #[default]
    Unknown
}

impl From<u32> for DeviceType {
    fn from(device_type: u32) -> Self {
        match device_type {
            1 => Self::Ethernet,
            2 => Self::Wifi,
            5 => Self::Bluetooth,
            14 => Self::Generic,
            16 => Self::TunTap,
            29 => Self::WireGuard,
            3..=32 => Self::Other,
            _ => Self::Unknown
        }
    }
}

impl From<u32> for ConnectivityState {
    fn from(state: u32) -> Self {
        match state {
            1 => Self::None,
            2 => Self::Portal,
            3 => Self::Loss,
            4 => Self::Full,
            _ => Self::Unknown
        }
    }
}

impl From<String> for ConnectivityState {
    fn from(state: String) -> Self {
        match state.as_str() {
            "inactive" | "disconnected" => Self::None,
            "portal" => Self::Portal,
            "failed" => Self::Loss,
            "connected" => Self::Full,
            _ => Self::Unknown
        }
    }
}

impl From<Vec<Self>> for ConnectivityState {
    fn from(states: Vec<Self>) -> Self {
        if states.is_empty() {
            return Self::Unknown;
        }

        let mut state = states[0];
        for s in states.iter().skip(1) {
            if Into::<u32>::into(*s) >= state.into() {
                state = *s;
            }
        }

        state
    }
}

impl From<ConnectivityState> for u32 {
    fn from(val: ConnectivityState) -> Self {
        match val {
            ConnectivityState::None => 1,
            ConnectivityState::Portal => 2,
            ConnectivityState::Loss => 3,
            ConnectivityState::Full => 4,
            ConnectivityState::Unknown => 0
        }
    }
}

impl From<u32> for DeviceState {
    fn from(device_state: u32) -> Self {
        match device_state {
            10 => Self::Unmanaged,
            20 => Self::Unavailable,
            30 => Self::Disconnected,
            40 => Self::Prepare,
            50 => Self::Config,
            60 => Self::NeedAuth,
            70 => Self::IpConfig,
            80 => Self::IpCheck,
            90 => Self::Secondaries,
            100 => Self::Activated,
            110 => Self::Deactivating,
            120 => Self::Failed,
            _ => Self::Unknown
        }
    }
}
