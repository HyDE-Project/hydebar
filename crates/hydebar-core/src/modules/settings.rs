//! Bar module configuring the bar itself.
//!
//! Every choice made here is written straight back into the configuration file
//! the bar was started from. The file watcher picks the change up and reloads,
//! so the menu never holds state of its own: what it draws is always what the
//! running configuration says.

mod layout {
    //! Pure edits of the module layout.
    //!
    //! Every operation of the module editor is a function from one layout to
    //! the next, with no rendering and no file access in sight: the editor
    //! stays a thin shell over rules that can be tested on their own.
    //!
    //! Edits address a single module rather than a whole island. The
    //! configuration stores islands, which is what the bar draws, but an
    //! edit stated against an island cannot say which of its modules it
    //! means.

    mod flat {
        //! Flattening a section into one entry per module.
        //!
        //! The configuration groups modules into islands, which is what the bar
        //! draws, but it is a poor thing to edit: a row standing for
        //! three modules leaves no way to say which of the three a
        //! button acts on. Flattening turns a section into one row per
        //! module, each carrying whether it shares an island with the
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
    }

    pub use flat::Entry;
    use flat::{flatten, rebuild};
    use hydebar_proto::config::{ModuleDef, ModuleName, Modules};

    /// Region of the bar a list of modules belongs to.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub enum Section {
        #[default]
        Left,
        Center,
        Right
    }

    impl Section {
        /// Every section, in the order the editor lists them.
        pub const ALL: [Section; 3] = [Section::Left, Section::Center, Section::Right];

        /// Name shown above the row of this section.
        #[must_use]
        pub const fn label(self) -> &'static str {
            match self {
                Section::Left => "Left",
                Section::Center => "Center",
                Section::Right => "Right"
            }
        }

        /// Section on the left of this one, if any.
        #[must_use]
        pub const fn before(self) -> Option<Self> {
            match self {
                Section::Left => None,
                Section::Center => Some(Section::Left),
                Section::Right => Some(Section::Center)
            }
        }

        /// Section on the right of this one, if any.
        #[must_use]
        pub const fn after(self) -> Option<Self> {
            match self {
                Section::Left => Some(Section::Center),
                Section::Center => Some(Section::Right),
                Section::Right => None
            }
        }

        /// Islands of this section.
        #[must_use]
        pub fn islands(self, modules: &Modules) -> &Vec<ModuleDef> {
            match self {
                Section::Left => &modules.left,
                Section::Center => &modules.center,
                Section::Right => &modules.right
            }
        }

        /// Modules of this section, one entry each.
        #[must_use]
        pub fn entries(self, modules: &Modules) -> Vec<Entry> {
            flatten(self.islands(modules))
        }

        /// Replaces the modules of this section.
        fn store(self, modules: &mut Modules, entries: &[Entry]) {
            let islands = rebuild(entries);

            match self {
                Section::Left => modules.left = islands,
                Section::Center => modules.center = islands,
                Section::Right => modules.right = islands
            }
        }
    }

    /// Where an edit applies.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Slot {
        /// Section the module sits in.
        pub section: Section,
        /// Position of the module inside its section.
        pub index:   usize
    }

    /// A single change the module editor can make.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum LayoutEdit {
        /// Swap the module with the one before it.
        MoveEarlier(Slot),
        /// Swap the module with the one after it.
        MoveLater(Slot),
        /// Move the module to the end of the section on the left.
        MoveToPreviousSection(Slot),
        /// Move the module to the start of the section on the right.
        MoveToNextSection(Slot),
        /// Join the module to the island above, or break it out of one.
        ToggleJoin(Slot),
        /// Drop the module from the bar.
        Remove(Slot),
        /// Append a module to a section.
        Add {
            section: Section,
            module:  ModuleName
        }
    }

    impl LayoutEdit {
        /// Slot this edit acts on, if it acts on one.
        #[must_use]
        pub fn slot(&self) -> Option<Slot> {
            match self {
                Self::MoveEarlier(slot)
                | Self::MoveLater(slot)
                | Self::MoveToPreviousSection(slot)
                | Self::MoveToNextSection(slot)
                | Self::ToggleJoin(slot)
                | Self::Remove(slot) => Some(*slot),
                Self::Add {
                    ..
                } => None
            }
        }
    }

    /// Takes the module out of its section, leaving the rest joined sensibly.
    ///
    /// A module that led an island hands the island over to the module after
    /// it, so removing the head of a group does not silently dissolve the
    /// group.
    fn take(entries: &mut Vec<Entry>, index: usize) -> Option<Entry> {
        if index >= entries.len() {
            return None;
        }

        let taken = entries.remove(index);

        if !taken.joined
            && let Some(next) = entries.get_mut(index)
        {
            next.joined = false;
        }

        Some(taken)
    }

    /// Returns the layout `edit` produces from `modules`.
    ///
    /// An edit that cannot apply, moving the first module earlier for instance,
    /// leaves the layout untouched: the editor may offer every button
    /// unconditionally and still never produce a broken layout.
    #[must_use]
    pub fn apply(modules: &Modules, edit: &LayoutEdit) -> Modules {
        let mut next = modules.clone();

        match edit {
            LayoutEdit::MoveEarlier(slot) => {
                let mut entries = slot.section.entries(modules);

                if slot.index > 0 && slot.index < entries.len() {
                    entries.swap(slot.index - 1, slot.index);
                    let first_joined = entries[0].joined;
                    entries[0].joined = false;
                    entries[slot.index].joined |= first_joined && slot.index == 1;
                    slot.section.store(&mut next, &entries);
                }
            }
            LayoutEdit::MoveLater(slot) => {
                let mut entries = slot.section.entries(modules);

                if slot.index + 1 < entries.len() {
                    entries.swap(slot.index, slot.index + 1);
                    entries[0].joined = false;
                    slot.section.store(&mut next, &entries);
                }
            }
            LayoutEdit::MoveToPreviousSection(slot) => {
                let Some(target) = slot.section.before() else {
                    return next;
                };

                let mut entries = slot.section.entries(modules);

                if let Some(mut moved) = take(&mut entries, slot.index) {
                    moved.joined = false;
                    slot.section.store(&mut next, &entries);

                    let mut into = target.entries(&next);
                    into.push(moved);
                    target.store(&mut next, &into);
                }
            }
            LayoutEdit::MoveToNextSection(slot) => {
                let Some(target) = slot.section.after() else {
                    return next;
                };

                let mut entries = slot.section.entries(modules);

                if let Some(mut moved) = take(&mut entries, slot.index) {
                    moved.joined = false;
                    slot.section.store(&mut next, &entries);

                    let mut into = target.entries(&next);
                    into.insert(0, moved);

                    if let Some(second) = into.get_mut(1) {
                        second.joined = false;
                    }

                    target.store(&mut next, &into);
                }
            }
            LayoutEdit::ToggleJoin(slot) => {
                let mut entries = slot.section.entries(modules);

                if slot.index > 0 && slot.index < entries.len() {
                    entries[slot.index].joined = !entries[slot.index].joined;
                    slot.section.store(&mut next, &entries);
                }
            }
            LayoutEdit::Remove(slot) => {
                let mut entries = slot.section.entries(modules);

                if take(&mut entries, slot.index).is_some() {
                    slot.section.store(&mut next, &entries);
                }
            }
            LayoutEdit::Add {
                section,
                module
            } => {
                let mut entries = section.entries(modules);
                entries.push(Entry {
                    module: module.clone(),
                    joined: false
                });
                section.store(&mut next, &entries);
            }
        }

        next
    }

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
        use super::*;

        fn layout(left: Vec<ModuleDef>) -> Modules {
            Modules {
                left,
                center: Vec::new(),
                right: Vec::new()
            }
        }

        fn slot(index: usize) -> Slot {
            Slot {
                section: Section::Left,
                index
            }
        }

        fn names(modules: &Modules, section: Section) -> Vec<String> {
            section
                .entries(modules)
                .into_iter()
                .map(|entry| {
                    format!(
                        "{}{}",
                        if entry.joined { "+" } else { "" },
                        entry.module.as_str()
                    )
                })
                .collect()
        }

        #[test]
        fn a_module_moves_inside_its_section() {
            let modules = layout(vec![
                ModuleDef::Single(ModuleName::Clock),
                ModuleDef::Single(ModuleName::Tray),
            ]);

            let moved = apply(&modules, &LayoutEdit::MoveLater(slot(0)));

            assert_eq!(names(&moved, Section::Left), vec!["Tray", "Clock"]);
        }

        #[test]
        fn a_move_beyond_the_ends_changes_nothing() {
            let modules = layout(vec![ModuleDef::Single(ModuleName::Clock)]);

            for edit in [
                LayoutEdit::MoveEarlier(slot(0)),
                LayoutEdit::MoveLater(slot(0)),
                LayoutEdit::MoveEarlier(slot(9))
            ] {
                assert_eq!(names(&apply(&modules, &edit), Section::Left), vec!["Clock"]);
            }
        }

        #[test]
        fn a_single_module_leaves_its_island() {
            let modules = layout(vec![ModuleDef::Group(vec![
                ModuleName::Clock,
                ModuleName::Tray,
                ModuleName::Battery,
            ])]);

            let split = apply(&modules, &LayoutEdit::ToggleJoin(slot(1)));

            assert_eq!(
                split.left,
                vec![
                    ModuleDef::Single(ModuleName::Clock),
                    ModuleDef::Group(vec![ModuleName::Tray, ModuleName::Battery])
                ]
            );
        }

        #[test]
        fn a_module_joins_the_island_above() {
            let modules = layout(vec![
                ModuleDef::Single(ModuleName::Clock),
                ModuleDef::Single(ModuleName::Tray),
            ]);

            let joined = apply(&modules, &LayoutEdit::ToggleJoin(slot(1)));

            assert_eq!(
                joined.left,
                vec![ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray])]
            );
        }

        #[test]
        fn the_first_module_of_a_section_cannot_join_upwards() {
            let modules = layout(vec![ModuleDef::Single(ModuleName::Clock)]);

            assert_eq!(
                apply(&modules, &LayoutEdit::ToggleJoin(slot(0))).left,
                modules.left
            );
        }

        #[test]
        fn removing_the_head_of_an_island_keeps_the_island() {
            let modules = layout(vec![ModuleDef::Group(vec![
                ModuleName::Clock,
                ModuleName::Tray,
                ModuleName::Battery,
            ])]);

            let removed = apply(&modules, &LayoutEdit::Remove(slot(0)));

            assert_eq!(
                removed.left,
                vec![ModuleDef::Group(vec![
                    ModuleName::Tray,
                    ModuleName::Battery
                ])]
            );
        }

        #[test]
        fn a_module_travels_between_sections() {
            let modules = Modules {
                left:   vec![ModuleDef::Single(ModuleName::Clock)],
                center: Vec::new(),
                right:  Vec::new()
            };

            let moved = apply(
                &modules,
                &LayoutEdit::MoveToNextSection(Slot {
                    section: Section::Left,
                    index:   0
                })
            );

            assert!(moved.left.is_empty());
            assert_eq!(names(&moved, Section::Center), vec!["Clock"]);

            let back = apply(
                &moved,
                &LayoutEdit::MoveToPreviousSection(Slot {
                    section: Section::Center,
                    index:   0
                })
            );

            assert_eq!(names(&back, Section::Left), vec!["Clock"]);
        }

        #[test]
        fn the_outer_sections_have_no_neighbour_beyond_them() {
            assert_eq!(Section::Left.before(), None);
            assert_eq!(Section::Right.after(), None);
        }

        #[test]
        fn a_module_moved_out_of_an_island_arrives_on_its_own() {
            let modules = Modules {
                left:   vec![ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray])],
                center: Vec::new(),
                right:  Vec::new()
            };

            let moved = apply(
                &modules,
                &LayoutEdit::MoveToNextSection(Slot {
                    section: Section::Left,
                    index:   1
                })
            );

            assert_eq!(names(&moved, Section::Left), vec!["Clock"]);
            assert_eq!(names(&moved, Section::Center), vec!["Tray"]);
        }

        #[test]
        fn an_added_module_lands_at_the_end_on_its_own() {
            let modules = layout(vec![ModuleDef::Group(vec![
                ModuleName::Clock,
                ModuleName::Tray,
            ])]);

            let added = apply(
                &modules,
                &LayoutEdit::Add {
                    section: Section::Left,
                    module:  ModuleName::Battery
                }
            );

            assert_eq!(
                names(&added, Section::Left),
                vec!["Clock", "+Tray", "Battery"]
            );
        }

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

        #[test]
        fn every_edit_but_adding_names_the_slot_it_acts_on() {
            assert_eq!(LayoutEdit::Remove(slot(2)).slot(), Some(slot(2)));
            assert_eq!(
                LayoutEdit::Add {
                    section: Section::Left,
                    module:  ModuleName::Clock
                }
                .slot(),
                None
            );
        }
    }
}
mod tab {
    //! Sections the settings window is split into.
    //!
    //! The window is about the bar. The desktop it sits on — its theme,
    //! wallpaper and colours — is driven from the theme module on the bar
    //! instead, so no page here reports or changes any of it.

