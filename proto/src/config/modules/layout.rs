//! The three position arrays of the bar and the questions asked of them.

use serde::Deserialize;

use super::name::ModuleName;

/// Layout definition describing which modules render in each region.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum ModuleDef {
    Single(ModuleName),
    Group(Vec<ModuleName>)
}

/// Overall module layout configuration.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Modules {
    #[serde(default)]
    pub left:   Vec<ModuleDef>,
    #[serde(default)]
    pub center: Vec<ModuleDef>,
    #[serde(default)]
    pub right:  Vec<ModuleDef>
}

impl Modules {
    /// Every module name the layout places, section by section.
    ///
    /// Groups are flattened because a grouped module is as much on screen as a
    /// standalone one; callers asking "is this drawn anywhere" must not have to
    /// know how the user chose to bundle it.
    pub fn placed(&self) -> impl Iterator<Item = &ModuleName> {
        self.left
            .iter()
            .chain(self.center.iter())
            .chain(self.right.iter())
            .flat_map(|definition| match definition {
                ModuleDef::Single(name) => std::slice::from_ref(name),
                ModuleDef::Group(group) => group.as_slice()
            })
    }

    /// Reports whether `name` is drawn in any section of the bar.
    ///
    /// Registering a module spawns the background work behind it — pollers,
    /// D-Bus listeners, spawned commands — and a module the layout never
    /// renders has no reader for what that work produces. Asking this
    /// before registration keeps an unused module from waking the runtime
    /// and repainting every surface on an otherwise idle bar.
    #[must_use]
    pub fn hosts(&self, name: &ModuleName) -> bool {
        self.placed().any(|placed| placed == name)
    }

    /// Reports whether any of `names` is drawn in the bar.
    ///
    /// Several bar entries can share one background worker: the control centre
    /// services feed the `Audio`, `Network`, `Bluetooth` and `PowerProfile`
    /// readouts alike, so the worker has to stay alive while at least one of
    /// them is on screen.
    #[must_use]
    pub fn hosts_any(&self, names: &[ModuleName]) -> bool {
        self.placed().any(|placed| names.contains(placed))
    }
}

impl Default for Modules {
    fn default() -> Self {
        Self {
            left:   vec![
                ModuleDef::Single(ModuleName::SystemInfo),
                ModuleDef::Single(ModuleName::Clock),
            ],
            center: vec![ModuleDef::Group(vec![
                ModuleName::Workspaces,
                ModuleName::WindowTitle,
            ])],
            right:  vec![
                ModuleDef::Group(vec![ModuleName::Updates, ModuleName::ControlCenter]),
                ModuleDef::Group(vec![
                    ModuleName::Privacy,
                    ModuleName::Tray,
                    ModuleName::Battery,
                ]),
                ModuleDef::Group(vec![ModuleName::Clipboard, ModuleName::AppLauncher]),
            ]
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn default_modules_match_expected_layout() {
        let modules = Modules::default();
        assert_eq!(
            modules.left,
            vec![
                ModuleDef::Single(ModuleName::SystemInfo),
                ModuleDef::Single(ModuleName::Clock),
            ]
        );
        assert_eq!(
            modules.center,
            vec![ModuleDef::Group(vec![
                ModuleName::Workspaces,
                ModuleName::WindowTitle,
            ])]
        );
        assert_eq!(
            modules.right,
            vec![
                ModuleDef::Group(vec![ModuleName::Updates, ModuleName::ControlCenter]),
                ModuleDef::Group(vec![
                    ModuleName::Privacy,
                    ModuleName::Tray,
                    ModuleName::Battery,
                ]),
                ModuleDef::Group(vec![ModuleName::Clipboard, ModuleName::AppLauncher]),
            ]
        );
    }

    #[test]
    fn a_module_placed_in_a_group_counts_as_hosted() {
        let modules = Modules::default();

        assert!(modules.hosts(&ModuleName::Tray));
        assert!(modules.hosts(&ModuleName::Clock));
    }

    #[test]
    fn a_module_absent_from_every_section_is_not_hosted() {
        let modules = Modules::default();

        assert!(!modules.hosts(&ModuleName::MediaPlayer));
        assert!(!modules.hosts(&ModuleName::Custom("weather".to_string())));
    }

    #[test]
    fn hosts_any_matches_when_a_single_alias_is_placed() {
        let modules = Modules {
            left:   vec![ModuleDef::Single(ModuleName::Network)],
            center: Vec::new(),
            right:  Vec::new()
        };

        assert!(modules.hosts_any(&[ModuleName::ControlCenter, ModuleName::Network]));
        assert!(!modules.hosts_any(&[ModuleName::ControlCenter, ModuleName::Bluetooth]));
    }

    #[test]
    fn placed_flattens_every_section_in_order() {
        let modules = Modules {
            left:   vec![ModuleDef::Single(ModuleName::Clock)],
            center: vec![ModuleDef::Group(vec![
                ModuleName::Workspaces,
                ModuleName::WindowTitle,
            ])],
            right:  vec![ModuleDef::Single(ModuleName::Tray)]
        };

        let placed: Vec<&ModuleName> = modules.placed().collect();

        assert_eq!(
            placed,
            vec![
                &ModuleName::Clock,
                &ModuleName::Workspaces,
                &ModuleName::WindowTitle,
                &ModuleName::Tray
            ]
        );
    }

    #[test]
    fn an_empty_layout_hosts_nothing() {
        let modules = Modules {
            left:   Vec::new(),
            center: Vec::new(),
            right:  Vec::new()
        };

        assert_eq!(modules.placed().count(), 0);
        assert!(!modules.hosts(&ModuleName::Clock));
    }
}
