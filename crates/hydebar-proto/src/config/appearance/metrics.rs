//! Layout metrics of the bar, stated as factors of the themed font size, and
//! the [`Appearance`] accessors resolving them into pixels.

use super::settings::Appearance;

/// Corner radius used when neither the configuration nor the `HyDE` theme names
/// one, in pixels.
///
/// Mirrors the `3pt` border radius of the reference waybar theme, which
/// resolves to `4px` at the default `96dpi` scale.
pub const DEFAULT_RADIUS: f32 = 4.0;

/// Text size used when neither the configuration nor the `HyDE` theme names
/// one, in pixels.
///
/// Mirrors the renderer default so a bar without a theme keeps the proportions
/// it renders text at.
pub const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Padding a module label reserves on each side along the bar, in `em`.
///
/// The reference waybar theme pads a bar label by `0.2em` and margins it by
/// another `0.2em`, so neighbouring modules keep `0.4em` of breathing room that
/// scales with the themed font instead of being fixed in pixels.
pub const MODULE_SIDE_PADDING_EM: f32 = 0.25;

/// Padding a module label reserves above and below its text, in `em`.
///
/// Matches the `0.2em` vertical label padding of the reference waybar theme.
pub const MODULE_VERTICAL_PADDING_EM: f32 = 0.2;

/// Gap between a module icon and the label rendered next to it, in `em`.
pub const ICON_LABEL_GAP_EM: f32 = 0.4;

/// Gap between neighbouring modules of the same bar section, in `em`.
pub const MODULE_GAP_EM: f32 = 0.4;

/// Inner padding an island keeps around its modules, in `em`.
///
/// Mirrors the `0em 1em` padding the reference theme gives every island.
pub const ISLAND_PADDING_EM: [f32; 2] = [0.0, 1.0];

/// Gap between neighbouring islands of the same bar section, in `em`.
///
/// The reference theme gives every island a `1em` margin on both sides, so
/// two adjacent islands are separated by twice that.
pub const ISLAND_GAP_EM: f32 = 2.0;

/// Gap between the left, center and right groups of the bar, in `em`.
pub const GROUP_GAP_EM: f32 = 2.0;

/// Outer margin kept around the island pills, in `em`, as
/// `[vertical, horizontal]`.
///
/// Mirrors the `0.3em 1em` island margin of the reference waybar theme plus
/// the `0.3em` margin its module sections add on the outer edges.
pub const BAR_PADDING_EM: [f32; 2] = [0.3, 1.6];

/// Horizontal padding of an idle workspace indicator, in `em`.
///
/// Mirrors the `0.3em` side padding every `#workspaces button` of the
/// reference waybar theme reserves around its label.
pub const WORKSPACE_PADDING_EM: f32 = 0.3;

/// Horizontal padding of the focused workspace indicator, in `em`.
///
/// The reference waybar theme widens the focused button to `1.2em` of side
/// padding, which is what turns it into a rounded rectangle instead of a
/// label-sized square.
pub const WORKSPACE_ACTIVE_PADDING_EM: f32 = 1.2;

/// Margin kept on both sides of the focused workspace indicator, in `em`.
///
/// Matches the `0.3em` horizontal margin the reference waybar theme adds to
/// the focused button so it never touches its neighbours.
pub const WORKSPACE_ACTIVE_MARGIN_EM: f32 = 0.3;

/// Gap between neighbouring workspace indicators, in `em`.
///
/// The reference waybar theme leaves `#workspaces button` without a horizontal
/// margin, so two idle indicators sit flush against each other and only the
/// focused one pushes its neighbours away through
/// [`WORKSPACE_ACTIVE_MARGIN_EM`].
pub const WORKSPACE_GAP_EM: f32 = 0.0;