    /// Page of the settings window.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub enum Tab {
        /// Height, style, colours and the rest of the look.
        #[default]
        Appearance,
        /// Which modules the bar shows, in which order and grouping.
        Modules
    }

    impl Tab {
        /// Every tab, in the order the window lists them.
        pub const ALL: [Tab; 2] = [Tab::Appearance, Tab::Modules];

        /// Name shown on the tab.
        #[must_use]
        pub const fn label(self) -> &'static str {
            match self {
                Tab::Appearance => "Appearance",
                Tab::Modules => "Modules"
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_window_opens_on_the_appearance_page() {
            assert_eq!(Tab::default(), Tab::Appearance);
        }

        #[test]
        fn every_tab_is_listed_once_and_named() {
            assert_eq!(Tab::ALL.len(), 2);

            for tab in Tab::ALL {
                assert!(!tab.label().is_empty());
            }
        }

        /// The desktop moved to the theme module wholesale, so the window is
        /// left with the two pages that are about the bar itself.
        #[test]
        fn the_window_is_about_the_bar_and_nothing_else() {
            assert_eq!(Tab::ALL, [Tab::Appearance, Tab::Modules]);
        }
    }
}
mod view {
    //! Rendering of the settings window.
    //!
    //! Every page reads its values from the running configuration, so the
    //! window shows the truth after a reload instead of a copy that
    //! drifted.

    mod appearance {
        //! Appearance page of the settings window.

        use iced::Element;

        use crate::{
            components::{
                page::{
                    metrics::row_width,
                    style,
                    widgets::{choice_row, page, rows as row_stack, section, stepper_row}
                },
                push_maybe::PushMaybe
            },
            config::{
                Appearance, AppearanceStyle, BarLayer, Config, DEFAULT_FONT_SIZE, HydeBranch,
                NotificationSource, Position
            },
            modules::settings::{Message, Settings}
        };

        /// Height the bar falls back to while the configuration names none.
        const FALLBACK_HEIGHT: f32 = 34.0;

        /// Title of the section deciding where on the screen the bar sits.
        const PLACEMENT: &str = "Placement";

        /// Title of the section deciding how large and how solid the bar is
        /// drawn.
        const SIZE: &str = "Size and colour";

        /// Title of the section about the desktop the bar sits on.
        const DESKTOP: &str = "Desktop";

        /// Every section of the page, as its title and the rows it holds.
        ///
        /// Each row is written down as its label and the controls beside it, so
        /// the width the page asks for and the number of rows it
        /// reserves height for both come from the same list: a row
        /// added to the page without an entry here would be measured
        /// out of existence. Rows the size section drops while the bar
        /// scales itself.
        ///
        /// Height, side padding and text size are then decided from the screen,
        /// so a stepper offering to change them would be offering
        /// something that is overwritten the moment it is written.
        const SCALED_ROWS: f32 = 3.0;

        const SECTIONS: [(&str, &[(&str, &[&str])]); 3] = [
            (
                PLACEMENT,
                &[
                    ("Position", &["Top", "Bottom"]),
                    ("Layer", &["Bottom", "Top", "Overlay"])
                ]
            ),
            (
                SIZE,
                &[
                    ("Style", &["Islands", "Solid", "Gradient"]),
                    ("Height", &["\u{2212}", "000", "+"]),
                    ("Side padding", &["\u{2212}", "000", "+"]),
                    ("Font size", &["\u{2212}", "000", "+"]),
                    ("Opacity", &["\u{2212}", "0.00", "+"])
                ]
            ),
            (DESKTOP, &[])
        ];

        /// Label of the row the notification source is picked on.
        ///
        /// Kept out of [`SECTIONS`] because its choices are named by the source
        /// list itself rather than written down here.
        const NOTIFICATIONS: &str = "Notifications";

        /// Label of the row the HyDE branch is picked on.
        ///
        /// Kept out of [`SECTIONS`] like the notification row, and drawn only
        /// while the updates module is configured: the choice is stored in
        /// that module's section of the file, and writing into a section
        /// that does not exist would leave one behind that cannot be read.
        const HYDE_BRANCH: &str = "HyDE branch";

        /// Renders the appearance page against the running `config`.
        ///
        /// Sizes are shown as they are written in the file, not as the bar
        /// draws them: the window magnifies what it renders, and a
        /// stepper that showed the magnified size would write it back
        /// and magnify it a second time.
        ///
        /// The side padding is shown as the one in force rather than as the one
        /// the file names, since a file that names none leaves the bar
        /// following the window gaps of the compositor: stepping from
        /// the gap actually drawn is what makes the first press nudge
        /// the bar instead of jumping it.
        pub(super) fn view(
            config: &Config,
            opacity: f32,
            magnification: f32
        ) -> Element<'_, Message> {
            let appearance: &Appearance = &config.appearance;
            let magnification = if magnification > 0.0 {
                magnification
            } else {
                1.0
            };
            let font_size = appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
            let written_font_size = font_size / magnification;
            let height = appearance.height.unwrap_or(FALLBACK_HEIGHT) / magnification;
            let side_padding = appearance.bar_padding()[1] / magnification;

