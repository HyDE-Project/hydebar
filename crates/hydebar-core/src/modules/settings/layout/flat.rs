//! Flattening a section into one entry per module.
//!
//! The configuration groups modules into islands, which is what the bar draws,
//! but it is a poor thing to edit: a row standing for three modules leaves no
//! way to say which of the three a button acts on. Flattening turns a section
//! into one row per module, each carrying whether it shares an island with the
//! row above, and rebuilding turns the rows back into islands.

use hydebar_proto::config::{ModuleDef, ModuleName};

/// One module of a section, as the editor lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Module this row stands for.
    pub module: ModuleName,
    /// Whether it shares an island with the row above.
    ///
    /// The first row of a section never joins upwards.
    pub joined: bool
}

/// Turns the islands of a section into one entry per module.
#[must_use]
pub fn flatten(section: &[ModuleDef]) -> Vec<Entry> {
    let mut entries = Vec::new();

    for island in section {
        match island {
            ModuleDef::Single(module) => entries.push(Entry {
                module: module.clone(),
                joined: false
            }),
            ModuleDef::Group(group) => {
                for (index, module) in group.iter().enumerate() {
                    entries.push(Entry {
                        module: module.clone(),
                        joined: index > 0
                    });
                }
            }
        }
    }

    entries
}

/// Turns entries back into the islands the configuration stores.
#[must_use]
pub fn rebuild(entries: &[Entry]) -> Vec<ModuleDef> {
    let mut islands: Vec<Vec<ModuleName>> = Vec::new();

    for entry in entries {
        match (entry.joined, islands.last_mut()) {
            (true, Some(island)) => island.push(entry.module.clone()),
            _ => islands.push(vec![entry.module.clone()])
        }
    }

    islands
        .into_iter()
        .map(|island| match island.len() {
            1 => ModuleDef::Single(island.into_iter().next().expect("one module")),
            _ => ModuleDef::Group(island)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn island(names: &[&str]) -> ModuleDef {
        match names {
            [one] => ModuleDef::Single(ModuleName::Custom((*one).to_owned())),
            many => ModuleDef::Group(
                many.iter()
                    .map(|name| ModuleName::Custom((*name).to_owned()))
                    .collect()
            )
        }
    }

    #[test]
    fn every_module_gets_its_own_entry() {
        let section = vec![island(&["a", "b"]), island(&["c"])];

        let entries = flatten(&section);

        assert_eq!(entries.len(), 3);
        assert!(!entries[0].joined);
        assert!(entries[1].joined);
        assert!(!entries[2].joined);
    }

    #[test]
    fn rebuilding_restores_the_islands() {
        let section = vec![
            island(&["a", "b"]),
            island(&["c"]),
            island(&["d", "e", "f"]),
        ];

        assert_eq!(rebuild(&flatten(&section)), section);
    }

    #[test]
    fn a_leading_join_still_opens_an_island() {
        let entries = vec![Entry {
            module: ModuleName::Clock,
            joined: true
        }];

        assert_eq!(
            rebuild(&entries),
            vec![ModuleDef::Single(ModuleName::Clock)]
        );
    }

    #[test]
    fn an_empty_section_survives_the_round_trip() {
        assert!(flatten(&[]).is_empty());
        assert!(rebuild(&[]).is_empty());
    }
}