/// Minimum width the label box of a workspace indicator reserves, in `em`.
///
/// A `#workspaces button` never shrinks below the `16px` minimum width the GTK
/// stylesheet behind waybar reserves for every button, so a single glyph label
/// still occupies a fixed box instead of collapsing onto its glyph. Against the
/// `10px` font the reference theme ships that is `1.6em`, and it is what keeps
/// the indicators as far apart as waybar draws them.
pub const WORKSPACE_MIN_WIDTH_EM: f32 = 1.6;

/// Minimum height a workspace indicator reserves, in `em`.
///
/// The same GTK minimum keeps a `#workspaces button` `24px` tall whatever its
/// label measures, which is what gives the focused indicator its rounded
/// rectangle instead of a box tight around the glyph. Against the `10px` font
/// the reference theme ships that is `2.4em`.
pub const WORKSPACE_MIN_HEIGHT_EM: f32 = 2.4;

/// Advance width of a single workspace label glyph, in `em`.
///
/// Only used to tell whether a label already fills [`WORKSPACE_MIN_WIDTH_EM`]
/// on its own; the monospace font the reference theme ships advances by
/// `0.6em` per glyph.
pub const WORKSPACE_GLYPH_ADVANCE_EM: f32 = 0.6;

impl Appearance {
    /// Returns the pill corner radius to render with.
    #[must_use]
    pub fn pill_radius(&self) -> f32 {
        self.radius.unwrap_or(DEFAULT_RADIUS)
    }

    /// Multiplies every size of this appearance by `factor`.
    ///
    /// Applied before the renderer starts, so the text size it adopts as its
    /// default is already the magnified one: every label of the bar grows with
    /// the screen without having to state a size of its own.
    /// The side padding follows: a padding the user wrote is stated in the
    /// same pixels as the height beside it, so it has to grow with the bar or
    /// a magnified bar would hug the screen edge.
    pub fn magnify(&mut self, factor: f32) {
        self.font_size = Some(self.font_size_px() * factor);
        self.height = self.height.map(|height| height * factor);
        self.radius = self.radius.map(|radius| radius * factor);
        self.side_padding = self.side_padding.map(|padding| padding * factor);
    }

    /// Returns the text size the bar renders with, in pixels.
    ///
    /// Falls back to [`DEFAULT_FONT_SIZE`] whenever neither the configuration
    /// nor the `HyDE` theme names one, which is exactly the size the renderer
    /// would pick on its own.
    #[must_use]
    pub fn font_size_px(&self) -> f32 {
        self.font_size.unwrap_or(DEFAULT_FONT_SIZE)
    }

    /// Converts a spacing expressed in `em` into pixels of the themed font.
    ///
    /// Every gap of the bar row is stated as a factor of the text size, so a
    /// theme raising or lowering its font moves the whole layout with it.
    #[must_use]
    pub fn spacing(&self, em: f32) -> f32 {
        self.font_size_px() * em
    }

    /// Returns the padding of a single module, as `[vertical, horizontal]`
    /// pixels.
    #[must_use]
    pub fn module_padding(&self) -> [f32; 2] {
        [
            self.spacing(MODULE_VERTICAL_PADDING_EM),
            self.spacing(MODULE_SIDE_PADDING_EM)
        ]
    }

    /// Returns the outer margin around the island pills, as
    /// `[vertical, horizontal]` pixels.
    ///
    /// The horizontal half is [`Appearance::side_padding`] whenever it is
    /// known, which is what lines the outermost islands up with the windows
    /// below the bar; without it the margin falls back to the font-derived
    /// [`BAR_PADDING_EM`], the only measure a bar that never reached the
    /// compositor has.
    #[must_use]
    pub fn bar_padding(&self) -> [f32; 2] {
        [
            self.spacing(BAR_PADDING_EM[0]),
            self.side_padding
                .unwrap_or_else(|| self.spacing(BAR_PADDING_EM[1]))
        ]
    }

    /// Returns the gap between a module icon and its label, in pixels.
    #[must_use]
    pub fn icon_label_gap(&self) -> f32 {
        self.spacing(ICON_LABEL_GAP_EM)
    }