            let placement = row_stack(font_size)
                .push(choice_row(
                    "Position",
                    vec![
                        ("Top", Position::Top, config.position == Position::Top),
                        (
                            "Bottom",
                            Position::Bottom,
                            config.position == Position::Bottom
                        ),
                    ],
                    Message::SetPosition,
                    font_size,
                    opacity
                ))
                .push(choice_row(
                    "Layer",
                    vec![
                        ("Bottom", BarLayer::Bottom, config.layer == BarLayer::Bottom),
                        ("Top", BarLayer::Top, config.layer == BarLayer::Top),
                        (
                            "Overlay",
                            BarLayer::Overlay,
                            config.layer == BarLayer::Overlay
                        ),
                    ],
                    Message::SetLayer,
                    font_size,
                    opacity
                ));

            let size = row_stack(font_size)
                .push(choice_row(
                    "Style",
                    vec![
                        (
                            "Islands",
                            AppearanceStyle::Islands,
                            appearance.style == AppearanceStyle::Islands
                        ),
                        (
                            "Solid",
                            AppearanceStyle::Solid,
                            appearance.style == AppearanceStyle::Solid
                        ),
                        (
                            "Gradient",
                            AppearanceStyle::Gradient,
                            appearance.style == AppearanceStyle::Gradient
                        ),
                    ],
                    Message::SetStyle,
                    font_size,
                    opacity
                ))
                .push_maybe((!appearance.auto_scale).then(|| {
                    stepper_row(
                        "Height",
                        format!("{height:.0}"),
                        Message::SetHeight(Settings::height_below(height)),
                        Message::SetHeight(Settings::height_above(height)),
                        font_size,
                        opacity
                    )
                }))
                .push_maybe((!appearance.auto_scale).then(|| {
                    stepper_row(
                        "Side padding",
                        format!("{side_padding:.0}"),
                        Message::SetSidePadding(Settings::side_padding_below(side_padding)),
                        Message::SetSidePadding(Settings::side_padding_above(side_padding)),
                        font_size,
                        opacity
                    )
                }))
                .push_maybe((!appearance.auto_scale).then(|| {
                    stepper_row(
                        "Font size",
                        format!("{written_font_size:.0}"),
                        Message::SetFontSize(Settings::font_size_below(written_font_size)),
                        Message::SetFontSize(Settings::font_size_above(written_font_size)),
                        font_size,
                        opacity
                    )
                }))
                .push(stepper_row(
                    "Opacity",
                    format!("{:.2}", appearance.opacity),
                    Message::SetOpacity(Settings::opacity_below(appearance.opacity)),
                    Message::SetOpacity(Settings::opacity_above(appearance.opacity)),
                    font_size,
                    opacity
                ));

            let desktop = row_stack(font_size)
                .push(choice_row(
                    NOTIFICATIONS,
                    NotificationSource::ALL
                        .into_iter()
                        .map(|source| {
                            (
                                source.label(),
                                source,
                                config.notifications.source == source
                            )
                        })
                        .collect(),
                    Message::SetNotificationSource,
                    font_size,
                    opacity
                ))
                .push_maybe(config.updates.as_ref().map(|updates| {
                    choice_row(
                        HYDE_BRANCH,
                        HydeBranch::ALL
                            .into_iter()
                            .map(|branch| (branch.label(), branch, updates.hyde_branch == branch))
                            .collect(),
                        Message::SetHydeBranch,
                        font_size,
                        opacity
                    )
                }));

            page(font_size)
                .push(section(PLACEMENT, placement.into(), font_size))
                .push(section(SIZE, size.into(), font_size))
                .push(section(DESKTOP, desktop.into(), font_size))
                .into()
        }

        /// Rows this page draws, its section headings counted in.
        ///
        /// The notification row is the one that is not written down in
        /// [`SECTIONS`], so it is added here rather than baked into a
        /// literal that could drift.
        #[must_use]
        pub(super) fn rows(auto_scale: bool, hyde_branch: bool) -> f32 {
            let settings: usize = SECTIONS.iter().map(|(_, rows)| rows.len()).sum();
            let scaled = if auto_scale { SCALED_ROWS } else { 0.0 };
            let branch = if hyde_branch { 1.0 } else { 0.0 };

            SECTIONS.len() as f32 * style::SECTION_TITLE_ROWS + settings as f32 + 1.0 + branch
                - scaled
        }

        /// Longest row of this page, which is how wide the window has to be.
        ///
        /// The notification row is measured from the choices themselves rather
        /// than from a copy of their names: a fourth source, or a
        /// renamed one, would otherwise be cut off by a window sized
        /// for the old list.
        #[must_use]
        pub(super) fn desired_width(font_size: f32) -> f32 {
            let notifications = row_width(
                NotificationSource::ALL
                    .into_iter()
                    .map(NotificationSource::label),
                font_size
            );
            let branches = row_width(
                HydeBranch::ALL.into_iter().map(HydeBranch::label),
                font_size
            );

            SECTIONS
                .into_iter()
                .flat_map(|(_, rows)| rows.iter())
                .map(|(_, controls)| row_width(controls.iter().copied(), font_size))
                .fold(notifications.max(branches), f32::max)
        }

        /// Height this page needs.
        #[must_use]
        pub(super) fn desired_height(font_size: f32, auto_scale: bool, hyde_branch: bool) -> f32 {
            style::page_height(rows(auto_scale, hyde_branch), font_size)
        }

        #[cfg(test)]
        mod tests {
            use super::{super::metrics::text_width, *};

            /// Every label this page draws in the shared label column.
            fn labels() -> Vec<&'static str> {
                SECTIONS
                    .into_iter()
                    .flat_map(|(_, rows)| rows.iter().map(|(label, _)| *label))
                    .chain(std::iter::once(NOTIFICATIONS))
                    .collect()
            }

            #[test]
            fn the_window_is_wide_enough_for_every_notification_source() {
                let font_size = 16.0;
                let notifications = row_width(
                    NotificationSource::ALL
                        .into_iter()
                        .map(NotificationSource::label),
                    font_size
                );

                assert!(desired_width(font_size) >= notifications);
            }

            #[test]
            fn the_notification_row_is_measured_from_all_three_sources() {
                assert_eq!(NotificationSource::ALL.len(), 3);
            }

            #[test]
            fn every_notification_source_has_room_for_its_name() {
                let font_size = 16.0;

                for source in NotificationSource::ALL {
                    assert!(desired_width(font_size) >= text_width(source.label(), font_size));
                }
            }

            #[test]
            fn every_row_label_fits_the_shared_label_column() {
                let font_size = 16.0;

                for label in labels() {
                    assert!(
                        text_width(label, font_size) <= style::label_width(font_size),
                        "{label} overflows the label column"
                    );
                }
            }

            #[test]
            fn the_page_reserves_a_row_for_every_row_and_every_heading_it_draws() {
                assert_eq!(
                    rows(false, false),
                    labels().len() as f32 + SECTIONS.len() as f32
                );
                assert_eq!(rows(true, false), rows(false, false) - SCALED_ROWS);
            }

            #[test]
            fn a_configured_updates_module_earns_the_branch_row() {
                assert_eq!(rows(false, true), rows(false, false) + 1.0);
            }

            #[test]
            fn every_section_carries_a_title() {
                for (title, _) in SECTIONS {
                    assert!(!title.is_empty());
                }
            }

            #[test]
            fn the_desktop_section_holds_only_the_row_named_elsewhere() {
                // its single row is the notification source, whose choices come from
                // the source list rather than from the table, and the row count adds it
                // back on its own
                let desktop = SECTIONS
                    .iter()
                    .find(|(title, _)| *title == DESKTOP)
                    .expect("the desktop section is drawn");

                assert!(desktop.1.is_empty());
            }

            #[test]
            fn nothing_the_bar_decides_for_itself_is_offered() {
                // following the desktop theme and scaling to the screen are not
                // choices: the bar does both, always, and a switch that pretended
                // otherwise would be a switch the bar ignores
                for (_, section_rows) in SECTIONS {
                    for (label, _) in section_rows {
                        assert_ne!(*label, "Follow HyDE theme");
                        assert_ne!(*label, "Scale to the screen");
                    }
                }
            }

