# hydebar Roadmap

**Goal:** Build the **fastest** and **most beautiful** Wayland panel for Hyprland.

**Vision:** Lighter and faster than Waybar, richer and more polished than HyprPanel.

---

## 🎯 Core Principles

1. **⚡ Blazing Fast** - events over polling, sub-1% idle CPU, ~50ms startup
2. **🎨 Beautiful** - Preset themes, HyDE theme following, smooth animations
3. **🛠️ Easy to Configure** - Settings window, hot-reload, sensible defaults
4. **🔧 Extensible** - Custom modules, plugin system (future)
5. **100% Rust** - Memory safe, zero-cost abstractions

---

## 📊 Current State

### ✅ Implemented Features

**Core Modules:**
- Workspaces (Hyprland integration)
- Window title
- Clock, with alternative formats, calendar menu and optional weather readout
- System info (CPU, RAM, temperatures, GPU, disk, network speeds; auto-detected readouts)
- Battery
- Network (WiFi, VPN, connections)
- Bluetooth
- Audio (volume, sink/source control)
- Brightness
- Media player (MPRIS)
- Tray
- Updates
- Privacy indicators (camera/mic/screenshare)
- Keyboard layout/submap
- Clipboard
- App launcher
- Notification center with selectable source (built-in popups / Hyprland / session daemon)
- Screenshot and screen recording (grim/slurp/wf-recorder)
- Idle inhibitor
- Control center (quick settings + power menu)
- Settings window (bar layout and appearance, written back to the config)
- Themes, wallpaper and HyDE menu modules driving the HyDE desktop
- Custom modules (Waybar-style: streams, `exec`+`interval`, signals, context menus)

**Visual:**
- 11 preset themes
- HyDE theme following (colours, font, radius) with live repaint on theme switch
- Animations (menu fade, hover) with configurable durations
- Islands / Solid / Gradient styles, opacity control, auto-scale per output

**Technical:**
- Multi-window support (multi-monitor)
- Wayland-native (layer-shell)
- Event-driven architecture with redraw coalescing
- Config hot-reload; theme watcher for HyDE state
- Single-instance takeover; child process supervision and orphan reaping

### 🔧 Current Limitations

- Hyprland-only for workspaces, window title, keyboard layout
- TOML configuration; the settings window covers layout and appearance, not every option
- No vertical panel mode, no auto-hide
- No plugin system (custom modules are external commands)

---

## 🗓️ Development Phases

The first three phases — visual polish (preset themes, animations),
performance (measured baseline, startup and idle work, process hygiene) and
enhanced features (notification center, screenshot/recording, inline
controls, weather and calendar) — are **delivered**. What remains:

## Phase: User Experience 🎯

- ⚙️ **Settings window growth** - more of the configuration editable from the bar
- 📚 **Documentation** - configuration reference, module reference, website

## Phase: Extra Features 🌟

### Future Ideas
- Plugin system (Lua/WASM)
- Multiple panel support (top + bottom)
- Vertical panel mode
- Panel auto-hide
- Gesture controls
- Compositor-agnostic mode (feature flags for the Hyprland-only modules)

---

## 🎯 Success Metrics

### Performance (measured, see [docs/perf-baseline-2026-07.md](docs/perf-baseline-2026-07.md))
- ✅ **Fast startup:** ~53ms to mapped surface (Waybar: ~100ms)
- ✅ **Low idle CPU:** ~0.5% (Waybar: ~2%)
- ⚠️ **Memory:** ~127MB resident — dominated by the GPU rendering stack, not the modules

### Features (vs HyprPanel)
- ✅ Core feature parity, including notifications, screenshot, weather, calendar
- ✅ Better performance (Rust vs TypeScript/GTK)
- ✅ Deep HyDE integration (theme following, session bar, HyDE menu)

### Adoption
- 🎯 100+ GitHub stars
- 🎯 10+ contributors
- 🎯 Featured in Hyprland showcase
- ✅ AUR package
- 🎯 Mentioned in r/hyprland

---

## 📋 Prioritization Framework

**Priority Levels:**

1. **CRITICAL** - Blocks release, major differentiator
2. **HIGH** - Important for UX, high impact
3. **MEDIUM** - Nice to have, improves experience
4. **LOW** - Future enhancement

**Current Priorities:**
1. Documentation (HIGH)
2. Settings window growth (MEDIUM)
3. Compositor-agnostic mode (LOW)

---

## 🤝 Contributing

Want to help? Check out:
- Issues labeled `good first issue`
- Issues with detailed implementation plans
- Our [Contributing Guide](docs/CONTRIBUTING.md)

**High-impact, beginner-friendly:**
- Individual theme implementations
- Documentation improvements
- Testing and bug reports

---

## 📞 Feedback

Have ideas? Open an issue or discussion!

- 🐛 Bugs: [Issues](https://github.com/RAprogramm/hydebar/issues)
- 💡 Feature requests: [Discussions](https://github.com/RAprogramm/hydebar/discussions)

---

**Status:** Active development 🚧
