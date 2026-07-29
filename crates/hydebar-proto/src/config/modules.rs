use std::fmt;

use serde::{Deserialize, Deserializer, de::Error as _};

/// Bar placement configuration.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    /// Render the bar at the top of the output.
    #[default]
    Top,
    /// Render the bar at the bottom of the output.
    Bottom
}

/// Compositor layer the bar surface is placed on.
///
/// Compositors composite the blur source from the background and bottom
/// levels, so a bar that should be blurred behind has to sit on [`Top`] or
/// above.
///
/// [`Top`]: BarLayer::Top
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarLayer {
    /// Behind every window, together with the wallpaper.
    Background,
    /// Behind windows but above the wallpaper.
    #[default]
    Bottom,
    /// Above windows, alongside most status bars.
    Top,
    /// Above everything, including fullscreen surfaces.
    Overlay
}

/// Named module variants supported by the bar.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModuleName {
    AppLauncher,
    Updates,
    Clipboard,
    Workspaces,
    WindowTitle,
    SystemInfo,
    KeyboardLayout,
    KeyboardSubmap,
    Tray,
    Clock,
    Battery,
    Privacy,
    ControlCenter,
    Audio,
    Network,
    Bluetooth,
    PowerProfile,
    Settings,
    Themes,
    Wallpaper,
    MediaPlayer,
    Notifications,
    Screenshot,
    IdleInhibitor,
    Custom(String)
}

impl ModuleName {
    /// Every module the bar ships, in the order the editor lists them.
    pub const BUILT_IN: [ModuleName; 24] = [
        ModuleName::AppLauncher,
        ModuleName::Updates,
        ModuleName::Clipboard,
        ModuleName::Workspaces,
        ModuleName::WindowTitle,
        ModuleName::SystemInfo,
        ModuleName::KeyboardLayout,
        ModuleName::KeyboardSubmap,
        ModuleName::Tray,
        ModuleName::Clock,
        ModuleName::Battery,
        ModuleName::Privacy,
        ModuleName::ControlCenter,
        ModuleName::Audio,
        ModuleName::Network,
        ModuleName::Bluetooth,
        ModuleName::PowerProfile,
        ModuleName::Settings,
        ModuleName::Themes,
        ModuleName::Wallpaper,
        ModuleName::MediaPlayer,
        ModuleName::Notifications,
        ModuleName::Screenshot,
        ModuleName::IdleInhibitor
    ];

    /// Name this module is written as in the configuration.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            ModuleName::AppLauncher => "AppLauncher",
            ModuleName::Updates => "Updates",
            ModuleName::Clipboard => "Clipboard",
            ModuleName::Workspaces => "Workspaces",
            ModuleName::WindowTitle => "WindowTitle",
            ModuleName::SystemInfo => "SystemInfo",
            ModuleName::KeyboardLayout => "KeyboardLayout",
            ModuleName::KeyboardSubmap => "KeyboardSubmap",
            ModuleName::Tray => "Tray",
            ModuleName::Clock => "Clock",
            ModuleName::Battery => "Battery",
            ModuleName::Privacy => "Privacy",
            ModuleName::ControlCenter => "ControlCenter",
            ModuleName::Audio => "Audio",
            ModuleName::Network => "Network",
            ModuleName::Bluetooth => "Bluetooth",
            ModuleName::PowerProfile => "PowerProfile",
            ModuleName::Settings => "Settings",
            ModuleName::Themes => "Themes",
            ModuleName::Wallpaper => "Wallpaper",
            ModuleName::MediaPlayer => "MediaPlayer",
            ModuleName::Notifications => "Notifications",
            ModuleName::Screenshot => "Screenshot",
            ModuleName::IdleInhibitor => "IdleInhibitor",
            ModuleName::Custom(name) => name.as_str()
        }
    }
}