            #[test]
            fn the_page_height_follows_the_shared_row_pitch() {
                let font_size = 16.0;

                assert_eq!(
                    desired_height(font_size, false, false),
                    style::page_height(rows(false, false), font_size)
                );
            }
        }
    }
    mod modules {
        //! Module editor page of the settings window.
        //!
        //! The page is a small picture of the bar: three sections, the modules
        //! in the order they are drawn, islands boxed together. At rest
        //! it says only what the bar looks like. Picking a module opens
        //! a card that names it, says where it sits, and offers its
        //! actions in labelled groups, so the page stays short
        //! while every action is spelled out when it matters.
        //!
        //! Reordering is done with buttons rather than by dragging: a drag
        //! needs a pointer held along a path, which a keyboard, a
        //! trackpad in accessibility mode and a voice control cannot
        //! produce.

        use hydebar_proto::config::Config;
        use iced::Element;

        use crate::{
            components::page::{
                metrics::{button_row_width, chip_cell_width, chip_width, wrap_chips_into_rows},
                style,
                widgets::{
                    card, chip, choice_button, grid, group, labelled_row, note, outlined, page,
                    rows as row_stack, section as titled
                }
            },
            modules::settings::{
                Message,
                layout::{Entry, LayoutEdit, Section, Slot, available}
            }
        };

        /// Label of the action moving a module one place earlier.
        const MOVE_EARLIER: &str = "\u{2190} move left";
        /// Label of the action moving a module one place later.
        const MOVE_LATER: &str = "move right \u{2192}";
        /// Label of the action moving a module to the section on the left.
        const TO_LEFT: &str = "to Left";
        /// Label of the action moving a module to the section on the right.
        const TO_RIGHT: &str = "to Right";
        /// Label of the action joining a module to the island beside it.
        const MERGE: &str = "merge with the left";
        /// Label of the action breaking a module out of its island.
        const BREAK_OUT: &str = "break out";
        /// Label of the action taking a module off the bar.
        const REMOVE: &str = "take off the bar";

        /// Title of the section picking which part of the bar is being edited.
        const SECTION_PICKER: &str = "Section";

        /// Title of the section showing what the bar carries today.
        const ON_THE_BAR: &str = "On the bar";

        /// Title of the section offering the modules that are not on the bar
        /// yet.
        const CATALOGUE: &str = "Add a module";

        /// Label of the row the reordering actions sit on.
        const ORDER: &str = "Order";

        /// Label of the row the moving and removing actions sit on.
        const MOVE_IT: &str = "Move it";

        /// Sections this page draws.
        const SECTION_COUNT: f32 = 3.0;

        /// Rows the section tab strip takes.
        const TAB_ROWS: f32 = 1.0;

        /// Rows the detail card takes: its heading and its two rows of actions.
        const CARD_ROWS: f32 = 3.0;

        /// Rows the catalogue takes before it wraps.
        const CATALOGUE_ROWS: f32 = 1.0;

        /// Groups the entries of a section into the islands they form.
        fn islands(entries: &[Entry]) -> Vec<Vec<usize>> {
            let mut islands: Vec<Vec<usize>> = Vec::new();

            for (index, entry) in entries.iter().enumerate() {
                match (entry.joined, islands.last_mut()) {
                    (true, Some(island)) => island.push(index),
                    _ => islands.push(vec![index])
                }
            }

            islands
        }

        /// Island the module at `index` belongs to, counted from one.
        fn island_of(entries: &[Entry], index: usize) -> usize {
            islands(entries)
                .into_iter()
                .position(|island| island.contains(&index))
                .map_or(1, |position| position + 1)
        }

        /// Renders the card describing the picked module and its actions.
        ///
        /// The card is built from the same labelled rows as every other page,
        /// so a module's actions line up with the steppers on the
        /// appearance tab rather than forming a grid of their own.
        fn detail<'a>(
            slot: Slot,
            entries: &[Entry],
            font_size: f32,
            opacity: f32
        ) -> Element<'a, Message> {
            let Some(entry) = entries.get(slot.index) else {
                return note("that module is gone", font_size);
            };

            let button = |label: &'static str, edit: LayoutEdit| {
                choice_button(label, Message::EditLayout(edit), false, font_size, opacity)
            };

            let heading = labelled_row(
                entry.module.as_str().to_owned(),
                note(
                    format!(
                        "{} section · island {} · position {} of {}",
                        section_name(slot.section),
                        island_of(entries, slot.index),
                        slot.index + 1,
                        entries.len()
                    ),
                    font_size
                ),
                font_size
            );

            let mut order = group(font_size);

            if slot.index > 0 {
                order = order.push(button(MOVE_EARLIER, LayoutEdit::MoveEarlier(slot)));
            }

            if slot.index + 1 < entries.len() {
                order = order.push(button(MOVE_LATER, LayoutEdit::MoveLater(slot)));
            }

            if slot.index == 0 && entries.len() == 1 {
                order = order.push(note("alone in this section", font_size));
            }

            let mut actions = group(font_size);

            if let Some(before) = slot.section.before() {
                actions = actions.push(button(
                    section_button_label(before),
                    LayoutEdit::MoveToPreviousSection(slot)
                ));
            }

            if let Some(after) = slot.section.after() {
                actions = actions.push(button(
                    section_button_label(after),
                    LayoutEdit::MoveToNextSection(slot)
                ));
            }

            actions = if slot.index == 0 {
                actions.push(note("first in the island", font_size))
            } else if entry.joined {
                actions.push(button(BREAK_OUT, LayoutEdit::ToggleJoin(slot)))
            } else {
                actions.push(button(MERGE, LayoutEdit::ToggleJoin(slot)))
            };

            actions = actions.push(button(REMOVE, LayoutEdit::Remove(slot)));

            card(
                row_stack(font_size)
                    .push(heading)
                    .push(labelled_row(ORDER, order.into(), font_size))
                    .push(labelled_row(MOVE_IT, actions.into(), font_size))
                    .into(),
                font_size,
                opacity
            )
        }

        /// Label of the button moving a module into `section`.
        const fn section_button_label(section: Section) -> &'static str {
            match section {
                Section::Left => TO_LEFT,
                Section::Center => "to Center",
                Section::Right => TO_RIGHT
            }
        }

        /// Name of a section as the card spells it.
        fn section_name(section: Section) -> &'static str {
            match section {
                Section::Left => "Left",
                Section::Center => "Center",
                Section::Right => "Right"
            }
        }

        /// Renders the modules that can still be added.
        ///
        /// The chips wrap onto as many rows as the width allows, so a long
        /// catalogue never runs past the edge of the window.
        fn catalogue<'a>(
            config: &Config,
            section: Section,
            font_size: f32,
            opacity: f32,
            available_width: f32
        ) -> Element<'a, Message> {
            let custom = config
                .custom_modules
                .iter()
                .map(|module| module.name.clone())
                .collect::<Vec<_>>();

            let modules = available(&config.modules, &custom);

            if modules.is_empty() {
                return note("every module is already on the bar", font_size);
            }

            let gap = style::group_gap(font_size);
            let labels = modules
                .iter()
                .map(|module| module.as_str().to_owned())
                .collect::<Vec<_>>();

            let cell = chip_cell_width(&labels, font_size);
            let mut block = grid(font_size);

            for indices in wrap_chips_into_rows(&labels, available_width, font_size, gap) {
                let mut row = group(font_size);

                for index in indices {
                    row = row.push(chip(
                        labels[index].clone(),
                        Message::EditLayout(LayoutEdit::Add {
                            section,
                            module: modules[index].clone()
                        }),
                        false,
                        font_size,
                        opacity,
                        Some(cell)
                    ));
                }

                block = block.push(row);
            }

            block.into()
        }

        /// Renders the row of section tabs.
        fn section_tabs<'a>(
            active: Section,
            font_size: f32,
            opacity: f32
        ) -> Element<'a, Message> {
            let mut row = group(font_size);

            for section in Section::ALL {
                row = row.push(choice_button(
                    section.label(),
                    Message::SelectSection(section),
                    section == active,
                    font_size,
                    opacity
                ));
            }

            row.into()
        }

        /// Renders the islands of one section, one island per row.
        fn section_islands<'a>(
            section: Section,
            entries: &[Entry],
            selected: Option<Slot>,
            font_size: f32,
            opacity: f32
        ) -> Element<'a, Message> {
            if entries.is_empty() {
                return note("this section is empty", font_size);
            }

            let mut column = row_stack(font_size);

            for (number, island) in islands(entries).into_iter().enumerate() {
                let mut chips = group(font_size);

                for index in island {
                    let picked = selected
                        == Some(Slot {
                            section,
                            index
                        });

                    chips = chips.push(chip(
                        entries[index].module.as_str().to_owned(),
                        Message::SelectSlot(if picked {
                            None
                        } else {
                            Some(Slot {
                                section,
                                index
                            })
                        }),
                        picked,
                        font_size,
                        opacity,
                        None
                    ));
                }

                column = column.push(labelled_row(
                    format!("island {}", number + 1),
                    outlined(chips.into(), font_size, opacity),
                    font_size
                ));
            }

            column.into()
        }

        /// Renders the module editor against the running `config`.
        pub(super) fn view(
            config: &Config,
            opacity: f32,
            font_size: f32,
            section: Section,
            selected: Option<Slot>,
            available_width: f32
        ) -> Element<'_, Message> {
            let entries = section.entries(&config.modules);

            let mut bar = row_stack(font_size).push(section_islands(
                section, &entries, selected, font_size, opacity
            ));

            bar = match selected {
                Some(slot) if slot.section == section => {
                    bar.push(detail(slot, &entries, font_size, opacity))
                }
                _ => bar.push(note("pick a module to move, group or remove it", font_size))
            };

            page(font_size)
                .push(titled(
                    SECTION_PICKER,
                    section_tabs(section, font_size, opacity),
                    font_size
                ))
                .push(titled(ON_THE_BAR, bar.into(), font_size))
                .push(titled(
                    CATALOGUE,
                    catalogue(config, section, font_size, opacity, available_width),
                    font_size
                ))
                .into()
        }

        /// Labels of the actions one card row can offer, so the width is
        /// measured against the very strings that are drawn.
        const ACTION_LABELS: [&str; 4] = [TO_LEFT, TO_RIGHT, MERGE, REMOVE];

        /// Rows this page draws for `section`, its section headings counted in.
        #[must_use]
        pub(super) fn rows(config: &Config, section: Section) -> f32 {
            let entries = section.entries(&config.modules);

            SECTION_COUNT * style::SECTION_TITLE_ROWS
                + TAB_ROWS
                + islands(&entries).len().max(1) as f32
                + CARD_ROWS
                + CATALOGUE_ROWS
        }

        /// Longest line of this page, which is how wide the window has to be.
        ///
        /// Only the lines that are actually drawn are measured: the section
        /// tabs, the widest island of the section on show, and the
        /// widest the action card can become. The catalogue is left out
        /// on purpose, since it wraps into whatever width the rest
        /// settles on.
        #[must_use]
        pub(super) fn desired_width(config: &Config, font_size: f32, section: Section) -> f32 {
            let control = style::control_size(font_size);
            let gap = style::group_gap(font_size);
            let entries = section.entries(&config.modules);

            let tabs =
                button_row_width(Section::ALL.into_iter().map(Section::label), control, gap);

            let widest_island = islands(&entries)
                .into_iter()
                .map(|island| {
                    let count = island.len() as f32;
                    let chips: f32 = island
                        .into_iter()
                        .map(|index| chip_width(entries[index].module.as_str(), control))
                        .sum();

                    labelled_row_width(font_size)
                        + chips
                        + gap * (count - 1.0).max(0.0)
                        + style::card_overhead(font_size)
                })
                .fold(0.0_f32, f32::max);

            let card = labelled_row_width(font_size)
                + button_row_width(ACTION_LABELS, control, gap)
                + style::card_overhead(font_size);

            tabs.max(widest_island).max(card)
        }

        /// Room a labelled row spends before its controls start.
        ///
        /// The same label column every other page reserves, so an island lines
        /// up with a stepper on the appearance tab.
        fn labelled_row_width(font_size: f32) -> f32 {
            style::label_width(font_size) + style::row_gap(font_size)
        }

        /// Height this page needs for `section`.
        #[must_use]
        pub(super) fn desired_height(config: &Config, font_size: f32, section: Section) -> f32 {
            style::page_height(rows(config, section), font_size)
        }

        #[cfg(test)]
        mod tests {
            use hydebar_proto::config::{ModuleDef, ModuleName, Modules};

            use super::{super::metrics::text_width, *};

            fn entries(left: Vec<ModuleDef>) -> Vec<Entry> {
                Section::Left.entries(&Modules {
                    left,
                    center: Vec::new(),
                    right: Vec::new()
                })
            }

            fn config(left: Vec<ModuleDef>) -> Config {
                Config {
                    modules: Modules {
                        left,
                        center: Vec::new(),
                        right: Vec::new()
                    },
                    ..Config::default()
                }
            }

            #[test]
            fn neighbouring_joined_modules_form_one_island() {
                let section = entries(vec![
                    ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray]),
                    ModuleDef::Single(ModuleName::Battery),
                ]);

                assert_eq!(islands(&section), vec![vec![0, 1], vec![2]]);
            }

            #[test]
            fn a_section_of_singles_is_a_row_of_islands() {
                let section = entries(vec![
                    ModuleDef::Single(ModuleName::Clock),
                    ModuleDef::Single(ModuleName::Tray),
                ]);

                assert_eq!(islands(&section), vec![vec![0], vec![1]]);
            }

            #[test]
            fn an_empty_section_has_no_islands() {
                assert!(islands(&[]).is_empty());
            }

            #[test]
            fn a_module_knows_which_island_it_sits_in() {
                let section = entries(vec![
                    ModuleDef::Group(vec![ModuleName::Clock, ModuleName::Tray]),
                    ModuleDef::Single(ModuleName::Battery),
                ]);

                assert_eq!(island_of(&section, 0), 1);
                assert_eq!(island_of(&section, 1), 1);
                assert_eq!(island_of(&section, 2), 2);
            }

            #[test]
            fn a_missing_module_is_reported_as_the_first_island() {
                assert_eq!(island_of(&[], 3), 1);
            }

            #[test]
            fn an_empty_section_still_reserves_a_row_for_its_notice() {
                let empty = config(Vec::new());

                assert_eq!(
                    rows(&empty, Section::Left),
                    SECTION_COUNT + TAB_ROWS + 1.0 + CARD_ROWS + CATALOGUE_ROWS
                );
            }

            #[test]
            fn more_islands_make_the_page_taller() {
                let few = config(vec![ModuleDef::Single(ModuleName::Clock)]);
                let many = config(vec![
                    ModuleDef::Single(ModuleName::Clock),
                    ModuleDef::Single(ModuleName::Tray),
                    ModuleDef::Single(ModuleName::Battery),
                ]);

                assert!(
                    desired_height(&many, 16.0, Section::Left)
                        > desired_height(&few, 16.0, Section::Left)
                );
            }

            #[test]
            fn an_island_row_reserves_the_shared_label_column() {
                let font_size = 16.0;
                let modules = config(vec![ModuleDef::Single(ModuleName::Clock)]);

                assert!(
                    desired_width(&modules, font_size, Section::Left)
                        > style::label_width(font_size)
                );
            }

            #[test]
            fn every_row_label_fits_the_shared_label_column() {
                let font_size = 16.0;

                for label in ["island 00", ORDER, MOVE_IT] {
                    assert!(
                        text_width(label, font_size) <= style::label_width(font_size),
                        "{label} overflows the label column"
                    );
                }
            }

            #[test]
            fn the_page_height_follows_the_shared_row_pitch() {
                let font_size = 16.0;
                let modules = config(vec![ModuleDef::Single(ModuleName::Clock)]);

                assert_eq!(
                    desired_height(&modules, font_size, Section::Left),
                    style::page_height(rows(&modules, Section::Left), font_size)
                );
            }
        }
    }

    use iced::{
        Alignment, Element, Length,
        widget::{Column, Row}
    };

    use super::{Message, Settings, Tab};
    use crate::{
        components::{
            icons::{IconTheme, Icons, icon},
            page::{metrics, style, widgets::choice_button},
            text::text
        },
        config::{Config, DEFAULT_FONT_SIZE}
    };

    /// Title drawn beside the icon at the top of the window.
    ///
    /// Named once so the header the window measures is the header it draws: a
    /// title changed in one place only would size the window for the other
    /// one.
    const TITLE: &str = "Bar settings";

    impl Settings {
        /// Renders the settings window against the running `config`.
        ///
        /// `magnification` is the factor the bar is drawn at, so the pages can
        /// show the sizes as they are written in the file rather than
        /// as they render.
        pub fn menu_view<'a>(
            &self,
            config: &'a Config,
            opacity: f32,
            icons: &IconTheme,
            magnification: f32,
            page_width: f32
        ) -> Element<'a, Message> {
            let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
            let active = self.tab();

            let header = Row::new()
                .push(icon(icons, Icons::Settings))
                .push(text(TITLE).size(font_size).width(Length::Fill))
                .spacing(style::row_gap(font_size))
                .align_y(Alignment::Center);

            let mut tabs = Row::new().spacing(style::group_gap(font_size));

            for tab in Tab::ALL {
                tabs = tabs.push(choice_button(
                    tab.label(),
                    Message::SelectTab(tab),
                    tab == active,
                    font_size,
                    opacity
                ));
            }

            let page = match active {
                Tab::Appearance => appearance::view(config, opacity, magnification),
                Tab::Modules => modules::view(
                    config,
                    opacity,
                    font_size,
                    self.section(),
                    self.selected(),
                    page_width
                )
            };

            Column::new()
                .push(header)
                .push(tabs)
                .push(page)
                .width(Length::Fill)
                .spacing(style::window_gap(font_size))
                .into()
        }

        /// Width a page is actually given to draw into, slack excluded.
        ///
        /// The grid that wraps — the module catalogue — has to wrap against the
        /// room it gets rather than against the room the window asks for, or it
        /// would fit one chip too many and run past the edge.
        /// Width the longest row of the current page needs.
        ///
        /// The window asks for exactly this much and no more: the screen only
        /// ever caps it, it never stretches it.
        #[must_use]
        pub fn content_width(&self, config: &Config) -> f32 {
            let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
            let tabs = Tab::ALL.into_iter().map(Tab::label).collect::<Vec<_>>();

            let header = metrics::text_width(TITLE, font_size)
                + style::row_gap(font_size)
                + style::icon_width(font_size);
            let tab_row = metrics::button_row_width(
                tabs,
                style::control_size(font_size),
                style::group_gap(font_size)
            );

            let page = match self.tab() {
                Tab::Appearance => appearance::desired_width(font_size),
                Tab::Modules => modules::desired_width(config, font_size, self.section())
            };

            header.max(tab_row).max(page) + metrics::ROW_SLACK_EM * font_size
        }

        /// Height the current page needs.
        ///
        /// Measured rather than guessed so the window can be capped to the
        /// screen and scroll the rest: a page taller than the screen
        /// would otherwise have its last rows cut off by the edge.
        /// The three window lengths, with the content walked exactly once.
        #[must_use]
        pub fn window_metrics(&self, config: &Config) -> crate::menu::MenuMetrics {
            let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);
            let width = self.content_width(config);

            crate::menu::MenuMetrics {
                width,
                page_width: width - metrics::ROW_SLACK_EM * font_size,
                height: self.content_height(config)
            }
        }

        #[must_use]
        pub fn content_height(&self, config: &Config) -> f32 {
            let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);

            let header = style::row_height(font_size);
            let tabs = style::row_height(font_size);
            let page = match self.tab() {
                Tab::Appearance => appearance::desired_height(
                    font_size,
                    config.appearance.auto_scale,
                    config.updates.is_some()
                ),
                Tab::Modules => modules::desired_height(config, font_size, self.section())
            };

            header + tabs + page + style::window_gap(font_size) * style::WINDOW_GAP_COUNT
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::modules::settings::{Section, Tab};

        #[test]
        fn every_page_derives_its_height_from_the_shared_row_pitch() {
            let font_size = 16.0;
            let config = Config::default();
            assert_eq!(
                appearance::desired_height(font_size, false, false),
                style::page_height(appearance::rows(false, false), font_size)
            );
            assert_eq!(
                modules::desired_height(&config, font_size, Section::Left),
                style::page_height(modules::rows(&config, Section::Left), font_size)
            );
        }

        #[test]
        fn every_page_reserves_the_shared_label_column() {
            let font_size = 16.0;
            let config = Config::default();
            let column = style::label_width(font_size);

            assert!(appearance::desired_width(font_size) > column);
            assert!(modules::desired_width(&config, font_size, Section::Left) > column);
        }

        #[test]
        fn a_larger_text_size_makes_every_page_taller() {
            let config = Config::default();

            assert!(
                appearance::desired_height(20.0, false, false)
                    > appearance::desired_height(16.0, false, false)
            );
            assert!(
                modules::desired_height(&config, 20.0, Section::Left)
                    > modules::desired_height(&config, 16.0, Section::Left)
            );
        }

        #[test]
        fn the_window_reserves_a_row_for_its_header_and_for_its_tabs() {
            let config = Config::default();
            let settings = Settings::default();
            let font_size = config.appearance.font_size.unwrap_or(DEFAULT_FONT_SIZE);

            assert!(
                settings.content_height(&config)
                    >= 2.0 * style::row_height(font_size)
                        + appearance::desired_height(
                            font_size,
                            config.appearance.auto_scale,
                            config.updates.is_some()
                        )
            );
        }

        #[test]
        fn a_page_draws_into_less_room_than_the_window_asks_for() {
            let config = Config::default();
            let settings = Settings::default();

            assert!(settings.window_metrics(&config).page_width < settings.content_width(&config));
        }

        #[test]
        fn the_header_fits_the_title_it_draws() {
            let font_size = 16.0;
            let config = Config::default();
            let settings = Settings::default();

            assert!(settings.content_width(&config) >= metrics::text_width(TITLE, font_size));
        }

        #[test]
        fn every_tab_asks_for_a_positive_size() {
            let config = Config::default();

            for tab in Tab::ALL {
                let settings = Settings {
                    tab,
                    ..Settings::default()
                };

                assert!(settings.content_width(&config) > 0.0);
                assert!(settings.content_height(&config) > 0.0);
            }
        }
    }
}
mod writer {
    //! Persisting a single setting back into the configuration file.
    //!
    //! The file belongs to the user: it carries their comments, their ordering
    //! and their formatting. Edits therefore go through a format preserving
    //! document instead of a serialise-the-whole-struct round trip, which
    //! would flatten the file into whatever the derive happens to emit.

