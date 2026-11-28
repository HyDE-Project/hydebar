# Professional UI Architecture and Implementation Plan

## Status

- Document type: normative implementation specification
- Project: Hydebar
- Last reviewed: 2026-09-08
- Immediate scope: visual architecture, frame correctness, interaction consistency, motion,
  configuration, accessibility, and visual verification
- Deferred scope: public third-party module SDK

This document defines the required path from the current Hydebar interface to a professional,
configurable HyDE-native shell. It is intentionally prescriptive. An implementation is not
complete merely because it looks correct in one screenshot. It must satisfy the geometry,
lifecycle, multi-output, accessibility, configuration, performance, and verification requirements
in this document.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be
interpreted as normative requirements.

## Executive Decision

Hydebar remains a Rust application built on `iced` and `iced_layershell`. It will not be rewritten
in Quickshell, QML, or Qt.

Caelestia is a reference for design-system discipline and interaction quality, not an implementation
dependency and not a visual template to copy literally. Hydebar should adopt the following ideas:

- semantic color and geometry tokens;
- shared interaction-state rendering;
- explicit spatial and effect motion categories;
- reusable surface and card roles;
- anchored reveals and interruptible transitions;
- declarative configuration and per-output behavior;
- strict visual regression tests.

Hydebar keeps its own identity:

- HyDE and Wallbash remain the source of dynamic color;
- Hyprland remains the source of compositor radius, borders, shadows, and blur policy;
- compact islands remain the primary bar grammar;
- existing spring, FLIP, theme-sweep, and island-fusion mechanisms are retained where appropriate;
- the Rust service, event-bus, lifecycle, and typed-configuration architecture remains intact.

The first milestone is visual quality and correctness. A third-party module SDK is planned after the
visual system is stable, but early refactors must not make that SDK harder to introduce.

## Product Goals

The completed visual milestone MUST provide:

1. A production-quality replacement for Waybar in a HyDE session.
2. Correct four-sided inward-rounded screen framing.
3. One coherent appearance across the bar, menus, settings, notifications, cards, and overlays.
4. Smooth, interruptible animations without idle redraw or unnecessary allocation.
5. Live adoption of Wallbash, HyDE, and compositor appearance changes.
6. User-facing configuration that changes design intent rather than exposing renderer internals.
7. Correct behavior on multiple outputs, mixed dimensions, transformed outputs, and mixed scale.
8. Complete keyboard focus visibility and reduced-motion support.
9. Deterministic visual tests that reject geometry and styling regressions.
10. No visual workaround that hides an incorrect surface lifecycle or geometry model.

## Non-Goals For The Visual Milestone

The following are explicitly deferred:

- rewriting built-in services in another language;
- exposing arbitrary `iced::Element` trees to external modules;
- loading Rust dynamic libraries;
- implementing a general shader or particle framework;
- cloning Material 3 or Caelestia component-for-component;
- publishing every internal spacing or animation constant as permanent public configuration;
- migrating all existing modules to a new trait-object architecture in one change;
- adding visual effects before geometry and lifecycle correctness are proven.

## Current Architecture To Preserve

The existing dependency direction remains mandatory:

```text
app -> gui -> core -> proto
```

- `proto` owns serialized configuration and renderer-independent vocabulary.
- `core` owns modules, services, the event bus, reusable components, style resolution, and animation.
- `gui` owns application composition, module dispatch, per-output view selection, and lifecycle wiring.
- `app` owns process startup, runtime construction, logging, and single-instance behavior.

Public configuration types MUST NOT depend on `iced`. Internal design tokens MUST NOT leak into
`proto` unless users have a concrete, stable reason to configure them.

## Existing Worktree Warning

At the time this specification was written, the following frame-related files contained uncommitted
changes:

```text
core/src/frame.rs
core/src/frame/geometry.rs
core/src/frame/program.rs
core/src/frame/vertical.rs
gui/src/app/view/frame.rs
proto/src/config/appearance/hyde.rs
```

An implementing agent MUST inspect the current worktree and diff before editing these files. It MUST
NOT revert, overwrite, or reconstruct those changes from an older revision. The current changes are
not assumed correct merely because they already exist; they must be evaluated against this
specification.

## Professional UI Invariants

Every implementation decision MUST preserve these invariants.

### Geometry Invariants

- The screen-frame viewport is square at the physical output boundary.
- Frame rounding belongs to the inner content aperture, not the outer output boundary.
- Drawing and layer-shell reservation are separate concerns.
- Canvas geometry is calculated in the canvas's actual logical coordinate space.
- Each output owns its own geometry and surface state.
- No output may use dimensions or animation state selected from another output.
- A globally disabled frame creates neither visible paint nor invisible work-area reservations.
- The edge occupied by the bar is reserved by the bar, not reserved a second time by the frame.
- Hot reload and output removal destroy every surface that is no longer required.

### Design-System Invariants

- Components consume semantic roles, not arbitrary raw palette slots.
- Identical interaction states render consistently across component families.
- Radius, spacing, padding, and control dimensions come from one resolved system.
- Wallbash and HyDE remain source inputs; the renderer derives semantic roles from them.
- No production component invents an isolated design vocabulary.
- Public configuration expresses intent and scale, not implementation-specific pixel tables.

### Motion Invariants

