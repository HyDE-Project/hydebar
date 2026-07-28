//! Failures that abort startup.

use hydebar_core::config::ConfigLoadError;

use crate::instance::InstanceError;

/// Reason the bar could not start.
#[derive(Debug)]
pub enum MainError {
    Logger(flexi_logger::FlexiLoggerError),
    Config(ConfigLoadError),
    Iced(iced::Error),
    Runtime(std::io::Error),
    /// The process could not become the single running instance.
    Instance(InstanceError),
    BusCapacity
}

impl std::fmt::Display for MainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Logger(err) => write!(f, "failed to initialize logger: {}", err),
            Self::Config(err) => write!(f, "configuration error: {}", err),
            Self::Iced(err) => write!(f, "iced runtime error: {}", err),
            Self::Runtime(err) => write!(f, "failed to build the async runtime: {}", err),
            Self::Instance(err) => write!(f, "single instance error: {}", err),
            Self::BusCapacity => write!(f, "invalid event bus capacity")
        }
    }
}

impl std::error::Error for MainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Logger(err) => Some(err),
            Self::Config(err) => Some(err),
            Self::Iced(err) => Some(err),
            Self::Runtime(err) => Some(err),
            Self::Instance(err) => Some(err),
            Self::BusCapacity => None
        }
    }
}

impl From<flexi_logger::FlexiLoggerError> for MainError {
    fn from(err: flexi_logger::FlexiLoggerError) -> Self {
        Self::Logger(err)
    }
}

impl From<ConfigLoadError> for MainError {
    fn from(err: ConfigLoadError) -> Self {
        Self::Config(err)
    }
}

impl From<InstanceError> for MainError {
    fn from(err: InstanceError) -> Self {
        Self::Instance(err)
    }
}

impl From<iced::Error> for MainError {
    fn from(err: iced::Error) -> Self {
        Self::Iced(err)
    }
}
