use smithay_client_toolkit::shell::wlr_layer;
use std::fmt;
use std::ops::BitOr;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SURFACE_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a layer shell surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(u64);

impl SurfaceId {
    /// The main surface, created via `.layer_shell()` on the application builder.
    pub const MAIN: Self = Self(0);

    /// Generate a new unique surface ID.
    pub fn unique() -> Self {
        Self(NEXT_SURFACE_ID.fetch_add(1, Ordering::Relaxed))
    }

    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

impl From<SurfaceId> for iced_core::window::Id {
    fn from(id: SurfaceId) -> Self {
        // SAFETY: Both SurfaceId and iced_core::window::Id are newtype wrappers
        // around u64 with no other fields, so transmute is safe.
        unsafe { std::mem::transmute(id.0) }
    }
}

impl fmt::Display for SurfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SurfaceId({})", self.0)
    }
}

/// Edge anchoring for a layer surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Anchor(u32);

impl Anchor {
    pub const NONE: Self = Self(0);
    pub const TOP: Self = Self(1);
    pub const BOTTOM: Self = Self(2);
    pub const LEFT: Self = Self(4);
    pub const RIGHT: Self = Self(8);

    #[must_use]
    pub fn all() -> Self {
        Self(Self::TOP.0 | Self::BOTTOM.0 | Self::LEFT.0 | Self::RIGHT.0)
    }

    #[must_use]
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub(crate) fn to_sctk(self) -> wlr_layer::Anchor {
        let mut a = wlr_layer::Anchor::empty();
        if self.contains(Self::TOP) {
            a |= wlr_layer::Anchor::TOP;
        }
        if self.contains(Self::BOTTOM) {
            a |= wlr_layer::Anchor::BOTTOM;
        }
        if self.contains(Self::LEFT) {
            a |= wlr_layer::Anchor::LEFT;
        }
        if self.contains(Self::RIGHT) {
            a |= wlr_layer::Anchor::RIGHT;
        }
        a
    }
}

impl BitOr for Anchor {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self::NONE
    }
}

/// Layer on which a surface is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Layer {
    Background,
    Bottom,
    #[default]
    Top,
    Overlay,
}

impl Layer {
    pub(crate) fn to_sctk(self) -> wlr_layer::Layer {
        match self {
            Self::Background => wlr_layer::Layer::Background,
            Self::Bottom => wlr_layer::Layer::Bottom,
            Self::Top => wlr_layer::Layer::Top,
            Self::Overlay => wlr_layer::Layer::Overlay,
        }
    }
}

/// Controls whether a surface receives keyboard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeyboardInteractivity {
    #[default]
    None,
    Exclusive,
    OnDemand,
}

impl KeyboardInteractivity {
    pub(crate) fn to_sctk(self) -> wlr_layer::KeyboardInteractivity {
        match self {
            Self::None => wlr_layer::KeyboardInteractivity::None,
            Self::Exclusive => wlr_layer::KeyboardInteractivity::Exclusive,
            Self::OnDemand => wlr_layer::KeyboardInteractivity::OnDemand,
        }
    }
}

/// Unique identifier for a Wayland output (monitor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(pub(crate) u32);

impl OutputId {
    /// A distinct identifier for state-space tests that never reach a real
    /// Wayland connection.
    ///
    /// Every call hands back an id no other surface or output claims, so
    /// test outputs cannot be confused with one another; only tests are
    /// meant to name an output that was never reported by a compositor.
    pub fn for_tests() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};

        // one more than the highest identifier for_tests has handed out so
        // far, from a range no real compositor reports
        static NEXT: AtomicU32 = AtomicU32::new(u32::MAX - 4096);

        OutputId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for OutputId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OutputId({})", self.0)
    }
}

/// Information about a connected output (monitor).
#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub id: OutputId,
    pub name: String,
    pub scale_factor: i32,
    pub logical_size: Option<(i32, i32)>,
    pub make: String,
    pub model: String,
}

/// Events related to output (monitor) changes.
#[derive(Debug, Clone)]
pub enum OutputEvent {
    Added(OutputInfo),
    Removed(OutputId),
    InfoChanged(OutputInfo),
    /// A layer surface has been mapped on (or moved to) the given output. For
    /// surfaces created with `output: None`, this reveals which output the
    /// compositor picked. May fire more than once if the surface intersects
    /// multiple outputs.
    SurfaceEnteredOutput {
        surface: SurfaceId,
        output: OutputId,
    },
    /// A layer surface left the given output (typically before destruction or
    /// when the compositor reassigns it).
    SurfaceLeftOutput {
        surface: SurfaceId,
        output: OutputId,
    },
}

