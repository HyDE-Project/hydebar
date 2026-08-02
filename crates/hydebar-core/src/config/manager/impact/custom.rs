//! The custom-module diff: added, changed and departed definitions.

use std::collections::HashMap;

use hydebar_proto::config::{CustomModuleDef, ModuleName};

use super::ConfigImpact;

/// Marks every custom module a reload added, changed or removed.
pub(super) fn update_custom_module_impact(
    impact: &mut ConfigImpact,
    previous: &[CustomModuleDef],
    next: &[CustomModuleDef]
) {
    let previous_map: HashMap<&str, &CustomModuleDef> = previous
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect();
    let next_map: HashMap<&str, &CustomModuleDef> = next
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect();

    for (name, module) in &next_map {
        let needs_update = previous_map
            .get(name)
            .is_none_or(|current| *current != *module);

        if needs_update {
            impact
                .affected_modules
                .insert(ModuleName::Custom((*name).to_string()));
        }
    }

    for name in previous_map.keys() {
        if !next_map.contains_key(name) {
            impact
                .affected_modules
                .insert(ModuleName::Custom((*name).to_string()));
        }
    }
}
