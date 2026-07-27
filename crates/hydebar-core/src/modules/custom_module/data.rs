//! Payload produced by a custom module listener process.

use serde::Deserialize;

/// One update emitted by a listener process as a single line of JSON.
///
/// The shape is a superset of the Waybar custom module return type, so scripts
/// written for Waybar work without modification.
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct CustomListenData {
    /// Alternate state name, matched against the configured icon and alert
    /// patterns.
    #[serde(default)]
    pub alt:        String,
    /// Text rendered next to the icon.
    #[serde(default)]
    pub text:       Option<String>,
    /// Text rendered when the pointer rests on the module.
    #[serde(default)]
    pub tooltip:    Option<String>,
    /// Style class requested by the listener.
    #[serde(default, deserialize_with = "first_class")]
    pub class:      Option<String>,
    /// Progress value in the zero to one hundred range.
    #[serde(default)]
    pub percentage: Option<f32>
}

/// Accepts both the single string and the list form Waybar allows for `class`,
/// keeping the first entry of a list.
fn first_class<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ClassField {
        One(String),
        Many(Vec<String>)
    }

    Ok(match Option::<ClassField>::deserialize(deserializer)? {
        Some(ClassField::One(value)) => Some(value),
        Some(ClassField::Many(values)) => values.into_iter().next(),
        None => None
    })
}