/// Configuration for a layer shell surface.
#[derive(Debug, Clone)]
pub struct LayerShellSettings {
    pub anchor: Anchor,
    pub layer: Layer,
    pub exclusive_zone: i32,
    pub keyboard_interactivity: KeyboardInteractivity,
    /// Width and height. `None` for a dimension means the compositor decides
    /// (typically full extent for anchored edges).
    pub size: Option<(u32, u32)>,
    pub margin: (i32, i32, i32, i32),
    pub namespace: String,
    /// Target a specific output. `None` lets the compositor choose.
    pub output: Option<OutputId>,
    /// Pointer/touch input regions, stated before the surface's first commit.
    ///
    /// A region is surface state like the anchor or the margin: set before
    /// the first commit it ships with the surface and can never be lost to a
    /// timing race. `None` leaves the region unset (the whole surface takes
    /// input); `Some` lists the rectangles, an empty list making the surface
    /// take none.
    pub input_region: Option<Vec<crate::task::InputRegionRect>>,
}

impl Default for LayerShellSettings {
    fn default() -> Self {
        Self {
            anchor: Anchor::NONE,
            layer: Layer::Top,
            exclusive_zone: 0,
            keyboard_interactivity: KeyboardInteractivity::None,
            size: None,
            margin: (0, 0, 0, 0),
            namespace: String::from("iced_layer"),
            output: None,
            input_region: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_id_main_is_zero() {
        assert_eq!(SurfaceId::MAIN, SurfaceId::new(0));
    }

    #[test]
    fn surface_id_unique_increments() {
        let a = SurfaceId::unique();
        let b = SurfaceId::unique();
        assert_ne!(a, b);
    }

    #[test]
    fn surface_id_display() {
        assert_eq!(format!("{}", SurfaceId::new(42)), "SurfaceId(42)");
    }

    #[test]
    fn surface_id_equality_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SurfaceId::new(1));
        set.insert(SurfaceId::new(1));
        assert_eq!(set.len(), 1);
        set.insert(SurfaceId::new(2));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn anchor_bit_values() {
        assert_eq!(Anchor::NONE, Anchor(0));
        assert_eq!(Anchor::TOP, Anchor(1));
        assert_eq!(Anchor::BOTTOM, Anchor(2));
        assert_eq!(Anchor::LEFT, Anchor(4));
        assert_eq!(Anchor::RIGHT, Anchor(8));
    }

    #[test]
    fn anchor_all_contains_every_edge() {
        let all = Anchor::all();
        assert!(all.contains(Anchor::TOP));
        assert!(all.contains(Anchor::BOTTOM));
        assert!(all.contains(Anchor::LEFT));
        assert!(all.contains(Anchor::RIGHT));
        assert_eq!(
            all,
            Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT
        );
    }

    #[test]
    fn anchor_contains_subset() {
        let tb = Anchor::TOP | Anchor::BOTTOM;
        assert!(tb.contains(Anchor::TOP));
        assert!(tb.contains(Anchor::BOTTOM));
        assert!(!tb.contains(Anchor::LEFT));
        assert!(tb.contains(Anchor::NONE));
    }

    #[test]
    fn anchor_default_is_none() {
        assert_eq!(Anchor::default(), Anchor::NONE);
    }

    #[test]
    fn layer_default_is_top() {
        assert_eq!(Layer::default(), Layer::Top);
    }

    #[test]
    fn layer_variants_distinct() {
        assert_ne!(Layer::Background, Layer::Bottom);
        assert_ne!(Layer::Bottom, Layer::Top);
        assert_ne!(Layer::Top, Layer::Overlay);
    }

    #[test]
    fn keyboard_interactivity_default_is_none() {
        assert_eq!(
            KeyboardInteractivity::default(),
            KeyboardInteractivity::None
        );
    }

    #[test]
    fn output_id_display() {
        assert_eq!(format!("{}", OutputId(0)), "OutputId(0)");
        assert_eq!(format!("{}", OutputId(42)), "OutputId(42)");
    }

    #[test]
    fn layer_shell_settings_default() {
        let s = LayerShellSettings::default();
        assert_eq!(s.anchor, Anchor::NONE);
        assert_eq!(s.layer, Layer::Top);
        assert_eq!(s.exclusive_zone, 0);
        assert_eq!(s.keyboard_interactivity, KeyboardInteractivity::None);
        assert_eq!(s.size, None);
        assert_eq!(s.margin, (0, 0, 0, 0));
        assert_eq!(s.namespace, "iced_layer");
        assert!(s.output.is_none());
    }
}