- Motion is interruptible and retargetable.
- A settled interface has no animation subscription or redraw clock.
- Color/opacity effects and geometry/position movement have separate policies.
- Reduced motion removes spatial movement while retaining clear state feedback.
- Animation MUST NOT compensate for incorrect final geometry.
- Exclusive zones MUST NOT animate unless the product explicitly requires moving application work
  areas and compositor behavior has been verified.

### Reliability Invariants

- Appearance-only changes do not restart unrelated listeners or services.
- A rendering failure in one optional module must not corrupt global output lifecycle state.
- Configuration reload is atomic from the user's perspective.
- The previous valid appearance remains active when a new configuration is invalid.
- Multi-output behavior is tested, not inferred from a single-monitor implementation.

## Target Visual Language

The default visual language is **HyDE Refined**:

- compact rather than mobile-sized;
- dynamic Wallbash color with restrained surface contrast;
- compositor-derived island rounding;
- clear hierarchy through spacing before adding more filled cards;
- selective pill geometry for workspaces, badges, and binary compact controls;
- connected islands where modules logically form a group;
- subtle borders and shadows that follow compositor policy;
- expressive movement for opening, relocation, and disclosure;
- quiet effect transitions for hover, press, focus, and changing values.

The default MUST NOT use blob deformation, large Material cards everywhere, continuous radial
ripples, or unrelated pill shapes merely to resemble Caelestia.

## Target Public Configuration

The final serialized shape should remain small and stable. Exact field names may change during the
schema design review, but the responsibilities MUST remain separate.

```toml
[appearance]
profile = "native"

[appearance.density]
spacing = 1.0
padding = 1.0
controls = 1.0

[appearance.shape]
rounding = 1.0
controls = "soft"

[appearance.motion]
mode = "system"
speed = 1.0
expressiveness = 0.35

[appearance.effects]
borders = "follow-compositor"
shadows = "follow-compositor"
surface_contrast = 1.0

[appearance.frame]
enabled = true
thickness = 10.0
rounding = 25.0
fullscreen = "hide"
```

Supported profiles SHOULD be:

| Profile | Purpose |
|---|---|
| `native` | Follow HyDE, Wallbash, and compositor values with balanced motion. |
| `quiet` | Retain polish while suppressing long sweeps and decorative spatial motion. |
| `expressive` | Enable stronger anchored reveals, stagger, and signature transitions. |
| `compact` | Preserve Waybar-like density for small or information-heavy bars. |

Profiles are resolver inputs, not alternate component implementations. A component MUST NOT branch
into an unrelated render tree for each profile unless its information architecture actually changes.

Existing configuration fields MUST remain accepted during migration. Compatibility conversion must
happen before the internal design system is resolved. Components MUST eventually stop reading legacy
raw fields directly.

# Workstream A: Frame Geometry And Surface Lifecycle

This workstream is the first implementation priority. No frame polish or animation may be accepted
before it is complete.

## A1. Required Geometric Model

The frame is a square output rectangle minus a rounded inner aperture, minus the docked bar band.

```text
outer = Rect(0, 0, viewport.width, viewport.height)
outer corner radii = 0

aperture.x = left_inset
aperture.y = top_inset
aperture.width = viewport.width - left_inset - right_inset
aperture.height = viewport.height - top_inset - bottom_inset
aperture corner radii = configured inward rounding, clamped to aperture dimensions

paint = outer minus aperture minus bar band (when the bar is docked)
```

The bar paints its own band: the ring MUST NOT paint a pixel the bar
paints, or translucent styles gain a second coat under the strip. The band
cut is a straight full-width rectangle of the bar's height on the docked
edge; it ends where the aperture begins — the docked inset already reaches
at least the bar height — so the two stay disjoint by construction and the
even-odd fill clears each exactly once. This disjointness MUST be asserted
by test, not assumed.

For a bar docked to the top:

```text
top_inset = max(enabled_top_frame_width, bar_visual_height)
top_frame_reservation = 0
top_bar_reservation = bar exclusive zone
```

For a bar docked to the bottom, the equivalent rule applies to `bottom_inset` and the bottom frame
reservation.

The visual result MUST satisfy:

- the screen-facing boundary is square wherever the ring paints;
- the content-facing boundary is rounded inward;
- the configured rounding is not reduced by frame thickness;
- translucent paint is applied exactly once at every point;
- no ring pixel falls inside the bar's own band;
- the aperture center remains fully transparent.

The old interpretation `inner_radius = outer_radius - thickness` MUST NOT be used. It models a
conventional rounded rectangle border and does not express the required inward screen frame.

## A2. Surface Architecture

Each output SHOULD own this explicit aggregate:

```rust
pub struct FrameSurfaces {
    pub visual: Option<SurfaceId>,
    pub reservations: EdgeMap<Option<SurfaceId>>,
}
```

The exact names may vary, but visual and reservation IDs MUST be grouped under the output that owns
them.

The implementation MUST use:

1. One full-output, transparent, click-through visual surface for continuous frame paint.
2. Up to four transparent edge surfaces for independent layer-shell exclusive zones.
3. The existing bar surface as the sole reservation owner for the docked edge.

The implementation MUST NOT use independently painted edge strips. Separate painted strips create
cross-surface clipping, overlap, alpha accumulation, and fractional-scale seams.

The visual surface MUST:

- target one specific output;
- anchor to all four output edges;
- reserve no work area;
- have an empty input region;
- use the intended bar/frame namespace and compositor layer;
- render from actual canvas bounds;
- remain visually continuous across all four edges.

Reservation surfaces MUST:

