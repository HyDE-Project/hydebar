# Frame

The frame is an opt-in ring around the screen, drawn on a surface of its own
after the fashion of the `Caelestia` shell. It is off by default.

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
```

## Model

Each edge is switched on its own. The edge the bar is docked to never paints
its straight run: the bar band covers it, and a translucent bar would pick up
a second layer of colour from a strip hiding underneath.

A side strip beside the docked bar starts where the bar band ends and rounds
both corners into a joint — that is the "growing out of the bar" look. A
strip meeting another strip stays square so the two read as one run. A strip
ending at a bare screen corner rounds its outer corner. Inner corners stay
square: the frame is flat 2D paint, not a signed-distance field, so there is
no gooey merge where a panel meets the ring.

The frame is painted in the bar's background colour at the bar's opacity, on
the strip's own layer. The surface takes no pointer input: clicks pass
through to the desktop underneath, exactly like the tooltip surface.

## Honest limits

- The screen size the geometry is computed from is the last one the
  compositor reported, shared by every output — the same limitation the menu
  placement already lives with. On mixed-size setups the frame follows the
  last measured screen; per-output sizes are a follow-up.
- The frame does not collapse over fullscreen windows yet: there is no
  fullscreen signal in the core services to drive it. A follow-up wires one
  through and fades the strips out with it.
- The frame snaps to a new theme with the target instead of morphing: it
  lives on a surface of its own, and resizing that surface mid-transition
  would fight the palette travelling on the bar.
