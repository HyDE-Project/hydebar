//! Loading and validation of a candidate configuration file.

use std::{path::Path, sync::Arc};

use iced::futures::{
    SinkExt,
    channel::mpsc::{SendError, Sender}
};
use log::error;

use super::ConfigEvent;
use crate::config::{
    ConfigApplied, ConfigManager, ConfigReadError, ConfigUpdateError, read_config
};

pub(super) fn load_candidate(
    path: &Path,
    manager: &ConfigManager
) -> Result<ConfigApplied, ConfigUpdateError> {
    let config = read_config(path).map_err(convert_read_error)?;

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

pub(super) async fn send_degradation(
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