    use std::{fmt, fs, io, path::Path};

    use toml_edit::{Array, DocumentMut, Item, Table, Value, value};

    /// Why a setting could not be written back.
    #[derive(Debug)]
    pub enum SettingsWriteError {
        /// The configuration file could not be read or written.
        Io(io::Error),
        /// The configuration file on disk is not valid TOML.
        Parse(toml_edit::TomlError),
        /// A key on the path to the setting is held by a non table value.
        NotATable {
            /// Dotted path of the offending key.
            path: String
        }
    }

    impl fmt::Display for SettingsWriteError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Io(err) => write!(f, "the configuration file could not be updated: {err}"),
                Self::Parse(err) => write!(f, "the configuration file is not valid TOML: {err}"),
                Self::NotATable {
                    path
                } => {
                    write!(f, "`{path}` is not a table and cannot hold a setting")
                }
            }
        }
    }

    impl std::error::Error for SettingsWriteError {}

    impl From<io::Error> for SettingsWriteError {
        fn from(err: io::Error) -> Self {
            Self::Io(err)
        }
    }

    impl From<toml_edit::TomlError> for SettingsWriteError {
        fn from(err: toml_edit::TomlError) -> Self {
            Self::Parse(err)
        }
    }

    /// Value a setting can be given.
    #[derive(Debug, Clone, PartialEq)]
    pub enum SettingValue {
        /// A named variant, written as a bare string.
        Text(String),
        /// A number, written as a float.
        Number(f64),
        /// A flag.
        Flag(bool),
        /// A list, whose entries may be lists themselves.
        List(Vec<SettingValue>)
    }

    impl SettingValue {
        /// Renders this value as a TOML value.
        fn into_toml(self) -> Value {
            match self {
                Self::Text(text) => Value::from(text),
                Self::Number(number) => Value::from(number),
                Self::Flag(flag) => Value::from(flag),
                Self::List(entries) => {
                    Value::Array(entries.into_iter().map(Self::into_toml).collect::<Array>())
                }
            }
        }
    }

    impl From<&str> for SettingValue {
        fn from(text: &str) -> Self {
            Self::Text(text.to_owned())
        }
    }

    impl From<f32> for SettingValue {
        fn from(number: f32) -> Self {
            Self::Number(f64::from(number))
        }
    }

    impl From<bool> for SettingValue {
        fn from(flag: bool) -> Self {
            Self::Flag(flag)
        }
    }

    /// Writes `value` at the dotted `path` of the configuration file at `file`.
    ///
    /// Missing intermediate tables are created, existing comments and ordering
    /// are kept. The write is atomic in the sense the bar cares about: the
    /// document is rendered in full and replaces the file in one call, so
    /// the watcher never observes a half written configuration.
    ///
    /// # Errors
    /// Returns [`SettingsWriteError`] when the file cannot be read or written,
    /// when its contents are not valid TOML, or when a key on `path` is
    /// occupied by a value that cannot hold a table.
    pub fn write_setting(
        file: &Path,
        path: &[&str],
        setting: SettingValue
    ) -> Result<(), SettingsWriteError> {
        write_settings(file, vec![(path, setting)])
    }

    /// Writes several settings in one pass: one read, one parse, one write.
    ///
    /// The configuration file is watched, and every write is a reload of the
    /// whole bar — settings that change together must land together.
    pub fn write_settings(
        file: &Path,
        settings: Vec<(&[&str], SettingValue)>
    ) -> Result<(), SettingsWriteError> {
        let source = match fs::read_to_string(file) {
            Ok(source) => source,
            Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
            Err(err) => return Err(err.into())
        };

        let mut document = source.parse::<DocumentMut>()?;

        for (path, setting) in settings {
            apply_setting(&mut document, path, setting)?;
        }

        fs::write(file, document.to_string())?;

        Ok(())
    }

    /// States one setting inside the parsed document.
    fn apply_setting(
        document: &mut DocumentMut,
        path: &[&str],
        setting: SettingValue
    ) -> Result<(), SettingsWriteError> {
        let Some((key, tables)) = path.split_last() else {
            return Ok(());
        };

        let mut item = document.as_item_mut();

        for (depth, table) in tables.iter().enumerate() {
            let entry = item
                .as_table_like_mut()
                .ok_or_else(|| SettingsWriteError::NotATable {
                    path: path[..depth].join(".")
                })?;

            if entry.get(table).is_none() {
                entry.insert(table, Item::Table(Table::new()));
            }

            item = entry
                .get_mut(table)
                .ok_or_else(|| SettingsWriteError::NotATable {
                    path: path[..=depth].join(".")
                })?;
        }

        let table = item
            .as_table_like_mut()
            .ok_or_else(|| SettingsWriteError::NotATable {
                path: tables.join(".")
            })?;

        let mut replacement = value(setting.into_toml());

        match table.get_mut(key) {
            Some(existing) => {
                if let (Some(previous), Some(fresh)) =
                    (existing.as_value(), replacement.as_value_mut())
                {
                    *fresh.decor_mut() = previous.decor().clone();
                }

                *existing = replacement;
            }
            None => {
                table.insert(key, replacement);
            }
        }

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn scratch(name: &str) -> std::path::PathBuf {
            let mut path = std::env::temp_dir();
            path.push(format!("hydebar-settings-writer-{name}.toml"));
            let _ = fs::remove_file(&path);
            path
        }

        #[test]
        fn a_top_level_setting_replaces_its_previous_value() {
            let file = scratch("top-level");
            fs::write(&file, "position = \"Top\"\n").expect("seed");

            write_setting(&file, &["position"], "Bottom".into()).expect("write");

            assert_eq!(
                fs::read_to_string(&file).expect("read"),
                "position = \"Bottom\"\n"
            );
        }

        #[test]
        fn comments_and_unrelated_keys_survive_a_write() {
            let file = scratch("comments");
            fs::write(
            &file,
            "# my bar\nposition = \"Top\"\n\n[appearance]\n# how tall\nheight = 38.0\nstyle = \"Islands\"\n"
        )
        .expect("seed");

            write_setting(&file, &["appearance", "height"], 42.0_f32.into()).expect("write");

            let written = fs::read_to_string(&file).expect("read");
            assert!(written.contains("# my bar"));
            assert!(written.contains("# how tall"));
            assert!(written.contains("height = 42.0"));
            assert!(written.contains("style = \"Islands\""));
        }

        #[test]
        fn a_missing_table_is_created_on_the_way() {
            let file = scratch("missing-table");
            fs::write(&file, "position = \"Top\"\n").expect("seed");

            write_setting(&file, &["appearance", "menu", "backdrop"], 0.5_f32.into())
                .expect("write");

            let written = fs::read_to_string(&file).expect("read");
            assert!(written.contains("[appearance.menu]"));
            assert!(written.contains("backdrop = 0.5"));
        }

        #[test]
        fn a_flag_is_written_as_a_boolean() {
            let file = scratch("flag");
            fs::write(&file, "").expect("seed");

            write_setting(&file, &["appearance", "follow_hyde"], false.into()).expect("write");

            assert!(
                fs::read_to_string(&file)
                    .expect("read")
                    .contains("follow_hyde = false")
            );
        }

        #[test]
        fn a_key_occupied_by_a_scalar_is_reported() {
            let file = scratch("occupied");
            fs::write(&file, "appearance = 3\n").expect("seed");

            let err = write_setting(&file, &["appearance", "height"], 38.0_f32.into())
                .expect_err("a scalar cannot hold a table");

            assert!(matches!(err, SettingsWriteError::NotATable { .. }));
        }

        #[test]
        fn a_nested_list_is_written_as_an_array_of_arrays() {
            let file = scratch("nested-list");
            fs::write(&file, "").expect("seed");

            write_setting(
                &file,
                &["modules", "left"],
                SettingValue::List(vec![
                    SettingValue::Text("Clock".to_owned()),
                    SettingValue::List(vec![
                        SettingValue::Text("Workspaces".to_owned()),
                        SettingValue::Text("WindowTitle".to_owned()),
                    ]),
                ])
            )
            .expect("write");

            let written = fs::read_to_string(&file).expect("read");
            assert!(written.contains("[modules]"));
            assert!(written.contains(r#"left = ["Clock", ["Workspaces", "WindowTitle"]]"#));
        }

        #[test]
        fn an_absent_file_is_created_from_scratch() {
            let file = scratch("absent");

            write_setting(&file, &["appearance", "height"], 38.0_f32.into()).expect("write");

            assert!(
                fs::read_to_string(&file)
                    .expect("read")
                    .contains("height = 38.0")
            );
        }
    }
}

use std::path::{Path, PathBuf};

use hydebar_proto::config::{Config, HydeBranch, ModuleDef, Modules, NotificationSource};
use iced::{Element, Task};
pub use layout::{LayoutEdit, Section, Slot};
use log::warn;
pub use tab::Tab;
pub use writer::{SettingValue, SettingsWriteError, write_setting};

use super::{Module, OnModulePress};
use crate::{
    components::icons::{IconTheme, Icons, icon},
    config::{AppearanceStyle, BarLayer, Position},
    menu::MenuType,
    services::hyprland_notify::{Notice, compositor_color, notify, post_to_bus}
};

/// How long the notice announcing a new notification source stays up, in
/// milliseconds.
const ANNOUNCE_DURATION: u32 = 4000;

/// Smallest bar height the menu will step down to, in pixels.
const MIN_HEIGHT: f32 = 16.0;
/// Largest bar height the menu will step up to, in pixels.
const MAX_HEIGHT: f32 = 96.0;
/// Height added or removed by one press, in pixels.
const HEIGHT_STEP: f32 = 2.0;

/// Smallest side padding the menu will step down to, in pixels.
///
/// Zero is a deliberate choice rather than a floor: a bar told to sit flush
/// with the screen edge is what a compositor without window gaps calls for.
const MIN_SIDE_PADDING: f32 = 0.0;
/// Largest side padding the menu will step up to, in pixels.
const MAX_SIDE_PADDING: f32 = 96.0;
/// Side padding added or removed by one press, in pixels.
const SIDE_PADDING_STEP: f32 = 1.0;

/// Smallest font size the menu will step down to, in pixels.
const MIN_FONT_SIZE: f32 = 6.0;
/// Largest font size the menu will step up to, in pixels.
const MAX_FONT_SIZE: f32 = 32.0;
/// Font size added or removed by one press, in pixels.
const FONT_SIZE_STEP: f32 = 1.0;

/// Opacity added or removed by one press.
const OPACITY_STEP: f32 = 0.05;

/// Choice made in the settings menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Move the bar to the given screen edge.
    SetPosition(Position),
    /// Place the bar on the given compositor layer.
    SetLayer(BarLayer),
    /// Draw the bar in the given style.
    SetStyle(AppearanceStyle),
    /// Set the bar height, in pixels.
    SetHeight(f32),
    /// Set the padding between the screen edge and the outermost island, in
    /// pixels.
    ///
    /// Writing it pins the padding: the bar stops taking the gap the
    /// compositor keeps around its windows.
    SetSidePadding(f32),
    /// Set the default text size, in pixels.
    SetFontSize(f32),
    /// Set the opacity of the module pills.
    SetOpacity(f32),
    /// Choose who draws the notification popups.
    SetNotificationSource(NotificationSource),
    /// Follow the given branch of the HyDE clone.
    SetHydeBranch(HydeBranch),
    /// Show another page of the window.
    SelectTab(Tab),
    /// Rearrange the modules of the bar.
    EditLayout(LayoutEdit),
    /// Pick the module the editor acts on, or drop the pick.
    SelectSlot(Option<Slot>),
    /// Show the modules of another section.
    SelectSection(Section)
}

