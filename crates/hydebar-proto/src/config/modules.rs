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
    MediaPlayer,
    Notifications,
    Screenshot,
    IdleInhibitor,
    Custom(String)
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
}
