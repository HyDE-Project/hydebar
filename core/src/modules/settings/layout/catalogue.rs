//! Which modules are on the bar and which can still be added.

use hydebar_proto::config::{ModuleName, Modules};

use super::Section;

/// Modules already placed somewhere on the bar.
#[must_use]
pub fn placed(modules: &Modules) -> Vec<ModuleName> {
    Section::ALL
        .into_iter()
        .flat_map(|section| section.entries(modules))
        .map(|entry| entry.module)
        .collect()
}

/// Modules offered by the editor that are not on the bar yet.
///
/// `custom` names the modules the user defined themselves, so they can be
/// placed like any built in one.
#[must_use]
pub fn available(modules: &Modules, custom: &[String]) -> Vec<ModuleName> {
    let placed = placed(modules);

    ModuleName::BUILT_IN
        .into_iter()
        .chain(custom.iter().cloned().map(ModuleName::Custom))
        .filter(|module| !placed.contains(module))
        .collect()
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::ModuleDef;

    use super::*;

    #[test]
    fn the_available_list_skips_what_is_already_placed() {
        let modules = Modules {
            left:   vec![ModuleDef::Single(ModuleName::Clock)],
            center: vec![ModuleDef::Group(vec![ModuleName::Workspaces])],
            right:  Vec::new()
        };

        let available = available(&modules, &["power".to_owned()]);

        assert!(!available.contains(&ModuleName::Clock));
        assert!(!available.contains(&ModuleName::Workspaces));
        assert!(available.contains(&ModuleName::Tray));
        assert!(available.contains(&ModuleName::Custom("power".to_owned())));
    }
}