    /// Returns the gap between neighbouring modules, in pixels.
    #[must_use]
    pub fn module_gap(&self) -> f32 {
        self.spacing(MODULE_GAP_EM)
    }

    /// Returns the inner padding of an island, in pixels.
    #[must_use]
    pub fn island_padding(&self) -> [f32; 2] {
        [
            self.spacing(ISLAND_PADDING_EM[0]),
            self.spacing(ISLAND_PADDING_EM[1])
        ]
    }

    /// Returns the gap between neighbouring islands of a section, in pixels.
    #[must_use]
    pub fn island_gap(&self) -> f32 {
        self.spacing(ISLAND_GAP_EM)
    }

    /// Returns the gap between the groups of the bar, in pixels.
    #[must_use]
    pub fn group_gap(&self) -> f32 {
        self.spacing(GROUP_GAP_EM)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    #[test]
    fn workspace_spacing_matches_the_reference_theme() {
        let themed = Appearance {
            font_size: Some(10.0),
            ..Appearance::default()
        };

        assert_eq!(themed.spacing(WORKSPACE_PADDING_EM), 3.0);
        assert_eq!(themed.spacing(WORKSPACE_ACTIVE_PADDING_EM), 12.0);
        assert_eq!(themed.spacing(WORKSPACE_ACTIVE_MARGIN_EM), 3.0);
        assert_eq!(themed.spacing(WORKSPACE_GAP_EM), 0.0);
        assert_eq!(themed.spacing(WORKSPACE_MIN_WIDTH_EM), 16.0);
        assert_eq!(themed.spacing(WORKSPACE_MIN_HEIGHT_EM), 24.0);
    }

    #[test]
    fn font_size_px_falls_back_to_the_renderer_default() {
        assert_eq!(Appearance::default().font_size_px(), DEFAULT_FONT_SIZE);

        let themed = Appearance {
            font_size: Some(10.0),
            ..Appearance::default()
        };
        assert_eq!(themed.font_size_px(), 10.0);
    }

    #[test]
    fn spacing_scales_an_em_factor_by_the_font_size() {
        let fallback = Appearance::default();
        assert_eq!(fallback.spacing(0.0), 0.0);
        assert_eq!(fallback.spacing(1.0), DEFAULT_FONT_SIZE);
        assert_eq!(fallback.spacing(0.5), DEFAULT_FONT_SIZE * 0.5);

        let themed = Appearance {
            font_size: Some(10.0),
            ..Appearance::default()
        };
        assert_eq!(themed.spacing(0.4), 4.0);
        assert_eq!(themed.spacing(0.2), 2.0);
    }

    #[test]
    fn derived_spacing_matches_the_reference_theme() {
        let themed = Appearance {
            font_size: Some(10.0),
            ..Appearance::default()
        };

        assert_eq!(themed.module_padding(), [2.0, 2.5]);
        assert_eq!(themed.bar_padding(), [3.0, 16.0]);
        assert_eq!(themed.module_gap(), 4.0);
        assert_eq!(themed.group_gap(), 20.0);
        assert_eq!(themed.icon_label_gap(), 4.0);
    }

    #[test]
    fn a_known_side_padding_replaces_the_font_derived_one() {
        let aligned = Appearance {
            font_size: Some(10.0),
            side_padding: Some(6.0),
            ..Appearance::default()
        };

        assert_eq!(aligned.bar_padding(), [3.0, 6.0]);
    }

    #[test]
    fn magnifying_grows_the_side_padding_with_the_bar() {
        let mut appearance = Appearance {
            side_padding: Some(6.0),
            ..Appearance::default()
        };

        appearance.magnify(2.0);

        assert_eq!(appearance.side_padding, Some(12.0));
    }

    #[test]
    fn magnifying_leaves_an_unknown_side_padding_unknown() {
        let mut appearance = Appearance::default();

        appearance.magnify(2.0);

        assert_eq!(appearance.side_padding, None);
    }
}
