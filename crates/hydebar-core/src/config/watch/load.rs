//! Loading and validation of a candidate configuration file.

use std::{path::Path, sync::Arc};

use hydebar_proto::theme_source::HydeTheme;
use iced::futures::{
    SinkExt,
    channel::mpsc::{SendError, Sender}
};
use log::error;

use super::ConfigEvent;
use crate::config::{
    Config, ConfigApplied, ConfigManager, ConfigReadError, ConfigUpdateError, read_config,
    read_config_with
};

/// Loads the configuration file with the theme the environment points at.
pub(crate) fn load_candidate(
    path: &Path,
    manager: &ConfigManager
) -> Result<ConfigApplied, ConfigUpdateError> {
    let config = read_config(path).map_err(convert_read_error)?;

    apply_candidate(config, manager)
}

/// Reloads the configuration while overlaying the theme produced by `theme`.
///
/// The desktop theme is not part of the configuration file, so a theme switch
/// has to re-run the very same load the file watcher runs. Taking the theme as
/// a closure is what lets the theme watcher overlay the directory it actually
/// watches instead of whatever the environment happens to say.
pub(crate) fn load_candidate_with<F>(
    path: &Path,
    manager: &ConfigManager,
    theme: F
) -> Result<ConfigApplied, ConfigUpdateError>
where
    F: FnOnce() -> HydeTheme
{
    let config = read_config_with(path, theme).map_err(convert_read_error)?;

    apply_candidate(config, manager)
}

/// Validates a freshly read configuration and hands it to the manager.
fn apply_candidate(
    config: Config,
    manager: &ConfigManager
) -> Result<ConfigApplied, ConfigUpdateError> {
    config.validate()?;

    manager
        .apply(config)
        .map_err(|err| ConfigUpdateError::state(err.to_string()))
}

fn convert_read_error(err: ConfigReadError) -> ConfigUpdateError {
    match err {
        ConfigReadError::Read {
            path,
            source
        } => ConfigUpdateError::read(path, &source),
        ConfigReadError::Parse {
            path,
            source
        } => ConfigUpdateError::parse(path, &source)
    }
}

/// Reports that the configuration could not be refreshed.
pub(crate) async fn send_degradation(
    output: &mut Sender<ConfigEvent>,
    manager: Arc<ConfigManager>,
    reason: ConfigUpdateError
) -> Result<(), SendError> {
    match manager.degraded(reason) {
        Ok(degradation) => output.send(ConfigEvent::Degraded(degradation)).await,
        Err(err) => {
            error!("Failed to report configuration degradation: {err}");
            Ok(())
        }
    }
}