- be fully transparent;
- have an empty input region;
- reserve exactly one edge each;
- use logical layer-shell units;
- be absent or have a zero zone when their edge is disabled;
- be absent on the edge already owned by the bar.

## A3. Per-Output State

Global `screen_width` and `screen_height` values MUST NOT drive frame rendering.

The frame view dispatch MUST know which surface is being rendered and resolve its owning output.
Per-output data must include, directly or through an existing output record:

- output identity;
- monitor name;
- current logical bounds;
- scale information;
- transform or portrait state when relevant;
- frame surface IDs;
- bar position and visual height;
- fullscreen visibility state;
- output-specific animation or desk wash state.

The canvas `draw` bounds are authoritative for visual geometry. Compositor-reported dimensions MAY
be used for lifecycle decisions and tests, but they MUST NOT replace actual render-target bounds.

## A4. Normalization Rules

The pure frame-layout function MUST normalize all input before path construction:

- negative thickness becomes zero or is rejected by config validation;
- disabled edges produce zero inset and zero reservation;
- aperture width and height never become negative;
- radius is clamped to half the smaller aperture dimension;
- degenerate apertures have explicitly defined behavior;
- no coordinate may be NaN or infinite;
- asymmetric edge widths remain valid;
- the docked edge is visually represented by bar height but is not double-reserved.

The preferred behavior when frame widths consume the entire output is a validation error. If runtime
normalization is still required for safety, it must produce deterministic full-frame paint without
invalid paths or panics.

## A5. Disabled-Edge Semantics

The initial professional milestone SHOULD use strict-edge semantics:

- an aperture is flush with a disabled edge;
- an inner corner is rounded only when both adjacent edges exist;
- otherwise that corner is square.

This policy is easy to reason about and test. Rounded end caps for partially enabled frames MAY be a
future feature, but MUST use a dedicated path model rather than accidental rounded-rectangle
behavior.

## A6. Lifecycle Reconciliation

`Outputs::sync` or its replacement MUST reconcile frame state whenever any of these changes:

- global frame enablement;
- edge enablement;
- edge width;
- frame rounding where surface recreation is unnecessary;
- bar position;
- bar height;
- output mode or logical size;
- output scale;
- output selection;
- compositor layer;
- namespace-affecting configuration;
- fullscreen policy.

Reconciliation MUST:

1. Compute desired surfaces from current config and output state.
2. Compare desired state with owned surfaces.
3. Create missing surfaces.
4. Reconfigure or recreate surfaces whose protocol settings changed.
5. Destroy obsolete surfaces.
6. Commit the new ownership record only after commands are constructed successfully.
7. Be idempotent when called repeatedly with unchanged state.

All removal paths MUST destroy the visual surface and every reservation surface. Tests must detect a
forgotten top, bottom, left, or right ID independently.

## A7. Fullscreen Policy

Add a renderer-independent configuration enum in `proto`:

```rust
pub enum FrameFullscreenPolicy {
    Hide,
    Keep,
    FollowLayer,
}
```

Recommended default: `Hide`.

Fullscreen state must be tracked per output. A fullscreen client on output A MUST NOT hide the frame
on output B. A hidden frame SHOULD fade only its visual alpha; reservations should follow an explicit
product decision. The default SHOULD retain reservations to avoid moving application windows when a
fullscreen client appears or disappears.

## A8. Frame Tests

### Pure geometry tests

Required cases:

- equal widths on four edges;
- asymmetric widths;
- radius zero;
- radius larger than the aperture limit;
- negative and zero widths;
- each single disabled edge;
- every adjacent disabled-edge pair;
- top bar;
- bottom bar;
- zero-size and tiny viewport;
- portrait viewport;
- widths consuming the aperture;
- finite coordinates for all accepted input.

Tests MUST assert representative points, not only stored rectangles.

### Raster or golden tests

Required deterministic scenes:

```text
64x64, width 8, radius 16
96x64, widths 5/9/13/17
64x96 portrait
top bar with translucent frame
bottom bar with translucent frame
disabled left edge
```

Alpha-sensitive tests MUST detect double paint.

### Surface settings tests

- disabled frame creates no visual and no reservation surfaces;
- top bar creates no top frame reservation;
- bottom bar creates no bottom frame reservation;
- visual surface has all anchors and no exclusive zone;
- every surface targets the correct output;
- every frame-related surface is click-through.

### Lifecycle tests

- enable after startup;
- disable after startup;
- width change;
- top-to-bottom bar move;
- output hotplug;
- output replacement with the same monitor name;
- repeated unchanged sync;
- complete shutdown;
- no surviving or leaked surface IDs.

## A9. Frame Acceptance Gate

Workstream A is complete only when:

- outer corners are square in geometry and raster tests wherever the ring paints;
- inner rounding visually matches the configured radius;
- no global screen dimensions are used by the frame renderer;
- frame enablement and edge widths hot-reload correctly;
- no edge is double-reserved;
- every output owns and renders independent state;
- all frame surfaces are destroyed on removal and shutdown;
- the old limitations in `docs/FRAME.md` are corrected;
- tests cover all requirements above.

# Workstream B: Semantic Design System

## B1. Internal Module Layout

Introduce an internal resolved system under `core/src/style/system/`:

```text
core/src/style/system.rs
core/src/style/system/color.rs
core/src/style/system/space.rs
core/src/style/system/shape.rs
core/src/style/system/motion.rs
core/src/style/system/state.rs
core/src/style/system/metrics.rs
```