/// Announces the notification source the user just picked, through that
/// very source.
///
/// A setting whose effect is invisible until something else happens is a
/// setting nobody can tell they changed. Sending one notice the moment the
/// choice is made answers the only question the choice raises — where will
/// my notifications appear now — by showing it.
pub fn announce_source(source: NotificationSource, config: &Config) {
    let message = format!("notifications are now shown by {}", source.label());

    if source.hands_to_compositor() {
        notify(
            Notice::Info,
            ANNOUNCE_DURATION,
            &compositor_color(config.appearance.primary_color),
            config.appearance.font_size_px(),
            &message
        );

        return;
    }

    post_to_bus(&message);
}

impl Message {
    /// Dotted path of the configuration key this choice writes.
    fn path(&self) -> &'static [&'static str] {
        match self {
            Self::SetPosition(_) => &["position"],
            Self::SetLayer(_) => &["layer"],
            Self::SetStyle(_) => &["appearance", "style"],
            Self::SetHeight(_) => &["appearance", "height"],
            Self::SetSidePadding(_) => &["appearance", "side_padding"],
            Self::SetFontSize(_) => &["appearance", "font_size"],
            Self::SetOpacity(_) => &["appearance", "opacity"],
            Self::SetNotificationSource(_) => &["notifications", "source"],
            Self::SetHydeBranch(_) => &["updates", "hyde_branch"],
            Self::SelectTab(_)
            | Self::EditLayout(_)
            | Self::SelectSlot(_)
            | Self::SelectSection(_) => &[]
        }
    }

    /// Value this choice writes at [`Self::path`].
    fn value(&self) -> SettingValue {
        match self {
            Self::SetPosition(position) => match position {
                Position::Top => "Top".into(),
                Position::Bottom => "Bottom".into()
            },
            Self::SetLayer(layer) => match layer {
                BarLayer::Background => "Background".into(),
                BarLayer::Bottom => "Bottom".into(),
                BarLayer::Top => "Top".into(),
                BarLayer::Overlay => "Overlay".into()
            },
            Self::SetStyle(style) => match style {
                AppearanceStyle::Islands => "Islands".into(),
                AppearanceStyle::Solid => "Solid".into(),
                AppearanceStyle::Gradient => "Gradient".into()
            },
            Self::SetHeight(height) => (*height).into(),
            Self::SetSidePadding(padding) => (*padding).into(),
            Self::SetFontSize(size) => (*size).into(),
            Self::SetOpacity(opacity) => (*opacity).into(),
            Self::SetNotificationSource(source) => match source {
                NotificationSource::Builtin => "Builtin".into(),
                NotificationSource::Compositor => "Compositor".into(),
                NotificationSource::Daemon => "Daemon".into()
            },
            Self::SetHydeBranch(branch) => match branch {
                HydeBranch::Master => "Master".into(),
                HydeBranch::Dev => "Dev".into()
            },
            Self::SelectTab(_)
            | Self::EditLayout(_)
            | Self::SelectSlot(_)
            | Self::SelectSection(_) => SettingValue::Flag(false)
        }
    }

    /// Writes this choice into the configuration file at `config_path`.
    ///
    /// Failures are logged rather than propagated: a settings menu that cannot
    /// persist a choice should still leave the bar running.
    fn apply(&self, config_path: &Path) {
        let path = self.path();

        if path.is_empty() {
            return;
        }

        if let Err(err) = write_setting(config_path, path, self.value()) {
            warn!("failed to store the setting: {err}");
        }
    }
}

