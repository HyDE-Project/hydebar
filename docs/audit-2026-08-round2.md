# Deep audit, August 2026 — round two

A second full pass over the workspace: performance, concurrency, rendering
and architecture, measured against current Rust practice. Items are ranked
inside each section; a checked box means the fix has landed on `main`.

## Fixed on the spot

- [x] **Audio events were built and thrown away unheard.** The bounded-queue
  change left three sends in the PulseAudio callbacks constructing a future
  and dropping it unpolled, so `ServerInfo`, sink and source updates never
  left the backend. Replaced with non-blocking sends
  (`services/audio/backend/control.rs`).
- [x] **Pedantic and nursery lints now stand in the manifest.** Workspace
  `[lints]` table enforces `clippy::pedantic`, `clippy::nursery`,
  `missing_debug_implementations`, and denies `let_underscore_future` — the
  lint that would have caught the audio bug. The machine-applicable debt
  (~1 850 warnings) is paid; the manual remainder is being worked file set
  by file set.

## Concurrency and reliability

- [x] **Tray watcher task is detached and immortal.** The server now starts
  once and lives in a wrapper whose drop aborts the name-watching task; a
  torn-down tray module leaves neither a task nor a claimed name behind,
  and retries reuse the one connection instead of stacking servers.
- [x] **Interface write guard held across signal emission.** The guard is
  dropped before the unregistration signal is emitted, so a slow peer can
  no longer stall registrations behind the emission.
- [x] **PulseAudio listener thread leaks on every re-registration.** A
  recurring heartbeat timer on the sound server's own loop wakes the parked
  listener every 300 ms; the loop checks whether its event channel is
  closed — which is exactly what dropping the handle does — and leaves,
  disconnecting from the server. Verified live: three config reloads,
  thread count and sound-server client count both flat.
- [x] **A transient full queue permanently kills custom modules and the media
  player.** Solved at the root: the bus is now infallible — an overflowing
  queue first evicts its oldest replaceable snapshot (lossless, a fresher
  one exists), then the oldest event, and a poisoned lock recovers because
  the queue holds no cross-operation invariant. The queue-full error class
  no longer exists, so no producer can die of it and the UI subscription
  cannot terminate.
- [x] **Four services wait the maximum 60 s before their first retry.**
  bluetooth, brightness, upower and the tray watcher now count failures in
  their driver loops and sleep the graded `reconnect_delay(failures)`, like
  network and privacy always did.
- [x] **Notifications service never retries.** The server lifecycle now runs
  inside the standard retry loop with the graded delay: every refusal — bus
  not up yet, interface registration, a holder that will not yield — ends
  one attempt instead of the whole daemon, and the desk reopens on the next
  knock.
- [x] **Tray rebuild window loses registrations.** The registration signals
  are subscribed exactly once and per-item streams join the running merge
  as items register; nothing is torn down between registrations, so a
  login-time burst of tray applications loses nobody.
- [x] **No timeout on tray D-Bus calls.** The item handshake is bounded at
  five seconds; a frozen application costs the tray one skipped icon, and a
  single broken item no longer fails the whole initialisation.
- [x] **`hyprctl` runs synchronously on the drawing thread.** The geometry
  question is asked from the blocking pool and comes back as its own
  message; the drawing thread only adopts the answer. The cache lock now
  recovers from poisoning instead of panicking the bar.
- [x] **Multiplexer singleton can initialise without a supervisor.** The
  singleton is published only after its supervisor is running; a failed
  start leaves the cell empty and hands out streams that end at once, so
  every subscriber's retry loop re-knocks instead of hanging. A later
  caller whose configuration differs from the one in force is told so.
- [x] **Listener runtime can be built twice.** A build gate serializes the
  fallible construction with a double check, so racing first callers build
  exactly one runtime.
- [x] **Exit is a fixed 200 ms timer.** The process now exits on a
  confirmation message chained behind the surface-destroy tasks; a two
  second backstop covers a stalled runtime. Verified live: the successor
  takes the lock right after the destroys complete, inside the takeover
  window.
- [x] **Bus overflow drops the newest events; poisoning ends the
  subscription for good.** Folded into the infallible-bus rework above: the
  newest event always lands, eviction prefers stale snapshots, poisoning
  recovers, and the subscription lives as long as the bar.

## Performance

- [ ] **Tray icons decode and rasterize on the shared runtime with no
  cache.** Theme lookup reads GTK settings per call; SVGs re-render and
  rasters re-decode on every change signal from chatty applications
  (`services/tray/icon.rs`, `watcher.rs:150`). Spawn-blocking plus a
  name-keyed memo.
- [ ] **The system-info window model is built twice per frame** — once to
  measure, once to render — and the standalone processor/memory/temperature
  windows build the *entire* model four times to keep one section each
  (`app/view.rs:246-307`, `system_info/window/model.rs:104`). Build once,
  measure the built model, add per-section constructors.
