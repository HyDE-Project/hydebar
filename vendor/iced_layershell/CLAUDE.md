# CLAUDE.md — Project Guidelines for iced_layershell

## Project Purpose
Wayland layer shell backend for standard iced 0.14 (no fork). Replaces iced_winit for ashell (status bar for Hyprland/Niri). Must work with upstream iced releases.

## Design Principles

### Performance is paramount
- **Zero idle CPU**: The event loop must block (calloop dispatch with no timeout) when nothing happens. No polling, no busy loops, no unnecessary wakeups. Target: <0.5% CPU at idle.
- **Event-driven rendering**: Matching iced_winit's model — draw when events arrive, sleep when they don't. Each event triggers update+draw+present (same as upstream iced, since `view()` creates fresh widgets that can't diff visual state). The reactive optimization is in IDLE time: when no events arrive, the loop blocks indefinitely.
- **Frame-synced**: Use wayland frame callbacks to prevent overrendering. Never present faster than the compositor's refresh rate.
- **Minimal GPU work**: A status bar sitting idle should use effectively 0% CPU and 0% GPU. During mouse interaction, draws are cheap (~1440x40 surface).

### Minimal code
- This library exists to bridge iced and wayland layer shell. Keep it thin.
- Don't reimplement what SCTK or iced already provide.
- Don't add features ashell doesn't need. No DND, no popups, no session lock, no subsurfaces unless explicitly requested.
- Prefer simple solutions. Use smithay-clipboard (same as iced upstream) rather than reimplementing clipboard protocol.

### Follow iced's architecture
- Match iced_winit's event loop patterns: update loop until stable, then draw, then present.
- Inject `RedrawRequested` only when widgets request it, not every frame.
- Handle `RedrawRequest::NextFrame`, `At(instant)`, and `Wait` from widget state.
- Use `task::into_stream()` for iced Task execution via `iced_futures::Runtime`.

### Tailored for ashell
- Status bar use case: narrow surfaces, primarily Top/Overlay layers.
- Multi-surface: bar + menu overlay pattern (create on demand, destroy on close).
- HiDPI: physical pixels = surface-local * monitor_scale. Viewport uses monitor_scale * app_scale.
- Keyboard: OnDemand interactivity for text inputs, None for display-only bars.
- Clipboard: smithay-clipboard with worker thread (no deadlocks).
- Touch: support touch events for touchscreen devices.
- Cursor: text beam on text inputs only, default arrow everywhere else.

### Surface lifecycle (ashell-specific)
- **MAIN surface can be destroyed and recreated.** ashell manages its own surface lifecycle — it destroys the initial `SurfaceId::MAIN` fallback and creates new per-output surfaces when monitors are added. Do NOT exit the event loop when MAIN is closed.
- **Output-driven lifecycle**: surfaces are created/destroyed in response to `OutputEvent::Added`/`Removed`. When all monitors disconnect, a fallback surface is created.
- **Layer changes trigger full destruction/recreation** of both the main surface and its menu overlay. Other config changes (size, position, style) use in-place updates (`set_size`, `set_anchor`, etc.).

## Git Workflow
- **Always work on a dedicated branch**, never commit directly to `main`.
- **Branch names must match the auto-labeler patterns**: `feature/<desc>`, `fix/<desc>`, `chore/<desc>`, `docs/<desc>`.
- **Merge to main only through PRs** — this ensures CI runs and release-drafter picks up the changes.
- **Atomic commits**: each commit should be a single logical change that compiles and passes clippy. Don't bundle unrelated changes.
- Before pushing, always run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings`.

## Key Technical Decisions
- **RedrawRequested**: iced 0.14 widgets (buttons, etc.) only update visual status on `Window::RedrawRequested` events. This event must be injected when a redraw is actually needed — not on every frame.
- **Buffer scale**: Call `wl_surface.set_buffer_scale(monitor_scale)` so the compositor correctly maps surface-local coordinates to physical pixels.
- **Task wrapper**: Our `Task<M>` enum wraps iced's Task + LayerShellCommand. Free functions (`set_layer()`, `new_layer_surface()`) return our Task for API parity with the pop-os fork.
- **Clipboard**: smithay-clipboard must be created BEFORE `registry_queue_init` so its worker thread receives initial selection events.