/// Keeps the pick on the module the edit acted on.
///
/// A module that moved would otherwise leave the pick pointing at whatever
/// took its place, and the next button press would act on the wrong module.
fn follow(edit: LayoutEdit, modules: &Modules) -> Option<Slot> {
    let slot = edit.slot()?;

    match edit {
        LayoutEdit::Remove(_) => None,
        LayoutEdit::MoveEarlier(_) => Some(Slot {
            section: slot.section,
            index:   slot.index.saturating_sub(1)
        }),
        LayoutEdit::MoveLater(_) => Some(Slot {
            section: slot.section,
            index:   slot.index + 1
        }),
        LayoutEdit::MoveToPreviousSection(_) => slot.section.before().map(|section| Slot {
            section,
            index: section.entries(modules).len().saturating_sub(1)
        }),
        LayoutEdit::MoveToNextSection(_) => slot.section.after().map(|section| Slot {
            section,
            index: 0
        }),
        _ => Some(slot)
    }
}

/// Renders a bar entry as the value the configuration stores.
fn entry_value(entry: &ModuleDef) -> SettingValue {
    match entry {
        ModuleDef::Single(name) => SettingValue::Text(name.as_str().to_owned()),
        ModuleDef::Group(group) => SettingValue::List(
            group
                .iter()
                .map(|name| SettingValue::Text(name.as_str().to_owned()))
                .collect()
        )
    }
}

