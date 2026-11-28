# iced_layershell

> **Warning**: This library is not intended for general use. It is specifically built for [ashell](https://github.com/MalpenZibo/ashell) and will not accept feature requests or bug reports unrelated to ashell. If you need a general-purpose iced layer shell backend, consider [iced-layershell](https://github.com/waycrate/exwlshelleventloop) or the [pop-os/iced](https://github.com/pop-os/iced) fork.

Wayland [layer shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) backend for [iced](https://github.com/iced-rs/iced) 0.14. Built to power [ashell](https://github.com/MalpenZibo/ashell), a Wayland status bar.

## AI-assisted development

This library was developed with the assistance of AI coding agents. The architecture, implementation, code reviews, and documentation were produced through human-AI collaboration. All code has been reviewed and tested by the maintainer.

## What this is

A thin bridge between iced's widget/rendering system and the Wayland layer shell protocol via [smithay-client-toolkit](https://github.com/Smithay/client-toolkit). It replaces `iced_winit` for applications that need layer shell surfaces (panels, overlays, status bars) instead of regular windows.

This library is **tailored for ashell**. It implements exactly the features ashell needs and nothing more. It is not a general-purpose iced backend and does not aim to support every layer shell use case.

## Design goals

- **Zero idle CPU** -- the event loop blocks when nothing happens. No polling, no busy loops.
- **Standard iced** -- works with upstream iced 0.14 releases, no fork required.
- **Frame-synced rendering** -- uses Wayland frame callbacks to prevent overrendering.
- **Multi-surface** -- supports multiple layer surfaces (e.g. status bar + dropdown overlay).

## Features

- Layer shell surface management (create, destroy, configure anchor/layer/size/margin/exclusive zone)
- Keyboard input with client-side repeat and physical key mapping, pointer, touch, and scroll events
- Full iced runtime action handling (clipboard read/write, widget focus, font loading)
- HiDPI support with configurable application scale factor
- Output (monitor) tracking with connect/disconnect subscriptions
- Persistent widget UIs with iced's `UserInterface` caching

## Usage

```rust
use iced_layershell::*;

fn main() -> Result<(), Error> {
    application(boot, update, view)
        .layer_shell(LayerShellSettings {
            anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            layer: Layer::Top,
            exclusive_zone: 40,
            size: Some((0, 40)),
            ..Default::default()
        })
        .subscription(subscription)
        .theme(|state| state.theme.clone())
        .run()
}
```

See [`examples/`](examples/) for working demos.

## What is NOT supported

Features that ashell doesn't need are intentionally omitted:

- Drag and drop
- Popups / xdg-popup
- Session lock surfaces
- Subsurfaces
- Window actions (minimize, maximize, resize, etc.)

