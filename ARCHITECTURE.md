# HydeBar Architecture

**Event-Driven Architecture for Hyprland**

## Philosophy

> Modules own their data, logic and rendering.
> The GUI layer wires modules to the bar: registration, dispatch, layout.
> Communication goes through the event bus.

## Layer Structure

```
┌──────────────────────────────────────┐
│         hydebar-proto                │
│  - Config schema (config/)           │
│  - Ports (ports/, e.g. HyprlandPort) │
│  - HyDE theme source (theme_source/, │
│    hyde_state/, hyde_dirs)           │
└──────────────┬───────────────────────┘
               │
┌─────────────────────────────────────┐
│         hydebar-core                 │
│  ┌────────────────────────────────┐  │
│  │ Modules (modules/)             │  │
│  │  - state, update and iced view │  │
│  │  - implement the Module trait  │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │ Event Bus (event_bus.rs)       │  │
│  │  - BusEvent: Redraw,           │  │
│  │    PopupToggle, Module(...)    │  │
│  │  - coalesces at enqueue time   │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │ Services (services/)           │  │
│  │  - D-Bus, PipeWire, Wayland,   │  │
│  │    compositor adapters         │  │
│  └────────────────────────────────┘  │
└──────────────┬───────────────────────┘
               │
┌─────────────────────────────────────┐
│         hydebar-gui                  │
│  ┌────────────────────────────────┐  │
│  │ App (app/)                     │  │
│  │  - registration, update,       │  │
│  │    view composition, bus pump  │  │
│  └────────────────────────────────┘  │
└──────────────┬───────────────────────┘
               │
┌─────────────────────────────────────┐
│         hydebar-app                  │
│  - Main entry point                  │
│  - Runtime, logging, instance lock,  │
│    process reaper, wiring            │
└──────────────────────────────────────┘
```

The four crates live at the workspace root: `proto/` (no dependency on the
widget toolkit at all — colours are stated as `theme_source::Rgba` and read as
the renderer's own only in `core/src/style/color.rs`), `core/` (modules,
services, event bus), `gui/` (the `App` state machine and bar composition) and
`app/` (the binary). Every item `proto` exports is documented: the crate warns
on `missing_docs`, so a setting cannot be added without saying what it means.

## Module Design Pattern

A module lives in `core/src/modules/` and implements the `Module` trait from
`core/src/modules.rs`:

```rust
pub trait Module<Message> {
    type ViewData<'a>;
    type RegistrationData<'a>;

    fn register(&mut self, ctx: &ModuleContext, data: Self::RegistrationData<'_>)
        -> Result<(), ModuleError>;
    fn deregister(&mut self);
    fn poll_schedule(&self) -> Option<PollSchedule>;
    fn poll(&mut self, ctx: &ModuleContext) -> Result<(), ModuleError>;
    fn view(&self, data: Self::ViewData<'_>)
        -> Option<(iced::Element<'static, Message>, Option<OnModulePress<Message>>)>;
    fn subscription(&self) -> Option<iced::Subscription<Message>>;
}
```

- `register` starts the module's background work — pollers, D-Bus listeners,
  spawned commands — through the shared `ModuleContext`, which carries the
  event sender and the runtime handle.
- `deregister` releases that work when the layout no longer hosts the module.
- `poll_schedule`/`poll` let a module declare a sampling cadence instead of
  owning a timer; the bar keeps one clock for modules at rest and one for the
  module being attended.
- `view` renders the bar entry and names the press action.

### One file, 150 lines

A source file holds at most **150 lines of code**. Doc comments, doctests and
the file's own `#[cfg(test)] mod tests` do not count against the limit — a
file is measured by the code that runs in production, and tests are expected
to be longer than what they cover.

A module is therefore one file in `modules/` only while its code fits in 150
lines — `cpu.rs`, `memory.rs` and `idle_inhibitor.rs` are that size. Anything
larger becomes a directory named after the module, one file per
responsibility: `media_player/` splits into `state`, `messages`, `commands`
and `view`, and a submodule that outgrows its own file nests further —
`system_info/` is the worked example, with `sensors/` and `window/` inside
it. The path a caller imports never changes when a file becomes a directory.

### Where module design is heading

The trait is a transitional shape. The target is the battery pattern: data
and update logic live in core, rendering is a plain function the GUI's
dispatch calls with the data it owns, and background work is started by
`register`-style methods only where a module genuinely owns any. A module
with no state of its own is nothing but render functions — `cpu`, `memory`,
`cpu_temp`, `gpu_temp` and `idle_inhibitor` are the worked examples; a
stateful module keeps its state in core without the trait — `battery` and
`calendar` show that shape.

Migration ledger — still on the trait: `bar_layout`, `clock`,
`command_button`, `control_center`, `custom_module`, `hyde_menu`,
`keyboard_layout`, `keyboard_submap`, `media_player`, `notifications`,
`privacy`, `screenshot`, `settings`, `system_info`, `taskbar`, `themes`,
`tray`, `updates`, `wallpaper`, `weather`, `window_title`, `workspaces`. A
migrated module leaves this list in the commit that moves it.

### Registration: one law

`gui/src/app/update/registration.rs` states the single rule: a module is
wired to the event bus while the layout hosts it, and released the moment it
is not. `Modules::hosts` (from `proto/src/config/modules.rs`) answers "is
this drawn anywhere", groups included, so an unused module never wakes the
runtime.

Some bar entries share one worker: the control-center services feed the
standalone `Audio`, `Network`, `Bluetooth` and `PowerProfile` readouts, the
`Battery` menu and the `Settings` window alike, so the connections stay alive
while at least one of them is on screen.

## Event Flow

```
User clicks → GUI dispatches Message → App updates the module →
Module publishes through ModuleContext → Event Bus (coalesced) →
App drains the bus → re-render
```

Background sources follow the same path: a service listener publishes a
`ModuleEvent`, the queue coalesces at enqueue time — a snapshot replaces its
stale twin, a duplicate redraw folds into the tail — and the GUI picks up
whatever accumulated while it was busy as one batch. The first event of a
burst is delivered immediately; no grace window taxes a user click.

## Benefits

### Clean Separation
- `hydebar-proto` holds the schema and shared protocol types; it never depends
  on the higher layers, and never on the widget toolkit.
- Modules own their behaviour end to end; the GUI layer only wires and
  composes them.

### No Circular Dependencies
- Dependencies point one way: app → gui → core → proto.

### Modularity
- Adding a module means one entry in the `ModuleName` enum
  (`proto/src/config/modules.rs`), the module itself in core, and its wiring
  in the GUI registration and dispatch.

### Performance
- Event-driven updates: the bar sleeps until a bus, socket or file watch wakes
  it (see `docs/data-sources.md`).
- Redraw coalescing keeps the repaint rate below the event rate.
- iced GPU rendering.

### Maintainability
- Clear responsibility boundaries per crate.
- Services are testable behind trait seams; modules carry their own tests.

## Example

The battery module is a complete worked example, and it is already split the
way the size limit asks: `core/src/modules/battery/` holds `data`, `state`
and `view` in a file each, backed by the UPower service in
`core/src/services/upower/`, and the GUI only dispatches to it from
`gui/src/app/modules/dispatch/view.rs`.