- [ ] **Themes menu re-normalises names quadratically, twice per frame.**
  Offered-name filtering canonicalises both sides of every comparison —
  thousands of small strings per frame with a large gallery
  (`themes/view.rs:163-237`). Precompute canonical keys when the catalogue
  or installed set changes.
- [ ] **The frame clock is a fixed 16 ms timer.** It beats against any
  refresh rate that is not 62.5 Hz; every animation rides it
  (`app/update/subscriptions.rs:120`). Drive it from the compositor's
  redraw callback.
- [ ] **The faded theme allocates a name, an arc and a full palette blend
  per surface per animated frame**, and the sweep memo lives per section
  instead of per frame (`style/theme.rs:16`, `app/view.rs:53`,
  `app/modules/section.rs:131`). Quantise the share, memoise on the app.
- [ ] **The wallpaper picker re-lists and re-decodes every thumbnail on
  every toggle, including the closing one** — full texture-cache miss each
  open, subprocess each close (`app/update/menus.rs:105`,
  `modules/wallpaper.rs:63`). Cache entries; reload only when the theme's
  set changes. The swatch reload has the same missing gate
  (`app/update/menus.rs:107`).
- [ ] **The theme subscription re-walks the environment on every
  re-evaluation** — eight env lookups and a dozen path joins, dozens of
  times a second under traffic, for values that cannot change
  (`config/theme_watch/recipe.rs:200`). Cache the roots once.
- [ ] **Sampled system data is deep-cloned per publish and per bar frame**
  (`system_info/runtime.rs:166`, `system_info/view.rs:328`), and the sample
  rides the bus by value — the largest variant in three queues
  (`event_bus.rs`, `app/state.rs`). Share it behind an `Arc`.
- [ ] **Small per-frame allocations that never stop:** media-player title
  formats through three strings per repaint (`modules/media_player.rs:97`,
  `services/mpris/data.rs:77`), window title re-truncates per view
  (`modules/window_title.rs:66`), tray strip clones each bus name per icon
  (`views.rs:132`), settings window measures and renders the same page
  separately per frame (`settings/view.rs:98-152`).
- [ ] **Startup raises menu surfaces every frame for three seconds** —
  compositor round-trips per surface per frame during the greeting
  (`app/update/lifecycle.rs:39`). Raise once, re-raise on output events.
- [ ] **The HyDE menu re-reads and re-parses its definition files inline in
  update** when opening (`modules/hyde_menu.rs:62`). Move to a blocking
  task like the swatch loader.
- [ ] **Output bookkeeping grows on hotplug.** Removal re-pushes a
  placeholder and untargeted adds never replace by name; every frame then
  linear-scans the grown list (`outputs/state/lifecycle.rs:64-168`).

## Architecture

- [ ] **Adding a module means editing seven parallel matches.** Dispatch,
  registration, menu paging, actions, menus, mapping and the state struct
  each enumerate every module; window views are inherent methods invisible
  to the `Module` trait (`app/modules/dispatch.rs`, `app/view.rs:127`,
  `app/state.rs:83`). Extend the trait, hold modules behind it, delete the
  matches.
- [ ] **The GUI writes core module private state directly.** Menu-open
  preparation pokes submenu and brightness fields that are `pub` only for
  that purpose (`app/update/menus.rs:97-132`). One menu-opened hook per
  module owns the invariant.
- [ ] **The domain crate knows the renderer and the filesystem.** The proto
  crate pulls the GUI toolkit for one colour type and reads theme/layout
  files directly (`hydebar-proto/Cargo.toml`, `bar_layout.rs:54`,
  `theme_source/`). A domain colour type and a theme-source port belong
  there instead.
- [ ] **Config failures collapse into silence.** Layout and theme reads
  fold every error into `None`/defaults, numeric appearance values accept
  any number, and unknown keys pass unnoticed — a typo yields a silently
  wrong bar (`bar_layout.rs:54`, `config/validation.rs:53`,
  `appearance/settings.rs`). Validating newtypes and logged degradations.
- [ ] **Errors are strings at heart.** Nearly two hundred sites flatten
  typed failures into internal strings; callers cannot distinguish a gone
  device from refused auth (`services/network/backend/network_manager.rs`,
  `services/tray.rs`). Typed error enums at the service boundary.
- [ ] **Two modules render outside core** (battery, tray strip in
  `views.rs`) against the modules-own-their-rendering rule, and the
  font-size fallback is re-inlined six times in the settings view.
- [ ] **Convention debt:** the updates state file passed 1 000 lines;
  themes plus its view total ~1 900 for one module. The config glob
  re-export gives every proto type two import paths; two workspace-internal
  enums are `#[non_exhaustive]`, forcing wildcards that swallow new events
  at runtime; menu view functions take twelve to fifteen positional
  arguments; the broken-tests feature gates six rotted test blocks.

## Standing constraints

Typed Hyprland IPC remains blocked by the upstream crate (monitor data
lacks physical dimensions; the control colour type has no public
constructor). Revisit on the next release.