impl<'de> Deserialize<'de> for ModuleName {
    fn deserialize<D>(deserializer: D) -> Result<ModuleName, D::Error>
    where
        D: Deserializer<'de>
    {
        struct ModuleNameVisitor;

        impl<'de> serde::de::Visitor<'de> for ModuleNameVisitor {
            type Value = ModuleName;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing a ModuleName")
            }

            fn visit_str<E>(self, value: &str) -> Result<ModuleName, E>
            where
                E: serde::de::Error
            {
                Ok(match value {
                    "AppLauncher" => ModuleName::AppLauncher,
                    "Updates" => ModuleName::Updates,
                    "Clipboard" => ModuleName::Clipboard,
                    "Workspaces" => ModuleName::Workspaces,
                    "WindowTitle" => ModuleName::WindowTitle,
                    "SystemInfo" => ModuleName::SystemInfo,
                    "KeyboardLayout" => ModuleName::KeyboardLayout,
                    "KeyboardSubmap" => ModuleName::KeyboardSubmap,
                    "Tray" => ModuleName::Tray,
                    "Clock" => ModuleName::Clock,
                    "Battery" => ModuleName::Battery,
                    "Privacy" => ModuleName::Privacy,
                    "ControlCenter" => ModuleName::ControlCenter,
                    "Audio" => ModuleName::Audio,
                    "Network" => ModuleName::Network,
                    "Bluetooth" => ModuleName::Bluetooth,
                    "PowerProfile" => ModuleName::PowerProfile,
                    "Settings" => ModuleName::Settings,
                    "Themes" => ModuleName::Themes,
                    "MediaPlayer" => ModuleName::MediaPlayer,
                    "Notifications" => ModuleName::Notifications,
                    "Screenshot" => ModuleName::Screenshot,
                    "IdleInhibitor" => ModuleName::IdleInhibitor,
                    other => ModuleName::Custom(other.to_string())
                })
            }
        }

        deserializer.deserialize_str(ModuleNameVisitor)
    }
}

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
    pub fn hosts(&self, name: &ModuleName) -> bool {
        self.placed().any(|placed| placed == name)
    }

    /// Reports whether any of `names` is drawn in the bar.
    ///
    /// Several bar entries can share one background worker: the control centre
    /// services feed the `Audio`, `Network`, `Bluetooth` and `PowerProfile`
    /// readouts alike, so the worker has to stay alive while at least one of
    /// them is on screen.
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

/// Output targeting configuration for module rendering.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum Outputs {
    /// Render on all outputs.
    #[default]
    All,
    /// Render on the currently focused output.
    Active,
    /// Render on the explicitly configured output list.
    #[serde(deserialize_with = "non_empty")]
    Targets(Vec<String>)
}

fn non_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>
{
    let values = <Vec<T>>::deserialize(deserializer)?;

    if values.is_empty() {
        Err(D::Error::custom("need non-empty"))
    } else {
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use serde::de::value::{Error as DeError, SeqDeserializer, StrDeserializer};

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

    #[test]
    fn non_empty_rejects_empty_vectors() {
        let error: DeError = non_empty::<_, String>(SeqDeserializer::<_, DeError>::new(
            Vec::<String>::new().into_iter()
        ))
        .expect_err("empty list should fail");
        assert!(error.to_string().contains("non-empty"));
    }

    #[test]
    fn module_name_deserializes_idle_inhibitor() {
        let name = ModuleName::deserialize(StrDeserializer::<DeError>::new("IdleInhibitor"))
            .expect("known variant");
        assert_eq!(name, ModuleName::IdleInhibitor);
    }

    #[test]
    fn module_name_deserializes_custom_values() {
        let name = ModuleName::deserialize(StrDeserializer::<DeError>::new("MyCustom"))
            .expect("custom variant");
        assert!(matches!(name, ModuleName::Custom(value) if value == "MyCustom"));
    }

    /// A module the editor cannot offer is a module nobody can place from the
    /// window, whatever else it can do.
    #[test]
    fn the_theme_module_is_one_the_layout_editor_offers() {
        assert!(ModuleName::BUILT_IN.contains(&ModuleName::Themes));
    }

    #[test]
    fn the_theme_module_reads_back_as_it_is_written() {
        let name: ModuleName =
            Deserialize::deserialize(StrDeserializer::<DeError>::new("Themes")).expect("name");

        assert_eq!(name, ModuleName::Themes);
        assert_eq!(name.as_str(), "Themes");
    }

    /// The user's own `[[CustomModule]] name = "theme"` must keep being a
    /// custom module: the built in one answers to `Themes`, and nothing else.
    #[test]
    fn a_lowercase_theme_stays_the_custom_module_of_that_name() {
        let name: ModuleName =
            Deserialize::deserialize(StrDeserializer::<DeError>::new("theme")).expect("name");

        assert_eq!(name, ModuleName::Custom("theme".to_owned()));
    }
}
