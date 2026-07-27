//! Definition of user provided modules driven by external commands.

use std::collections::HashMap;

use serde::Deserialize;
use serde_with::serde_as;

use super::serde_helpers::RegexCfg;

/// A module whose content is produced by an external command.
#[serde_as]
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CustomModuleDef {
    pub name:    String,
    pub command: String,
    #[serde(default)]
    pub icon:    Option<String>,

    /// Yields json lines containing text, alt and an optional tooltip.
    pub listen_cmd: Option<String>,
    /// Map of regex to icon.
    pub icons:      Option<HashMap<RegexCfg, String>>,
    /// Regex selecting output that raises an alert.
    pub alert:      Option<RegexCfg>
}
