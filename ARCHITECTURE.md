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
│  │  - coalesces bursts (~8ms)     │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │ Services (services/)           │  │
│  │  - D-Bus, PipeWire, Wayland,   │  │
│  │    Hyprland adapters           │  │
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

The four crates live under `crates/`: `hydebar-proto` (no GUI dependencies
beyond shared types), `hydebar-core` (modules, services, event bus),
`hydebar-gui` (the `App` state machine and bar composition) and `hydebar-app`
(the binary).

## Module Design Pattern

A module lives in `crates/hydebar-core/src/modules/` and implements the
`Module` trait from `crates/hydebar-core/src/modules.rs`:

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

A module is one file in `modules/` while it fits in one; a mid-sized one is
organised with inline `mod` blocks inside that file (for example
`modules/media_player.rs` holds `state`, `messages`, `commands` and `view` as
inline submodules). A module that outgrows a readable file becomes a
directory named after it, one file per submodule, nested where a submodule
outgrows its own file — `modules/system_info/` is the worked example, with
`sensors/` and `window/` nested inside.

### Registration: one law

`crates/hydebar-gui/src/app/update/registration.rs` states the single rule: a
module is wired to the event bus while the layout hosts it, and released the
moment it is not. `Modules::hosts` (from
`crates/hydebar-proto/src/config/modules.rs`) answers "is this drawn
anywhere", groups included, so an unused module never wakes the runtime.

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
`ModuleEvent`, the bus coalesces bursts inside an ~8ms window, and the GUI
repaints once instead of once per event.

## Benefits

### Clean Separation
- `hydebar-proto` holds the schema and shared protocol types; it never depends
  on the higher layers.
- Modules own their behaviour end to end; the GUI layer only wires and
  composes them.

### No Circular Dependencies
- Dependencies point one way: app → gui → core → proto.

### Modularity
- Adding a module means one entry in the `ModuleName` enum
  (`crates/hydebar-proto/src/config/modules.rs`), the module itself in core,
  and its wiring in the GUI registration and dispatch.

### Performance
- Event-driven updates: the bar sleeps until a bus, socket or file watch wakes
  it (see `docs/data-sources.md`).
- Redraw coalescing keeps the repaint rate below the event rate.
- iced GPU rendering.

### Maintainability
- Clear responsibility boundaries per crate.
- Services are testable behind trait seams; modules carry their own tests.

## Example

The battery module is a complete worked example: data and logic in
`crates/hydebar-core/src/modules/battery.rs`, backed by the UPower service in
`crates/hydebar-core/src/services/upower/`, with a GUI view in
`crates/hydebar-gui/src/views/battery.rs`.
