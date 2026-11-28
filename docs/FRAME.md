# Frame

The frame is an opt-in ring around the screen, painted after the fashion of
the `Caelestia` shell. It is off by default.

```toml
[appearance.frame]
enabled = true
thickness = 10.0
rounding = 25.0

[appearance.frame.edges]
top = false
bottom = true
left = true
right = true

[appearance.frame.widths]
left = 14.0
```

## Model

The frame is the screen square, less a rounded inner aperture, less the
band of the docked bar. The screen boundary stays square — the physical
corners of the output are fully painted wherever the bar's own band does
not cover them — and every rounding lives on the aperture, where the
content faces it. The configured `rounding` names the aperture radius
directly: a thickness of 10 leaves a requested radius of 25 a radius of
25, ready to continue the strip's own arc. A corner one drawn side gives
way on stays square; a corner two drawn sides meet at reads rounded.

The aperture sits below the band of the docked bar even when no strip is
drawn along its edge, and the band itself is cut straight out of the ring:
the bar paints that rectangle itself, so the ring never lays a second coat
under it. The cut ends where the aperture begins, and the two stay disjoint
by construction — the even-odd fill clears each exactly once. The ring
reads as one paint travelling out of the strip and around the screen. The
edge the bar is docked to is the bar's own exclusive zone, never a second
frame reservation.

## Single source of truth

The frame states no colour of its own. It paints the bar palette's
background at the island opacity, read from the same theme cache the island
buttons draw from — the exact expression the islands rest at. A theme morph
repaints the ring with the bar, frame by frame, because both read the
sampled appearance.

The geometry is per output: every monitor rings itself in its own measured
logical size, so a 4K panel and a 1080p one never borrow each other's
numbers.

## Surfaces

Each output's surface group carries at most one ring and one reservation
per drawn edge:

- one fullscreen visual surface, anchored on all four sides, clicks pass
  through it, reserves nothing;
- up to four panel surfaces, each anchored to its own edge, transparent
  and click-through, reserving only its width as an exclusive zone.

A switched-off edge and the edge the bar occupies own no surface at all.
A config flip is reconciled in place: a switched-on frame gains its ring,
a switched-off one loses ring and reservations, and a resized edge keeps
exactly its new zone. Hot reload and output removal destroy every surface
that is no longer owned.

## Blur

The bar asks the compositor for blur itself — the same `layerrule`
handshake it makes for the strip — naming both the bar surface and the
frame's visual surface. No manual rule is needed, and a `HyDE` theme
switch re-states the rules after the desktop reloads, as it already does
for the bar. Pixels the frame paints fully clear are skipped by the
compositor (`ignore_alpha 0.0`), so the blur is only paid for behind the
strips the user switched on. The frame's own `hydebar-frame` surfaces are
excluded from blur with a layer `blur = false` rule, so a theme blurring
every namespace cannot pull the sharp paint under the bar.

## Honest limits

- The frame does not collapse over fullscreen windows yet: there is no
  fullscreen signal in the core services to drive it. A follow-up wires one
  through and applies a documented per-output policy.
- The frame snaps to a new theme with the target instead of morphing:
  animating its widths mid-transition would relayout the reserved edges
  under the compositor while the palette is still travelling.