The system is renderer-owned and is computed from validated `Appearance`, HyDE theme data,
compositor values, and selected profile.

Suggested root type:

```rust
pub struct UiSystem {
    pub colors: SemanticColors,
    pub space: SpaceScale,
    pub shape: ShapeScale,
    pub motion: MotionScale,
    pub metrics: ComponentMetrics,
}
```

`App` SHOULD cache one resolved `UiSystem` next to its theme cache. Resolution occurs on appearance
reload, not during individual component rendering.

## B2. Semantic Color Roles

The minimum required roles are:

```rust
pub struct SemanticColors {
    pub canvas: Color,
    pub strip: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub surface_recessed: Color,
    pub surface_hovered: Color,
    pub surface_pressed: Color,
    pub on_surface: Color,
    pub on_surface_muted: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub outline: Color,
    pub outline_focus: Color,
    pub positive: Color,
    pub warning: Color,
    pub critical: Color,
    pub scrim: Color,
}
```

Mapping requirements:

- `strip` comes from the HyDE/Wallbash bar background role.
- `surface` comes from the normal module or menu container role.
- `accent` comes from active/selection background.
- `on_accent` comes from active/selection foreground.
- `surface_hovered` honors HyDE's hover background.
- hover foreground data must not be discarded when provided by HyDE.
- `outline` is a neutral structural role and MUST NOT alias an arbitrary secondary accent.
- status roles continue to use existing positive, warning, and danger configuration.
- every derived foreground must pass contrast validation against its intended background.

Components MUST NOT select colors by directly interpreting `AppearanceColor.weak` or `.strong` once
migrated. Those remain source values used by the resolver.

## B3. Spacing And Density

Use one named logical scale:

```rust
pub struct SpaceScale {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}
```

Initial nominal values MAY be `4, 6, 8, 12, 16, 24, 32`, but the resolver must combine them with:

- profile density;
- user spacing scale;
- user padding scale;
- typography metrics where an `em` relationship is semantically required;
- application logical scale.

Production components MUST NOT add new unexplained spacing literals. A literal is acceptable only
for a protocol constant, a one-device-pixel alignment rule, or a value with a documented geometric
reason.

## B4. Shape Scale

Use semantic shape roles:

```rust
pub struct ShapeScale {
    pub none: f32,
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub island: f32,
    pub pill: f32,
}
```

Required resolution rules:

- `island` follows the explicit user radius, then Hyprland radius, then a safe default.
- card radius is derived from `large` or `island`, not from page font size.
- a pill is exactly constrained by control height and is not a magic fixed number.
- connected corners are asymmetric only when surfaces are logically connected.
- frame inward rounding is a separate frame property, not `ShapeScale::island`.

Existing hard-coded component radii such as `32`, `16`, and `4` must be inventoried and either
replaced with a role or documented as a necessary exception.

## B5. Component Metrics

Centralize recurring dimensions without creating a public token dump:

```rust
pub struct ComponentMetrics {
    pub bar_height: f32,
    pub compact_target: f32,
    pub menu_target: f32,
    pub icon_small: f32,
    pub icon_medium: f32,
    pub icon_large: f32,
    pub border_hairline: f32,
    pub focus_ring: f32,
    pub menu_max_height_ratio: f32,
}
```

Metrics may derive from existing font-relative appearance measurements. Existing `em` behavior
should be preserved where it intentionally maintains typography rhythm.

## B6. Visual-Equivalence Migration

The first design-system change MUST aim for visual equivalence:

1. Resolve current values into `UiSystem`.
2. Migrate one component family at a time.
3. Compare deterministic snapshots before and after.
4. Keep pixel change below the agreed tolerance until redesign begins explicitly.
5. Remove direct raw appearance reads only after all callers use semantic roles.

This prevents a foundational refactor from being mixed with subjective redesign.

## B7. Design-System Acceptance Gate

- one cached semantic resolver exists;
- Wallbash dark and light fixtures resolve valid contrast roles;
- component families no longer derive unrelated meanings from the same raw color;
- radius and spacing literals are inventoried;
- migrated components consume semantic roles only;
- baseline visual delta is documented and reviewed;
- no measurable idle CPU regression is introduced.

# Workstream C: Shared Interaction States

## C1. Required State Model

All actionable components must support the relevant subset of:

```text
rest
hovered
pressed
focused
selected
disabled
urgent
destructive
```

Introduce a shared state resolver rather than another component-specific style switch.

```rust
pub struct InteractionState {
    pub enabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub selected: bool,
    pub urgent: bool,
    pub destructive: bool,
}
```

The state resolver should output semantic paint decisions. It should not own application behavior.

## C2. Visual State Rules

- Rest: normal surface and foreground.
- Hover: subtle state overlay or resolved hovered surface.
- Pressed: stronger overlay and optional paint-only radius contraction.
- Focused: visible high-contrast ring at least two logical pixels where scaling allows.
- Selected: accent container or a clearly sized accent indicator.
- Disabled: reduced contrast, no hover or pressed animation, no pointing cursor.
- Urgent: status icon or shape plus color; color alone is insufficient.
- Destructive: danger foreground or outline visible before hover.

The default profile MUST NOT use Material radial ripples. A future optional ripple may be considered
only after interaction consistency, performance, clipping, and reduced-motion behavior are proven.

## C3. Migration Order

