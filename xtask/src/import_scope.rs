//! Walking one migration pass: expansion, deduplication, reporting.
//!
//! The walk shares its tables through one state instead of threading nine
//! arguments through two functions.

use std::collections::{BTreeMap, BTreeSet};

use hydebar_proto::{bar_layout::module_for, config::ModuleName};

use super::import::SectionEntry;
/// Working state of one migration pass.
///
/// Group expansion, deduplication and reporting share these tables; bundling
/// them keeps the walk to two short methods instead of two functions taking
/// nine arguments each.
pub struct Scope<'a> {
    /// Pill groups declared beside the module arrays.
    groups:              &'a BTreeMap<String, Vec<String>>,
    /// Scripted customs without a native counterpart, by tail.
    custom_set:          BTreeSet<&'a str>,
    /// Every scripted custom tail, for dropped-script tracking.
    exec_tails:          BTreeSet<&'a str>,
    /// Group chain above the current entry, against cycles.
    visiting:            Vec<String>,
    /// Entries of the section being built.
    entries:             Vec<SectionEntry>,
    /// Spellings already emitted into the section.
    seen:                BTreeSet<String>,
    /// Entries with no counterpart, in first-seen order.
    unmapped:            Vec<String>,
    /// Scripted customs a native module displaced.
    dropped_scripts:     Vec<String>,
    /// Scripted customs placed as custom modules.
    placed_customs:      BTreeSet<String>,
    /// Scripted customs met while walking, placed or displaced.
    encountered_customs: BTreeSet<String>
}

impl Scope<'_> {
    /// Starts one migration pass over shared source tables.
    pub const fn new<'a>(
        groups: &'a BTreeMap<String, Vec<String>>,
        custom_set: BTreeSet<&'a str>,
        exec_tails: BTreeSet<&'a str>
    ) -> Scope<'a> {
        Scope {
            groups,
            custom_set,
            exec_tails,
            visiting: Vec::new(),
            entries: Vec::new(),
            seen: BTreeSet::new(),
            unmapped: Vec::new(),
            dropped_scripts: Vec::new(),
            placed_customs: BTreeSet::new(),
            encountered_customs: BTreeSet::new()
        }
    }
    /// Clears the per-section state for the next bar section.
    pub fn begin_section(&mut self) {
        self.visiting.clear();
        self.entries.clear();
        self.seen.clear();
    }

    /// Whether the section being built holds anything.
    pub const fn has_entries(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Takes the built section entries, leaving an empty section behind.
    pub fn take_entries(&mut self) -> Vec<SectionEntry> {
        std::mem::take(&mut self.entries)
    }

    /// Whether a scripted custom was met while walking, placed or displaced.
    pub fn is_encountered(&self, tail: &str) -> bool {
        self.encountered_customs.contains(tail)
    }

    /// Takes the placed scripted customs, leaving an empty set behind.
    pub fn take_placed(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.placed_customs)
    }

    /// Takes the reporting lists, leaving empty ones behind.
    pub fn take_reports(&mut self) -> (Vec<String>, Vec<String>) {
        (
            std::mem::take(&mut self.unmapped),
            std::mem::take(&mut self.dropped_scripts)
        )
    }

    /// Notes an entry with no counterpart, once.
    pub fn note_unmapped(&mut self, name: &str) {
        if !self.unmapped.iter().any(|known| known == name) {
            self.unmapped.push(name.to_owned());
        }
    }

    /// Notes a script a native module displaced, once.
    pub fn note_dropped(&mut self, name: &str) {
        if !self.dropped_scripts.iter().any(|known| known == name) {
            self.dropped_scripts.push(name.to_owned());
        }
    }

    /// Notes a defined custom nothing placed, once.
    pub fn note_unplaced(&mut self, tail: &str) {
        self.unmapped
            .push(format!("custom/{tail} (defined but not placed)"));
    }

    /// Expands one section entry, following pill groups recursively.
    ///
    /// A group becomes one island of whatever its members map to; cycles
    /// fall away silently, and whatever maps to nothing is reported, never
    /// emitted.
    pub fn expand_entry(&mut self, name: &str) {
        if let Some(group) = name.strip_prefix("group/") {
            if self.visiting.iter().any(|known| known == group) {
                return;
            }
            self.visiting.push(group.to_owned());

            // The exact tail first (`pill#1`); the un-suffixed family only
            // when nothing answers to it.
            let base = group.split('#').next().unwrap_or(group);
            let declared = self
                .groups
                .get(group)
                .or_else(|| {
                    if base == group {
                        None
                    } else {
                        self.groups.get(base)
                    }
                })
                .cloned()
                .unwrap_or_default();

            let mut members = Vec::new();
            for member in &declared {
                if let Some(mapped) = self.map_member(member) {
                    members.push(mapped);
                }
            }
            self.visiting.pop();

            if !members.is_empty() {
                self.entries.push(SectionEntry::Group(members));
            }
            return;
        }

        if let Some(mapped) = self.map_member(name) {
            self.entries.push(SectionEntry::Single(mapped));
        }
    }

    /// Maps one Waybar module name, tracking scripts and the unknown.
    pub fn map_member(&mut self, name: &str) -> Option<String> {
        let plain = name.split('#').next().unwrap_or(name);

        if let Some(tail) = plain
            .strip_prefix("custom/")
            .filter(|tail| self.exec_tails.contains(tail))
        {
            self.encountered_customs.insert(tail.to_owned());
        }
        match module_for(plain, &self.custom_set) {
            None => {
                self.note_unmapped(name);
                None
            }
            Some(ModuleName::Custom(tail)) => {
                if self.seen.insert(tail.clone()) {
                    self.placed_customs.insert(tail.clone());
                    Some(tail)
                } else {
                    None
                }
            }
            Some(module) => {
                if plain
                    .strip_prefix("custom/")
                    .is_some_and(|tail| self.exec_tails.contains(tail))
                {
                    self.note_dropped(name);
                }
                let spelling = module.as_str().to_owned();
                if self.seen.insert(spelling.clone()) {
                    Some(spelling)
                } else {
                    None
                }
            }
        }
    }
}
