//! Configuration for the media player module.

use serde::Deserialize;

/// Media player module behaviour.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MediaPlayerModuleConfig {
    #[serde(default = "default_media_player_max_title_length")]
    pub max_title_length: u32
}

impl Default for MediaPlayerModuleConfig {
    fn default() -> Self {
        Self {
            max_title_length: default_media_player_max_title_length()
        }
    }
}

fn default_media_player_max_title_length() -> u32 {
    100
}