1. Bar module buttons.
2. Workspace indicators.
3. Quick-settings controls.
4. Plain settings buttons.
5. Theme and wallpaper cards.
6. Notification actions.
7. Menus and context-menu entries.
8. Sliders, switches, text fields, and radio controls.

Relevant current areas include:

```text
core/src/style/buttons/module.rs
core/src/style/buttons/quick.rs
core/src/style/buttons/workspace.rs
core/src/style/buttons/plain/
core/src/components/page/widgets/controls.rs
gui/src/app/modules/element/pill.rs
```

## C4. Interaction Tests

Every actionable primitive must have tests for:

- rest;
- hover;
- press;
- keyboard focus;
- selected and unselected;
- disabled;
- urgent or destructive when supported;
- interrupted state animation;
- theme switch while hovered or focused;
- reduced-motion and motion-off behavior.

# Workstream D: Reusable Surfaces And Cards

## D1. Surface Roles

Generalize the existing settings-card vocabulary into a shared primitive:

```rust
pub enum SurfaceRole {
    Base,
    Raised,
    Recessed,
    Selectable,
    Critical,
}
```

The primitive controls only common presentation:

- semantic background;
- optional outline;
- shape role;
- standard padding;
- elevation or compositor-following shadow decision.

It must not become a universal component with unrelated layout, navigation, and business logic.

## D2. Hierarchy Rules

- Use spacing and typography before adding another filled layer.
- Do not render more than two visibly filled nested surface levels by default.
- A recessed control area must look distinct from a raised card.
- Selection must not be represented solely by an extra nested rectangle.
- Cards in settings, notifications, media, themes, and quick settings must share roles.
- A menu is a surface role, not a unique palette implementation for every page.

## D3. Migration Targets

- settings scaffolds and cards;
- theme cards and chips;
- notification rows and groups;
- quick-settings groups and submenus;
- media controls;
- calendar cells and selected day;
- wallpaper picker entries;
- system-information meters where they use filled containers.

# Workstream E: Menu Contract And Motion

## E1. Replace The Tuple Contract

Replace the current page tuple with a semantic contract similar to:

```rust
pub struct MenuPage<'a> {
    pub content: Element<'a, Message>,
    pub size: MenuSize,
    pub measured_height: Option<f32>,
    pub surface: MenuSurfaceRole,
    pub navigation: NavigationModel,
    pub reveal: RevealPolicy,
    pub focus: FocusPolicy,
}
```

Suggested supporting enums:

```rust
pub enum MenuSurfaceRole {
    Menu,
    Panel,
    Dialog,
    Picker,
}

pub enum RevealPolicy {
    EffectsOnly,
    EdgeReveal,
    AnchoredReveal,
}
```

Every page must declare its role. The wrapper, not the page business logic, applies shared backdrop,
radius, outline, clipping, placement, overflow, and reduced-motion policy.

## E2. Motion Taxonomy

Separate motion into intent-specific categories:

```rust
pub struct MotionScale {
    pub effect_fast: MotionSpec,
    pub effect_standard: MotionSpec,
    pub spatial_fast: MotionSpec,
    pub spatial_standard: MotionSpec,
    pub spatial_large: MotionSpec,
}
```

Effects include:

- color interpolation;
- opacity;
- shadow or outline interpolation;
- changing-value crossfade.

Spatial motion includes:

- position;
- extent;
- clipping reveal;
- disclosure;
- FLIP relocation;
- island joining and separation.

The current `SNAPPY`, `STANDARD`, `GENTLE`, and `SWEEP` springs SHOULD be mapped into these semantic
roles rather than discarded.

## E3. Menu Opening Sequence

The normal full-motion sequence SHOULD be:

1. Begin scrim or backdrop effect fade.
2. Reveal the menu from the measured opener edge.
3. Expand clipped extent using spatial motion.
4. Fade content using effect motion.
5. Optionally stagger major content groups in `expressive` mode only.
6. Move keyboard focus after geometry is available.

Closing reverses intent, not necessarily every frame literally. It must remain interruptible: a
closing menu reopened halfway must retarget from its current position and velocity.

The implementation MUST NOT animate from a fabricated output center. It must use the actual measured
bar module or opener bounds.

## E4. Disclosure

Create one reusable disclosure primitive for:

- HyDE menu tree branches;
- control-center submenus;
- expandable notification groups;
- optional settings detail groups.

The primitive must support:

- clipping without relayout jumps;
- interrupted opening and closing;
- full, reduced, and off motion;
- deterministic progress for visual tests;
- no persistent animation state when closed and settled.

## E5. FLIP Rules

Use FLIP only for actual relocation:

- module reorder;
- movement between bar sections;
- workspace active-indicator travel;
- desk-block rearrangement.

Do not use FLIP for hover, color changes, selection, or simple expansion.

## E6. Scale And Morph Limits

- Whole-menu scale is prohibited until transformed clipping and hit testing are verified.
- Text must not blur from repeated scale animation.
- Shape morph is paint-only by default and must not move neighboring layout.
- Ordinary controls should not overshoot aggressively.
- Long theme sweep and desk flourish remain optional signature effects.

# Workstream F: Accessibility And Input

## F1. Keyboard Requirements

- Every actionable bar module has a visible focused state.
- Left and right navigate bar entries in visual order.
- Enter and Space activate the focused entry.
- Menu Up and Down navigate actionable rows.
- Left and Right collapse and expand hierarchical rows where appropriate.
- Escape closes the current menu level or surface.
- Focus returns to the opener after a menu closes.
- Keyboard-opened menus use the opener's measured bounds.
- Configured keybindings are authoritative; hard-coded duplicates are prohibited.

