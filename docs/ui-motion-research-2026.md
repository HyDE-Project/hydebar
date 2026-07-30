# UI motion and visual polish — research, analysis, plan (July 2026)

Research into current best practice for interface motion and visual depth,
an honest audit of where hydebar stands, and a staged plan for closing the
gap. Written against the state of the tree at the end of July 2026.

## 1. What the field considers right in mid-2026

### Motion

- **Springs for interaction, easing for decoration.** Physics-based springs
  are the norm for anything the user touches — buttons, menus, sheets —
  because interrupted motion retargets naturally instead of restarting.
  Pre-baked easing curves remain right for one-shot decorative moves.
- **Bounce budget.** For productivity surfaces the overshoot ("bounce")
  parameter stays under ~0.1; anything springier reads as a toy. A bar is
  the most utilitarian surface on the desktop — it sits at the low end.
- **Duration budget.** Everything interactive answers within ~100 ms
  (immediate pressed state) and completes within ~300 ms. Long fades on
  small elements read as lag, not as polish.
- **Enter fast-out, exit fast-in.** Entrances decelerate (ease-out feel),
  exits accelerate. Springs give this for free when tuned by "response"
  rather than raw duration.
- **Motion is language, not decoration.** Movement exists to communicate
  cause and effect: a menu grows out of the module that owns it, a value
  change flashes where the value lives. Motion that cannot answer "what is
  it telling the user?" is cut.
- **Reduced motion is a hard requirement.** Desktop platforms expose an
  OS-level reduced-motion preference and WCAG 2.3.3 expects it honoured.
  The accepted approach: replace movement with short opacity fades rather
  than deleting feedback entirely.

### Visual depth

- **Glassmorphism matured.** Translucent layers with backdrop blur are a
  foundational technique now, not a trend; 2026 usage favours *subtle*
  translucency, thin gradient borders, soft shadows, and a fine noise/grain
  texture over the glass to avoid the "flat plastic" look.
- **Blur belongs to the compositor on Wayland.** Bars and shells that look
  native on Hyprland get their glass from compositor blur layer rules, not
  from re-implementing blur in the toolkit. That keeps the effect coherent
  with the rest of the desktop and costs the bar nothing per frame.
- **Shaders are entering shell UI, sparingly.** The reference points among
  Wayland shells (Noctalia and the Quickshell family, HyprPanel-style
  bars) ship smooth animated bars with rounded, breathing surfaces; actual
  fragment-shader work appears only where the toolkit cannot express the
  effect — refraction, grain, glow — and always behind a switch.

### The peer group

The bars people rate for feel in 2026 — Noctalia ("smooth animations" is a
headline feature), HyprPanel-inspired Quickshell bars — share three habits:
menus that scale-and-fade from their anchor, hover states that transition
rather than flip, and one coherent radius/spacing/typography scale driven by
the theme. Nothing about their look requires a custom renderer; it requires
discipline about where motion happens.

## 2. Where hydebar stands today

What exists and is genuinely solid:

- **A real spring core.** `animation::Spring` with response-based tuning,
  settle detection, retargeting; used by the theme transition
  (`style/transition.rs`, response 220 ms) and by menu opacity
  (`menu/state.rs`). This matches the "springs for interaction" doctrine.
- **A gated frame clock.** The 16 ms ticker runs only while something
  animates; a settled bar draws nothing. Better hygiene than most peers.
- **Theme-driven geometry.** One radius, spacing and type scale derived
  from the themed font size and screen magnification; glass comes from
  Hyprland blur rules attached to per-surface namespaces, restated across
  theme switches. This is the correct Wayland-native division of labour.
- **A config seam already shaped for motion.** `AnimationConfig` carries
  `enabled`, `menu_fade_duration_ms`, `hover_duration_ms`.

Where we fall short of the 2026 bar:

1. **Menus only fade.** Opacity is the sole animated channel. The accepted
   pattern is scale-plus-fade from the anchor (about 0.96 → 1.0) so the
   window reads as growing out of its module. We have the anchor position
   already (`ButtonUIRef`) and a spring on opacity; the scale channel is
   simply not wired.
2. **Hover states flip instantly.** `hover_duration_ms` exists in config
   but button styles switch colour in one frame. Every peer transitions
   hover backgrounds. iced 0.14 style closures receive only the status, so
   this needs a small per-module hover spring (or the widget-level
   animation support landed in iced 0.14) driven by the existing ticker.
3. **No reduced-motion path.** `enabled = false` kills all animation dead
   (feedback included), and nothing reads the desktop's reduced-motion
   preference (the freedesktop appearance portal exposes it). The right
   shape: three-state motion setting — full / reduced (fades only, short) /
   off — defaulting from the portal.
4. **Value changes teleport.** Volume percent, workspace switches, update
   counts, calendar month changes swap content with no transition. Peers
   crossfade or slide numeric changes; a 120–150 ms crossfade is enough.
5. **Submenu unfold snaps.** The control-centre submenus and the HyDE menu
   tree expand instantly to full height. A height spring (or clip-reveal)
   in the 200 ms class is the expected behaviour.
