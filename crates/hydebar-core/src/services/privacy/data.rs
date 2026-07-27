//! Values describing which devices are currently in use.

use super::{WEBCAM_DEVICE_PATH, events::is_device_in_use};

/// Media class reported by PipeWire for an application node.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Media {
    /// The node represents a video stream, typically screen sharing.
    Video,
    /// The node represents an audio stream, typically microphone usage.
    Audio
}

/// Metadata describing an application node that is accessing privacy-sensitive
/// resources.
#[derive(Debug, Clone)]
pub struct ApplicationNode {
    /// Identifier assigned by PipeWire.
    pub id:    u32,
    /// Media classification of the node.
    pub media: Media
}

/// Aggregated privacy information exposed to UI consumers.
#[derive(Debug, Clone)]
pub struct PrivacyData {
    pub(super) nodes:         Vec<ApplicationNode>,
    pub(super) webcam_access: i32
}

impl PrivacyData {
    pub(super) fn new() -> Self {
        Self {
            nodes:         Vec::new(),
            webcam_access: is_device_in_use(WEBCAM_DEVICE_PATH)
        }
    }

    /// Returns `true` when no privacy-sensitive resources are currently in use.
    pub fn no_access(&self) -> bool {
        self.nodes.is_empty() && self.webcam_access == 0
    }

    /// Returns `true` when an audio input node is active.
    pub fn microphone_access(&self) -> bool {
        self.nodes.iter().any(|node| node.media == Media::Audio)
    }

    /// Returns `true` while the webcam device is reported as in use.
    pub fn webcam_access(&self) -> bool {
        self.webcam_access > 0
    }

    /// Returns `true` when a video capture node (typically screen sharing) is
    /// active.
    pub fn screenshare_access(&self) -> bool {
        self.nodes.iter().any(|node| node.media == Media::Video)
    }
}
