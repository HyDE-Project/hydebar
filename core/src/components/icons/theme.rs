//! Glyph table assembled from the configuration overrides.

use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex, PoisonError}
};

use hydebar_proto::config::IconsConfig;
use log::warn;

use super::catalog::Icons;

/// Every glyph string handed to the renderer so far.
///
/// The renderer demands `'static` glyphs, and each one is leaked to get
/// there; the set is what makes the leak once per distinct glyph instead of
/// once per reload — a theme switch reloads in a burst, and a bar left
/// running would otherwise grow by its overrides on every one of them.
static GLYPHS: LazyLock<Mutex<HashSet<&'static str>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Glyph table handed to the rendering helpers.
///
/// Holds only the icons the user redefined; everything else falls back to
/// [`Icons::default_glyph`], so the default table costs nothing.
///
/// The table also carries the size icons render at. Text without an explicit
/// size falls back to the size the renderer was started with, which never
/// changes: carrying the size here is what lets a reload resize the glyphs
/// along with the rest of the bar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IconTheme {
    overrides: HashMap<Icons, &'static str>,
    size:      Option<f32>
}

impl IconTheme {
    /// Builds the table from the `[icons]` configuration section.
    ///
    /// Unknown names are reported and skipped so a typo cannot take the bar
    /// down.
    #[must_use]
    pub fn from_config(config: &IconsConfig) -> Self {
        let mut overrides = HashMap::new();

        for (name, glyph) in config.iter() {
            match Icons::from_name(name) {
                Some(icon) => {
                    overrides.insert(icon, leaked(glyph));
                }
                None => warn!("Unknown icon override `{name}`, ignoring it")
            }
        }

        Self {
            overrides,
            size: None
        }
    }

    /// Returns the table rendering at `size` pixels.
    #[must_use]
    pub const fn with_size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Size glyphs render at, when the table carries one.
    #[must_use]
    pub const fn size(&self) -> Option<f32> {
        self.size
    }

    /// Glyph to render for `icon`, honouring any configured override.
    ///
    /// The reference lives forever: the defaults are compiled in and the
    /// overrides are leaked once per reload, so every icon of every frame is
    /// drawn without allocating a copy of its glyph.
    #[must_use]
    pub fn glyph(&self, icon: Icons) -> &'static str {
        self.overrides
            .get(&icon)
            .copied()
            .unwrap_or_else(|| icon.default_glyph())
    }

    /// Declares an override for `icon`, replacing any previous value.
    pub fn set(&mut self, icon: Icons, glyph: impl Into<String>) {
        self.overrides.insert(icon, leaked(&glyph.into()));
    }

    /// Returns `true` when every icon resolves to its built in glyph.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.overrides.is_empty()
    }
}

impl From<&IconsConfig> for IconTheme {
    fn from(config: &IconsConfig) -> Self {
        Self::from_config(config)
    }
}

/// A glyph made immortal for the renderer.
///
/// The same bargain the font family cache strikes: a handful of bytes per
/// distinct glyph, in exchange for never allocating on the draw path. A glyph
/// seen before is answered from the set, so reloading the same configuration
/// a thousand times costs what loading it once did.
fn leaked(glyph: &str) -> &'static str {
    let mut glyphs = GLYPHS.lock().unwrap_or_else(PoisonError::into_inner);

    if let Some(&known) = glyphs.get(glyph) {
        return known;
    }

    let immortal: &'static str = Box::leak(glyph.to_owned().into_boxed_str());
    glyphs.insert(immortal);

    immortal
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_table_carries_no_size() {
        assert_eq!(IconTheme::default().size(), None);
    }

    #[test]
    fn the_size_travels_with_the_table() {
        let theme = IconTheme::default().with_size(13.0);

        assert_eq!(theme.size(), Some(13.0));
        assert!(theme.is_default());
    }
}