Known areas requiring correction include:

```text
gui/src/app/update/navigation.rs
gui/src/app/update/subscriptions/sources.rs
gui/src/app/modules/element/pill.rs
proto/src/config/keybindings.rs
```

## F2. Contrast And Size Requirements

- Normal text contrast: at least 4.5:1.
- Large text and essential graphics: at least 3:1.
- Focus indicator contrast: at least 3:1 against adjacent colors.
- Compact bar target: at least 28-32 logical pixels where output constraints permit.
- Menu action target: at least 40 logical pixels.
- The entire visible island or row should be clickable, not only its glyph.
- State must never depend exclusively on red/green distinction.

Automated contrast assertions must cover every shipped built-in theme and deterministic Wallbash
dark/light fixtures.

## F3. Motion Modes

Required modes:

| Mode | Effects | Spatial motion | Signature motion |
|---|---|---|---|
| `system` | Determined by resolved preference | Determined by resolved preference | Determined by profile |
| `full` | Enabled | Enabled | Profile-controlled |
| `reduced` | Maximum 100 ms where useful | Snapped | Disabled |
| `off` | Snapped | Snapped | Disabled |

`system` resolution order:

1. explicit user override;
2. desktop accessibility or portal preference when available;
3. Hyprland animation setting as fallback;
4. full motion otherwise.

Even with motion off, hover, focus, selected, pressed, and disabled states remain distinct.

# Workstream G: HyDE And Wallbash Integration

## G1. Source Of Truth

Hydebar should consume XDG-resolved HyDE state, not hard-coded home-directory paths. Relevant sources
include:

```text
$XDG_STATE_HOME/hyde/lua_state/colors.lua
$XDG_STATE_HOME/hyde/lua_state/ui.lua
$XDG_STATE_HOME/hyde/staterc
$XDG_CACHE_HOME/hyde/wall.set
$XDG_CACHE_HOME/hyde/wall.dcol
$XDG_CONFIG_HOME/hypr/themes/colors.conf
```

Existing typed theme extraction should remain the primary integration path where it already handles
these sources correctly.

## G2. Optional Dedicated Wallbash Target

A dedicated Hydebar Wallbash target MAY be added later to avoid parsing formats intended for Lua or
Waybar. If introduced, it should generate one atomic renderer-independent document containing
semantic source roles, not renderer colors.

Example target data:

```json
{
  "mode": "dark",
  "bar": "#0f1a2a",
  "onBar": "#ffffff",
  "surface": "#506182",
  "onSurface": "#ffffff",
  "accent": "#d1768b",
  "onAccent": "#0e0f0f",
  "hover": "#695476",
  "onHover": "#ffffff"
}
```

The target file must be watched as an atomically replaced file because Wallbash writes temporary
files and renames them. Watch logic must survive inode replacement.

## G3. Wallpaper And Theme UI

Wallpaper UI should use the existing HyDE command contract and cached thumbnails rather than
decoding full-size wallpapers on the UI thread.

Relevant command behavior:

```text
hyde-shell wallpaper --json
hyde-shell wallpaper --get
hyde-shell wallpaper --set <path> --global
hyde-shell theme.switch -s <name>
```

Global wallpaper changes must be serialized so an expensive Wallbash regeneration is not launched
concurrently multiple times.

## G4. Session Integration

- Waybar and Hydebar must not both reserve the same bar edge.
- Hydebar startup should replace the HyDE Waybar startup command through supported user overrides.
- Existing Waybar-specific signal keybindings need Hydebar IPC equivalents.
- Blur rules must cover the actual Hydebar namespace.
- Gaming and power-saving workflows must apply equivalent blur/motion policies to Hydebar.

# Workstream H: Visual Regression And Performance

## H1. Deterministic Visual Suite

Suggested structure:

```text
gui/tests/visual.rs
gui/tests/visual/bar.rs
gui/tests/visual/frame.rs
gui/tests/visual/menus.rs
gui/tests/visual/states.rs
gui/tests/visual/motion.rs
gui/tests/visual/accessibility.rs
gui/tests/goldens/
```

Required scene matrix:

- deterministic Wallbash dark and light;
- Catppuccin Mocha and Latte;
- Islands, Solid, and Gradient styles;
- top and bottom bar;
- logical scale 1.0, 1.5, and 2.0;
- native, compact, quiet, and expressive profiles;
- rest, hover, pressed, focus, selected, urgent, and disabled states;
- control center;
- notifications;
- settings;
- themes;
- wallpaper picker;
- calendar;
- menu closed, 25%, 50%, 75%, and open;
- frame equal and asymmetric widths;
- portrait and landscape viewport.

## H2. Comparison Policy

- Token migration before redesign: exact or at most 0.1% differing pixels.
- Intentional redesign: reviewed golden update with before/after montage.
- Geometry masks are compared separately from color output.
- Alpha-sensitive comparisons detect overlap and double paint.
- Anti-aliasing tolerance must not permit one-pixel clipping or radius changes to pass silently.
- Tests assert bounds, target dimensions, and contrast in addition to pixels.

## H3. Performance Gates

Preserve these properties:

- no idle frame subscription after motion settles;
- no per-frame file reads;
- no per-frame image decode;
- no per-frame theme or token resolution;
- no persistent heap state for inert hover animations;
- compositor-owned blur;
- bounded event delivery and snapshot coalescing.

