# Frame

The frame is an opt-in ring around the screen, painted on the desk surface
after the fashion of the `Caelestia` shell. It is off by default. One layer
carries both the desk canvas and the frame: the bar strip stays a surface of
its own, because it reserves the screen edge and takes pointer input, while
neither is true of the frame.

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

## Single source of truth

The frame states no colour of its own. It paints the bar palette's
background at the island opacity, read from the same theme cache the island
buttons draw from — the exact expression the islands rest at. A theme morph
repaints the ring with the bar, frame by frame, because both read the
sampled appearance.

Blur is compositor-side and keys on the surface namespace: the frame rides
the desk surface, so a blur rule for the bar strip does not reach it. Mirror
the rule for the desk layer:

```ini
layerrule = blur, hydebar-desk-layer
```

## Model

Each edge is switched and sized on its own: `thickness` paints every edge,
and `[appearance.frame.widths]` overrides any edge with its own width.
The edge the bar is docked to never paints its straight run: the bar band
covers it, and a translucent bar would pick up a second layer of colour
from a strip hiding underneath.

A side strip beside the docked bar starts where the bar band ends and rounds
both corners into a joint — that is the "growing out of the bar" look. A
strip meeting another strip stays square so the two read as one run. A strip
ending at a bare screen corner rounds its outer corner. Inner corners stay
square: the frame is flat 2D paint, not a signed-distance field, so there is
no gooey merge where a panel meets the ring.

The frame is painted in the bar's background colour at the island opacity,
on the strip's own layer — the desk surface shares the strip's level, so
both read the same blur background. The desk surface takes no pointer input
while folded, so clicks pass through to the desktop underneath; the empty
region is stated once the screen is measured, because the region stated
with the creation is dropped before the surface exists.

## Honest limits

- The screen size the geometry is computed from is the last one the
  compositor reported, shared by every output — the same limitation the menu
  placement already lives with. On mixed-size setups the frame follows the
  last measured screen; per-output sizes are a follow-up.
- The frame does not collapse over fullscreen windows yet: there is no
  fullscreen signal in the core services to drive it. A follow-up wires one
  through and fades the strips out with it.
- The frame snaps to a new theme with the target instead of morphing: the
  strips ride the desk surface, and animating their widths mid-transition
  would relayout that surface under the compositor while the palette is
  still travelling.