6. **No grain, no gradient edge on glass.** Our menu boxes are flat fills
   over compositor blur. The 2026 glass look adds a ~2–3% noise texture and
   a one-pixel gradient border to kill banding and lift the panel. Noise is
   a one-off tiled image or a trivial fragment shader; the border is plain
   styling.
7. **Workspace focus jump.** The active-workspace pill switches place
   instantly. The signature move of the peer group is the indicator sliding
   between workspace slots — one spring on the highlight's x-position.

On shaders specifically: iced 0.14 ships a first-class `shader` widget with
a wgpu `Program` API, and we already run the wgpu renderer, so custom WGSL
is available to us without architectural change. The honest assessment is
that nothing in the gap list *needs* one except the grain texture (and even
that can be a baked image); refraction/caustics-grade effects would fight
the compositor's blur rather than compose with it. Shaders stay in the plan
as a bounded, optional layer, not a foundation.

## 3. Plan

Ordered so every step ships alone, each behind the existing animation
config, all durations derived from one motion scale.

1. **Motion tokens.** One place stating the three springs the bar uses:
   `snappy` (~150 ms response — hover, pressed), `standard` (~220 ms —
   menus, unfolds; the theme transition already lives here) and `gentle`
   (~300 ms — theme, layout). Config keeps overriding durations; the
   tokens replace scattered constants.
2. **Menu scale-plus-fade.** Add a scale channel next to the existing menu
   opacity spring; transform origin at the anchor point already carried by
   the wrapper. Exit accelerates (spring retarget to 0.97/0.0).
3. **Hover transitions.** Per-module hover spring fed by the existing
   `ModuleHover` events and ticked by the existing frame clock, blending
   the two button style states; honours `hover_duration_ms`.
4. **Reduced motion.** Extend `animations.enabled` to a three-state
   `motion = "full" | "reduced" | "off"` (serde keeps `true`/`false`
   working), read the portal's reduced-motion preference as the default,
   and map "reduced" to fades-only at 100 ms.
5. **Submenu unfold.** Height spring on `sub_menu_wrapper` and the HyDE
   menu branches, standard token, clip during flight.
6. **Value crossfades.** A small `animated_text` helper crossfading when
   its string changes (volume, updates count, keyboard layout, clock
   minute); snappy token, opacity only — safe under reduced motion.
7. **Workspace slide.** Animate the active pill's position between slots
   with the standard token; under reduced motion, fall back to the current
   instant switch.
8. **Glass finish.** One-pixel gradient border + tiled noise at ~2%
   opacity on menu and island surfaces, stated once in the container
   styles; purely static, no per-frame cost.
9. **Shader layer (optional, last).** A single `shader` widget behind
   `appearance.effects` for the one thing styling cannot do: animated
   grain / subtle glow on the active island. Off by default, absent under
   reduced motion, and dropped entirely if it costs more than a milliseconds
   budget of one frame in measurement.

Non-goals, decided now: no parallax, no bounce above 0.1 equivalent, no
blur re-implementation inside the toolkit, no motion on the bar strip
itself outside the channels above — the bar is furniture, not a character.

## 4. Sources

- [Understanding Spring Animation (2026 edition) — physics-based motion guide](https://medium.com/codetodeploy/understanding-spring-animation-in-swiftui-2026-edition-the-complete-guide-to-physics-based-4835b7c2b095)
- [Web Animation Best Practices & Guidelines](https://gist.github.com/uxderrick/07b81ca63932865ef1a7dc94fbe07838)
- [UI Animation Trends & UX Design, 2026 practical guide](https://www.ripplix.com/blog/ui-animation-practical-guide-for-2026)
- [Framer Academy — transitions and easing](https://www.framer.com/academy/lessons/framer-animations-transitions-and-easing)
- [iced 0.14 release notes](https://github.com/iced-rs/iced/releases/tag/0.14.0)
- [iced shader widget documentation](https://docs.iced.rs/iced/widget/shader/struct.Shader.html)
- [Minimal iced fragment-shader widget example](https://github.com/w23/iced-fragment-shader-widget-example)
- [Noctalia — quiet by design (Quickshell shell)](https://github.com/quickshill-place/Noctalia)
- [HyprQuick — HyprPanel-inspired Quickshell bar](https://github.com/x86kernel/HyprQuick)
- [Hyprland wiki — status bars](https://wiki.hypr.land/Useful-Utilities/Status-Bars/)
- [MDN — prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion)
- [Atomic Accessibility — animation & motion WCAG checklist (2026)](https://www.atomica11y.com/accessible-design/animation/)
- [Glassmorphism in 2026 — matured usage guide](https://invernessdesignstudio.com/glassmorphism-what-it-is-and-how-to-use-it-in-2026)
- [Dark glassmorphism — the 2026 aesthetic](https://medium.com/@developer_89726/dark-glassmorphism-the-aesthetic-that-will-define-ui-in-2026-93aa4153088f)
- [UI design trends 2026 — full guide](https://midrocket.com/en/guides/ui-design-trends-2026/)