Required budgets relative to the existing measured baseline:

- idle CPU: no statistically meaningful regression;
- startup-to-surface: less than 10% regression;
- animated menu cost: less than 10% regression;
- resident memory: less than 5 MiB increase for the visual foundation;
- UI update/render preparation p95: below 8 ms on the existing 4K reference system;
- unplaced modules: no new polling or animation wakeups.

Any budget failure requires profiling and a written explanation. An agent MUST NOT simply increase a
threshold to make CI pass.

# Workstream I: Module Extensibility Foundation

This workstream follows the visual milestone. It is included now to prevent design-system work from
creating another closed, duplicated registration system.

## I1. Immediate Internal Catalog

Introduce one explicit built-in module catalog that owns metadata currently repeated across dispatch,
owner, registration, tooltip, menu, and configuration paths.

Suggested descriptor responsibilities:

```rust
pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub label: &'static str,
    pub owner: Option<ModuleId>,
    pub activation: Activation,
    pub menu: Option<MenuDescriptor>,
    pub snapshot_policy: SnapshotPolicy,
}
```

Do not immediately replace all concrete `App` fields with trait objects. Use adapters around existing
built-ins first. The catalog should reduce identity and metadata duplication while preserving typed
state and exhaustive built-in dispatch.

## I2. Future External API

The preferred external architecture is:

1. `hydebar-module-api`: protocol-neutral types without `iced` or Tokio.
2. A semantic host-rendered view tree.
3. A versioned supervised process protocol.
4. `hydebar-module-sdk`: ergonomic Rust helpers.
5. XDG manifest discovery.

External modules should describe meaning:

```rust
View::row([
    Node::icon("weather-cloudy"),
    Node::text("12 C"),
])
.tooltip("Cloudy")
.action(MouseButton::Left, "refresh")
```

Hydebar owns fonts, color, spacing, interaction states, animation, accessibility, and island
composition. External modules must not inject arbitrary raw style by default.

## I3. Rejected Extension Designs

- Rust dynamic libraries: rejected because Rust has no stable ABI and cannot safely exchange Iced
  elements, trait objects, allocators, panics, or futures across that boundary.
- `inventory` as runtime discovery: rejected because it only discovers crates already linked into the
  binary.
- WASM in the first SDK release: deferred because host capabilities and asynchronous system APIs
  would dominate the work before the process protocol is proven.
- Unrestricted arbitrary widget trees: rejected because they bypass native design-system and
  accessibility guarantees.

# Implementation Phases

Each phase is independently reviewable and must satisfy its gate before the next phase begins.

## Phase 0: Baseline And Specification Tests

Deliverables:

- current visual capture matrix;
- reproduced performance baseline;
- geometry fixtures for the frame;
- inventory of hard-coded colors, radii, spacing, and durations;
- explicit mapping from HyDE roles to semantic roles;
- documented current worktree frame changes.

No production redesign occurs in this phase.

## Phase 1: Frame Correctness

Deliverables:

- pure inward-frame layout;
- square outer boundary and rounded aperture;
- per-output frame view;
- grouped surface ownership;
- complete reconciliation and teardown;
- removal of double reservation;
- geometry, raster, surface, lifecycle, and mixed-output tests;
- corrected `docs/FRAME.md`.

Phase 1 must be completed before frame animation.

## Phase 2: Semantic Tokens With Visual Equivalence

Deliverables:

- cached `UiSystem`;
- semantic color mapping;
- spacing, shape, motion, and component metric scales;
- migration of bar and menu shell without intentional redesign;
- visual-equivalence snapshots.

## Phase 3: Shared Interaction States

Deliverables:

- state resolver;
- bar, workspace, quick-settings, and settings-button migration;
- visible focus;
- disabled, selected, urgent, and destructive states;
- interaction-state visual fixtures.

## Phase 4: Shared Surfaces And Shape Cleanup

Deliverables:

- shared surface roles;
- migrated cards and recessed groups;
- removal or justification of fixed component radii;
- no more than two default nested filled levels;
- clipping checks across scale and placement matrix.

## Phase 5: Menu Contract And Motion Split

Deliverables:

- semantic `MenuPage` contract;
- separate presence and extent progress;
- anchored reveal from measured opener;
- reusable disclosure;
- full/reduced/off behavior;
- interrupted-transition tests.

## Phase 6: Public Profiles And Settings UI

Deliverables:

- density, shape, motion, and effects configuration;
- native, quiet, expressive, and compact profiles;
- settings controls for supported public values;
- backward-compatible deserialization;
- atomic hot reload;
- documentation and examples.

## Phase 7: Accessibility Completion

Deliverables:

- full keyboard traversal;
- configured keybindings;
- measured keyboard menu anchors;
- focus restoration;
- contrast validation;
- target-size validation;
- system reduced-motion integration.

## Phase 8: Full Visual Polish

Only now apply intentional final redesign to:

- bar island rhythm;
- workspace active state;
- control-center composition;
- notification hierarchy;
- media and calendar cards;
- settings and theme views;
- wallpaper picker;
- OSD and tooltip consistency.

Every changed scene requires reviewed before/after evidence.

## Phase 9: Module Catalog

Deliverables:

- stable module IDs;
- one internal metadata catalog;
- table-driven adapters where practical;
- reduced multi-file registration checklist;
- no functional regression to existing built-ins or custom modules.

The external process SDK remains a later project with its own specification.

