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

- [x] **Tray icons decode and rasterize on the shared runtime with no
  cache.** Named icons are memoised under a theme-and-name key — a HyDE
  theme switch changes the key, so the cache refreshes itself — the theme
  answer itself is held for five seconds, and every decode or
  rasterisation runs on the blocking pool instead of the shared runtime.
- [x] **The system-info window model is built twice per frame.** Each
  window now comes back as one build carrying its own height: the monitor
  states its model once and both the drawing and the measurement read it,
  and every standalone entry builds only its own section through its own
  constructor instead of building the whole machine and keeping one row
  group.
- [x] **Themes menu re-normalises names quadratically, twice per frame.**
  The offered list and a canonical catalogue index are restated only when
  the installed set or the catalogue moves; a frame now costs one small
  lookup per chip instead of thousands of normalised comparisons.
- [ ] **The frame clock is a fixed 16 ms timer.** Blocked upstream: the
  layer-shell fork synthesizes its redraw event only into the widget
  update, never into the subscription stream, so the runtime's
  frame subscription can never fire under it. Revisit when the fork
  forwards window events to subscriptions.
- [x] **The faded theme allocates a name, an arc and a full palette blend
  per surface per animated frame.** Fade shares snap to a sixty-fourth and
  every derived theme lands in one app-level memo cleared per frame — one
  palette blend serves every island and menu on the same step of the
  wave.
- [x] **The wallpaper picker re-lists and re-decodes every thumbnail on
  every toggle, including the closing one.** The loads fire only on the
  opening toggle now, and decoded thumbnails ride along by path — a
  reopened picker decodes nothing, a theme switch decodes only the
  pictures it brought. The theme swatch and catalogue loads carry the
  same opening gate.
- [x] **The theme subscription re-walks the environment on every
  re-evaluation.** The roots and targets are derived once per process —
  the environment cannot change under a running one — while the follow
  flag stays live so a reload can still turn the watch off.
- [x] **Sampled system data is deep-cloned per publish and per bar frame.**
  The sample crosses the bus behind an `Arc`; the publish baseline is a
  reference count, and the oversized-variant expects on three message
  enums became unfulfilled and were removed — proof of the shrink.
- [x] **Small per-frame allocations that never stop.** The media title is
  composed once per service event, the window title is cut once per focus
  change, and the settings window builds its entries once per frame for
  the width, the height and the drawing together. The tray strip still
  clones each bus name per icon — two small strings per icon per animated
  frame, left as the cost of the owned menu message.
- [x] **Startup raises menu surfaces every frame for three seconds.** Each
  surface is raised exactly once; newcomers are checked for per frame but
  a raised surface costs no further compositor request.
- [x] **The HyDE menu re-reads and re-parses its definition files inline in
  update.** The reads and the parse run on the blocking pool and come back
  as a message; the opening animation no longer waits on the filesystem.
- [x] **Output bookkeeping grows on hotplug.** An untargeted add now
  replaces the same-name entry the way the targeted one always did, so a
  reconnect cycle leaves the roster the same size.

## Architecture

- [ ] **Adding a module means editing several parallel matches.** Three of
  them are gone: the module trait gained an object-safe shape
  (`modules/bar.rs`) whose view data takes itself out of one context, so
  subscriptions, sampling cadences and samples are all asked of the one
  owner lookup in `app/modules/dispatch/owner.rs` rather than each
  restating the roster. The view stays a table on purpose: every entry now
  draws through a method of its own, so the table is one line per module
  and nothing more. The desk panels, the hints and the registration
  rosters still enumerate their own.
- [x] **The GUI writes core module private state directly.** Menu-open
  preparation now goes through methods the modules own — `open_audio_menu`,
  `open_bluetooth_menu`, `close_submenu`, `refresh_brightness`,
  `collapse_submenus` — and nothing in `gui/` assigns to a core field.
- [x] **The domain crate knows the renderer.** `hydebar-proto` no longer
  depends on the widget toolkit at all: the appearance schema states colours
  as `theme_source::Rgba` and hands the base, text, weak and strong shades to
  the caller, and `core/src/style/color.rs` is the one place that reads them
  as the renderer's colour and builds the readable pairs. The filesystem half
  stands deliberately: the `HyDE` theme source is what the crate is chartered
  to hold, every read is behind a named function and covered by tests against
  a temporary directory, so a port would be plumbing without a second
  implementation to justify it.
- [ ] **Config failures collapse into silence.** Numeric appearance values
  are now range-checked with named refusals — a zero scale, a negative
  font or a twelvefold opacity is rejected with the field and the allowed
  range, and the reload keeps the last valid configuration. Still open:
  layout and theme reads folding errors into `None`, and unknown keys
  passing unnoticed.
- [x] **Errors are strings at heart.** One hundred and sixty of the sites
  now go through `services/bus.rs`, which reads the D-Bus error name the peer
  raised and answers with the kind it stands for — a refusal, an absence, a
  silence, an unreachable bus — and the rest carry the kind their own failure
  names (a missing key is a configuration error, a rate limit is a rate
  limit, a `PulseAudio` server that will not start is unavailable).
  `NetworkServiceError` carries the kind through to the service and prints it
  beside the message, so a wrong password and a vanished adapter no longer
  read the same in the journal. Thirty-two sites remain internal, and each is
  a genuine programming-level failure — a value that would not convert, a
  channel that is gone.
- [x] **Two modules render outside core.** The battery now draws itself
  through its module; the tray strip stays in the bar layer by documented
  necessity — its per-icon press carries a positioned menu reference no
  message-generic view can construct. The font-size fallback speaks
  through the one shared derivation everywhere.
- [x] **Convention debt.** The updates state moved its tests into the
  folder and stands under the line again; the rotted broken-tests feature
  and its six blocks — none of which even compiled — are gone; the config
  crossing is a named export list; the two bus enums dropped
  `#[non_exhaustive]`, so the mapping is total and a new variant is a
  compile error, not a silently dropped event. Menu view argument structs
  remain future taste work, each site carries its reasoned expectation.

## Standing constraints

The compositor-driven frame clock is blocked by the layer-shell fork, whose
redraw events never reach subscriptions. Revisit on its next release.

Typed compositor IPC no longer stands on anyone else's release: the bar
reads the compositor's own sockets — questions and commands through
`compositor_ipc`, announcements through `adapters/compositor/events.rs` —
and models only the fields it draws, so a record can carry whatever the
answer holds rather than whatever a general purpose crate chose to model.
