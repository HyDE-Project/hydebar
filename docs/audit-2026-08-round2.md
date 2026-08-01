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

- [ ] **Tray watcher task is detached and immortal.** The name-owner stream
  task holds its own connection clone, never exits, and a new one is spawned
  on every `State::Init` re-entry — each tray reconnect leaks a connection
  still owning the watcher object (`services/tray/dbus.rs:68`,
  `services/tray/watcher.rs:250`). Store and abort the handle with the
  connection.
- [ ] **Interface write guard held across signal emission.**
  `interface.get_mut().await` stays alive while the unregistration signal is
  emitted on the same connection — self-deadlock-shaped under backpressure
  (`services/tray/dbus.rs:88`). Drop the guard before emitting.
- [ ] **PulseAudio listener thread leaks on every re-registration.** Dropping
  a `JoinHandle` detaches rather than aborts; the listener parks in
  `mainloop.iterate(true)` forever, and the control center respawns the
  service on every reload (`services/audio/backend/api.rs:52`,
  `threads.rs:123`). Signal `mainloop.quit()` from a `Drop` and join.
- [ ] **A transient full queue permanently kills custom modules and the media
  player.** `QueueFull` propagates as a terminal error out of the custom
  listener/poller and breaks the media-player supervisor loop
  (`modules/custom_module/listener.rs:59`, `poller.rs:129`,
  `modules/media_player.rs:176`). Warn and continue, as workspaces already
  does.
- [ ] **Four services wait the maximum 60 s before their first retry.**
  bluetooth, brightness, upower and the tray watcher sleep
  `RECONNECT_MAX_DELAY` instead of the graded `reconnect_delay(failures)` —
  a bar started before the daemons stays blank a full minute at login
  (`services/bluetooth.rs:198`, `services/brightness.rs:186`,
  `services/upower/events.rs:138`, `services/tray/watcher.rs:292`).
- [ ] **Notifications service never retries.** Six return paths end the
  daemon silently — after the bar may already have stopped the incumbent
  (`services/notifications/service.rs:116-234`). Wrap in the standard
  Init/Active/Error loop.
- [ ] **Tray rebuild window loses registrations.** Every registration tears
  down and rebuilds all item streams; items arriving during the rebuild are
  dropped, unregistrations missed (`services/tray/watcher.rs:270`).
  Subscribe to the registration signals once, add items incrementally.
- [ ] **No timeout on tray D-Bus calls.** One frozen tray application parks
  the whole tray forever (`services/tray.rs:81`, `watcher.rs:64`). Wrap item
  builds in a timeout and skip on expiry.
- [ ] **`hyprctl` runs synchronously on the drawing thread.** The geometry
  query blocks the UI for as long as the compositor takes, which is also
  what lets the event bus fill (`outputs/scaling.rs:145-172`); the cache
  lock also panics on poisoning. Move off the UI path; recover the lock.
- [ ] **Multiplexer singleton can initialise without a supervisor.** The
  `OnceLock` publishes before the runtime check; on failure subscribers hang
  forever with no log (`adapters/hyprland_client/listeners/multiplex.rs:41`).
  Its config is also whatever the first caller happened to hold.
- [ ] **Listener runtime can be built twice.** The fallible build happens
  outside the `OnceLock` initialiser; the losing runtime and its threads
  live on (`adapters/hyprland_client/listeners/runtime.rs:76`).
- [ ] **Exit is a fixed 200 ms timer.** `process::exit` fires whether or not
  surface destruction reached the compositor (`app/shutdown.rs:35`). Exit on
  a completion signal, timer as backstop.
- [ ] **Bus overflow drops the newest events; poisoning ends the
  subscription for good.** Only three message kinds coalesce; everything
  else races a 64-slot cap, and one poisoned lock stops all module events
  with a single log line (`event_bus.rs:121`, `app/bus.rs:66`).

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