/// Renders a section as the list the configuration stores.
fn section_value(entries: &[ModuleDef]) -> SettingValue {
    SettingValue::List(entries.iter().map(entry_value).collect())
}

/// Writes every section of `modules` into the configuration file.
///
/// In one write on purpose: the file is watched, and three writes in a row
/// would reload the whole bar up to three times for one edit.
fn store_layout(config_path: &Path, modules: &Modules) {
    let settings = vec![
        (["modules", "left"].as_slice(), section_value(&modules.left)),
        (
            ["modules", "center"].as_slice(),
            section_value(&modules.center)
        ),
        (
            ["modules", "right"].as_slice(),
            section_value(&modules.right)
        ),
    ];

    if let Err(err) = writer::write_settings(config_path, settings) {
        warn!("failed to store the module layout: {err}");
    }
}

/// Bar entry opening the settings of the bar.
#[derive(Default, Debug, Clone)]
pub struct Settings {
    /// File the choices are written to.
    config_path: PathBuf,
    /// Page the window currently shows.
    tab:         Tab,
    /// Module the editor acts on, once one is picked.
    selected:    Option<Slot>,
    /// Section the editor is showing.
    section:     Section
}

impl Settings {
    /// Creates the module writing to `config_path`.
    #[must_use]
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            tab: Tab::default(),
            selected: None,
            section: Section::Left
        }
    }

    /// Section the editor is showing.
    #[must_use]
    pub fn section(&self) -> Section {
        self.section
    }

    /// Module the editor acts on, once one is picked.
    #[must_use]
    pub fn selected(&self) -> Option<Slot> {
        self.selected
    }

    /// Page the window currently shows.
    #[must_use]
    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// Applies a choice made in the window.
    ///
    /// Picking a tab is the only choice kept in memory; everything else lands
    /// in the configuration file and comes back through the reload.
    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::SelectTab(tab) => self.tab = tab,
            Message::SelectSlot(slot) => self.selected = slot,
            Message::SelectSection(section) => {
                self.section = section;
                self.selected = None;
            }
            Message::EditLayout(edit) => {
                let modules = layout::apply(&config.modules, &edit);
                store_layout(&self.config_path, &modules);
                self.selected = follow(edit, &modules);

                if let Some(slot) = self.selected {
                    self.section = slot.section;
                }
            }
            other => other.apply(&self.config_path)
        }

        Task::none()
    }

    /// File the choices are written to.
    #[must_use]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Height one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn height_below(current: f32) -> f32 {
        (current - HEIGHT_STEP).clamp(MIN_HEIGHT, MAX_HEIGHT)
    }

    /// Height one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn height_above(current: f32) -> f32 {
        (current + HEIGHT_STEP).clamp(MIN_HEIGHT, MAX_HEIGHT)
    }

    /// Side padding one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn side_padding_below(current: f32) -> f32 {
        (current - SIDE_PADDING_STEP).clamp(MIN_SIDE_PADDING, MAX_SIDE_PADDING)
    }

    /// Side padding one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn side_padding_above(current: f32) -> f32 {
        (current + SIDE_PADDING_STEP).clamp(MIN_SIDE_PADDING, MAX_SIDE_PADDING)
    }

    /// Font size one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn font_size_below(current: f32) -> f32 {
        (current - FONT_SIZE_STEP).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
    }

    /// Font size one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn font_size_above(current: f32) -> f32 {
        (current + FONT_SIZE_STEP).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
    }

    /// Opacity one step below `current`, clamped to the supported range.
    #[must_use]
    pub fn opacity_below(current: f32) -> f32 {
        ((current - OPACITY_STEP) * 100.0).round() / 100.0
    }

    /// Opacity one step above `current`, clamped to the supported range.
    #[must_use]
    pub fn opacity_above(current: f32) -> f32 {
        ((current + OPACITY_STEP) * 100.0).round() / 100.0
    }
}

impl<M> Module<M> for Settings
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a IconTheme;
    type RegistrationData<'a> = ();

    fn view(
        &self,
        icons: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        Some((
            icon(icons, Icons::Settings).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Settings))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_choice_names_the_key_it_writes() {
        assert_eq!(Message::SetPosition(Position::Bottom).path(), &["position"]);
        assert_eq!(Message::SetLayer(BarLayer::Top).path(), &["layer"]);
        assert_eq!(
            Message::SetStyle(AppearanceStyle::Solid).path(),
            &["appearance", "style"]
        );
        assert_eq!(Message::SetHeight(38.0).path(), &["appearance", "height"]);
        assert_eq!(
            Message::SetSidePadding(8.0).path(),
            &["appearance", "side_padding"]
        );
        assert_eq!(
            Message::SetFontSize(10.0).path(),
            &["appearance", "font_size"]
        );
        assert_eq!(Message::SetOpacity(0.8).path(), &["appearance", "opacity"]);
        assert_eq!(
            Message::SetNotificationSource(NotificationSource::Daemon).path(),
            &["notifications", "source"]
        );
    }

    #[test]
    fn named_variants_are_written_the_way_the_reader_spells_them() {
        assert_eq!(
            Message::SetPosition(Position::Bottom).value(),
            SettingValue::Text("Bottom".to_owned())
        );
        assert_eq!(
            Message::SetLayer(BarLayer::Overlay).value(),
            SettingValue::Text("Overlay".to_owned())
        );
        assert_eq!(
            Message::SetStyle(AppearanceStyle::Gradient).value(),
            SettingValue::Text("Gradient".to_owned())
        );
        assert_eq!(
            Message::SetNotificationSource(NotificationSource::Builtin).value(),
            SettingValue::Text("Builtin".to_owned())
        );
    }

    #[test]
    fn a_written_variant_reads_back_as_the_same_value() {
        let position: Position = toml::from_str("v = \"Bottom\"\n")
            .map(|w: Wrapper<Position>| w.v)
            .expect("position");
        assert_eq!(position, Position::Bottom);

        let style: AppearanceStyle = toml::from_str("v = \"Gradient\"\n")
            .map(|w: Wrapper<AppearanceStyle>| w.v)
            .expect("style");
        assert_eq!(style, AppearanceStyle::Gradient);

        let layer: BarLayer = toml::from_str("v = \"Overlay\"\n")
            .map(|w: Wrapper<BarLayer>| w.v)
            .expect("layer");
        assert_eq!(layer, BarLayer::Overlay);
    }

    #[derive(serde::Deserialize)]
    struct Wrapper<T> {
        v: T
    }

    #[test]
    fn the_height_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::height_above(38.0), 40.0);
        assert_eq!(Settings::height_below(38.0), 36.0);
        assert_eq!(Settings::height_below(MIN_HEIGHT), MIN_HEIGHT);
        assert_eq!(Settings::height_above(MAX_HEIGHT), MAX_HEIGHT);
    }

    #[test]
    fn the_side_padding_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::side_padding_above(8.0), 9.0);
        assert_eq!(Settings::side_padding_below(8.0), 7.0);
        assert_eq!(
            Settings::side_padding_below(MIN_SIDE_PADDING),
            MIN_SIDE_PADDING
        );
        assert_eq!(
            Settings::side_padding_above(MAX_SIDE_PADDING),
            MAX_SIDE_PADDING
        );
    }

    #[test]
    fn the_font_size_steps_stay_inside_the_supported_range() {
        assert_eq!(Settings::font_size_above(10.0), 11.0);
        assert_eq!(Settings::font_size_below(10.0), 9.0);
        assert_eq!(Settings::font_size_below(MIN_FONT_SIZE), MIN_FONT_SIZE);
        assert_eq!(Settings::font_size_above(MAX_FONT_SIZE), MAX_FONT_SIZE);
    }

    #[test]
    fn the_opacity_steps_keep_two_decimals() {
        assert_eq!(Settings::opacity_above(0.8), 0.85);
        assert_eq!(Settings::opacity_below(0.8), 0.75);
    }
}