# Agent Execution Protocol

Any agent implementing this plan MUST follow this order for each phase.

## Before Editing

1. Read this document and the relevant existing architecture documentation.
2. Inspect `git status` and the exact diff of files it will touch.
3. Identify unrelated user changes and preserve them.
4. Locate existing tests and conventions before introducing new abstractions.
5. State the phase and acceptance gate being implemented.
6. Refuse to combine unrelated later phases into the same change.

## During Editing

1. Prefer the smallest architecture that satisfies the complete phase.
2. Keep pure geometry and resolver logic independent from rendering and I/O.
3. Add tests with the implementation, not afterward.
4. Do not add compatibility code without a concrete persisted or public behavior to preserve.
5. Do not expose internal tokens merely to avoid designing a semantic configuration field.
6. Do not add visual conditionals directly to individual modules when the rule belongs to the shared
   system.
7. Do not silence warnings or dead code to land an incomplete abstraction.
8. Do not claim multi-output support without tests using at least two distinct output geometries.

## Before Declaring Completion

The agent MUST provide:

- files changed;
- behavioral changes;
- preserved compatibility;
- tests added;
- commands run and exact result;
- performance measurements when the phase affects rendering or motion;
- unresolved requirements from the current phase;
- before/after visual evidence for intentional redesign.

An agent MUST NOT mark a phase complete when any REQUIRED acceptance item is deferred.

# Prohibited Shortcuts

The following approaches are explicitly non-professional and must be rejected:

- rounding the outer frame because it looks close in one screenshot;
- covering wrong geometry with another opaque rectangle;
- painting four independent translucent frame strips;
- using the first or last monitor's dimensions for all outputs;
- retaining invisible reservation surfaces while the frame is disabled;
- adding arbitrary delays to hide surface lifecycle races;
- restarting the entire shell for appearance-only changes;
- hard-coding 1920x1080 menu coordinates;
- adding more one-off button styles instead of using shared interaction states;
- using accent colors as structural outlines without a semantic reason;
- adding fixed radii to repair one component at one scale;
- exposing the entire internal token table as public configuration;
- animating exclusive zones as a substitute for a stable final layout;
- running a permanent 60 FPS animation clock while idle;
- accepting screenshot-only verification without geometry and lifecycle tests;
- rewriting existing services or modules while implementing the visual foundation;
- introducing Rust dynamic plugin libraries;
- replacing unrelated user worktree changes.

# Verification Commands

At minimum, every completed phase must run the relevant subset, and the final milestone must run the
complete project-required suite:

```text
cargo +nightly fmt --check
cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery
cargo test --all-features
cargo llvm-cov
cargo bench --no-fail-fast
cargo build --release --all-features
cargo audit
cargo doc --no-deps --all-features
cargo qual
actionlint
hadolint Dockerfile
yamllint
reuse lint
```

If a command is unavailable in the environment, the result must state that explicitly. It must not
be reported as passed.

# Final Definition Of Done

The professional visual milestone is complete only when all statements below are true.

## Frame

- The frame has square screen-facing corners wherever the ring paints, and rounded content-facing corners.
- The configured radius directly controls inward aperture rounding.
- Each output uses its own actual logical render bounds.
- The docked bar edge is not double-reserved.
- Disabled frames create no invisible reservation.
- Hot reload, output removal, and shutdown leave no frame surfaces behind.
- Fullscreen policy works independently per output.

## Design System

- One cached semantic resolver controls colors, spacing, shapes, motion, and component metrics.
- Wallbash and HyDE remain the dynamic source of appearance.
- Components use semantic roles rather than raw palette interpretation.
- Fixed radii and spacing constants are removed or explicitly justified.
- Bars, menus, cards, notifications, and settings share one visual vocabulary.

## Interaction And Motion

- Rest, hover, press, focus, selection, disabled, urgent, and destructive states are consistent.
- Menus reveal from their real opener.
- Motion is interruptible.
- Effect and spatial policies are separate.
- Reduced and off modes meet their exact requirements.
- The interface has no animation clock while settled.

## Configuration

- Existing configurations continue to deserialize.
- Profiles and focused appearance controls are documented.
- Hot reload applies appearance atomically.
- Invalid changes preserve the previous valid UI.
- Users can configure visual intent without knowing renderer internals.

## Accessibility

- Every actionable element is keyboard reachable where appropriate.
- Focus is visible on every shipped theme.
- Menus restore focus to their opener.
- Contrast and target-size gates pass.
- No state depends only on color.

## Verification

- Deterministic visual scenes cover all required themes, states, scales, and surfaces.
- Geometry masks catch clipping and radius regressions.
- Multi-output and lifecycle tests pass.
- Performance budgets pass or have an approved measured exception.
- Formatting, linting, tests, coverage, release build, documentation, and repository quality checks
  pass.

## Extensibility Readiness

- New visual components can consume the design system without adding raw style logic.
- Existing module behavior remains intact.
- The architecture does not require external modules to control raw visual styling.
- The internal catalog path is clear and does not require a big-bang rewrite.

# Closing Principle

The target is not "Caelestia implemented in Rust." The target is a reliable Rust-native HyDE shell
that reaches Caelestia's level of visual discipline while retaining Hydebar's stronger service,
configuration, lifecycle, and performance properties.

Correct geometry comes before polish. Semantic roles come before component tweaking. Shared behavior
comes before one-off effects. Verification comes before declaring the interface professional.
