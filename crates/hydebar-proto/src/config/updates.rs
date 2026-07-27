//! Configuration for the updates module.

use serde::Deserialize;

/// Commands used to query and apply system package updates.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UpdatesModuleConfig {
    pub check_cmd:  String,
    pub update_cmd: String
}
