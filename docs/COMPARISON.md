# hydebar vs Waybar vs HyprPanel

Detailed comparison of Wayland panel solutions for Hyprland.

---

## Quick Comparison

| Feature | hydebar | Waybar | HyprPanel |
|---------|---------|--------|-----------|
| **Language** | Rust | C++ | TypeScript |
| **UI Framework** | iced (layer-shell) | GTK3 | GTK3 (Astal) |
| **Memory (idle)** | ~127MB* (GPU stack) | ~10MB | ~30MB |
| **CPU (idle)** | ~0.5%* | ~2% | ~3% |
| **Startup time** | ~53ms* | ~100ms | ~200ms |
| **Config format** | TOML | JSON | TypeScript |
| **Hot reload** |  Yes |  Partial |  Yes |
| **GUI config** |  Settings window (layout, appearance) |  No |  Yes |
| **Preset themes** |  11 themes + HyDE following |  No |  Yes |
| **Animations** |  Smooth |  Basic |  Smooth |
| **Wayland-native** |  Yes |  Yes |  Yes |
| **Multi-monitor** |  Yes |  Yes |  Yes |

\* Measured, release build — see [perf-baseline-2026-07.md](perf-baseline-2026-07.md).
The memory figure is the wgpu/Vulkan rendering stack, not the modules.

---

## Detailed Feature Comparison

### Core Modules

| Module | hydebar | Waybar | HyprPanel |
|--------|---------|--------|-----------|
| **Workspaces** |  Full |  Full |  Full |
| **Window title** |  Yes |  Yes |  Yes |
| **Clock** |  Yes |  Yes |  Yes |
| **Battery** |  Full |  Full |  Full |
| **Network** |  Full |  Full |  Full |
| **Bluetooth** |  Full |  Basic |  Full |
| **Audio** |  Full |  Full |  Full |
| **Brightness** |  Yes |  Basic |  Yes |
| **Media player** |  MPRIS |  MPRIS |  MPRIS |
| **System tray** |  Yes |  Yes |  Yes |
| **Updates** |  Yes |  Basic |  Yes |
| **Keyboard layout** |  Yes |  Yes |  Yes |
| **Privacy indicators** |  Yes |  No |  Basic |
| **Notifications** |  Yes (selectable source) |  Dunst |  Yes |
| **Weather** |  Yes (OpenWeatherMap) |  Basic |  Yes |
| **Calendar** |  Yes |  No |  Basic |

### Advanced Features

| Feature | hydebar | Waybar | HyprPanel |
|---------|---------|--------|-----------|
| **Custom modules** |  Yes (script, Waybar-style) |  Yes (Script) |  Yes (TS) |
| **Module ordering** |  Config + settings window |  Config |  GUI |
| **Inline controls** |  Yes (sliders) |  No |  Yes |
| **Screenshot tool** |  Yes (grim/wf-recorder) |  No |  Yes |
| **Power menu** |  Yes |  Basic |  Yes |
| **Clipboard history** |  Yes |  No |  Basic |

---

## Performance Comparison

Measured hydebar numbers come from
[perf-baseline-2026-07.md](perf-baseline-2026-07.md) (release build, 4K
output); the others are typical figures.

### Memory Usage

```
hydebar:   ~127MB resident — dominated by the wgpu/Vulkan rendering stack
Waybar:    ~10MB (GTK, CPU-rendered)
HyprPanel: ~30MB+ (TypeScript + GTK overhead)
```

**Winner:**  Waybar — hydebar pays for GPU rendering in resident memory

### CPU Usage

**Idle:**
```
hydebar:   ~0.5%
Waybar:    ~2%
HyprPanel: ~3%
```

**Active (menu opening and settling):**
```
hydebar:   ~0.5% — indistinguishable from idle
```

**Winner:**  hydebar

### Startup Time

```
hydebar:   ~53ms to mapped surface
Waybar:    ~100ms
HyprPanel: ~200ms (TypeScript compilation)
```

**Winner:**  hydebar

---

## User Experience

### Configuration

**hydebar:**
```toml
# Clean, typed TOML
appearance = "catppuccin-mocha"

[clock]
format = "%H:%M"
```

**Pros:**
- Type-safe
- Validation on load
- Hot reload
- Settings window for layout and appearance
- Script-driven custom modules (Waybar-style `exec`/`listen_cmd`)

**Cons:**
- Less flexible than full scripting

---

**Waybar:**
```json
{
  "modules-left": ["hyprland/workspaces"],
  "clock": {
    "format": "{:%H:%M}"
  }
}
```

**Pros:**
- Well-documented
- Large user base
- Script modules

**Cons:**
- JSON (no comments, strict)
- No hot reload (full)
- No GUI config
- Manual theming

---

**HyprPanel:**
```typescript
// TypeScript config
import { Config } from 'astal'

export default {
  theme: 'catppuccin-mocha',
  modules: {
    clock: { format: '%H:%M' }
  }
}
```

**Pros:**
- Full TypeScript power
- GUI config available
- Preset themes
- Hot reload

**Cons:**
- Requires TypeScript knowledge
- More complex setup
- Heavier runtime

---

## Theming

### hydebar

**Preset themes:**
- Catppuccin (Mocha, Macchiato, Frappe, Latte)
- Dracula
- Nord
- Gruvbox (Dark, Light)
- Tokyo Night (Night, Storm, Light)

**One line:**
```toml
appearance = "catppuccin-mocha"
```

Plus HyDE theme following: with no `appearance` at all the bar recolours
itself with the desktop on every HyDE theme switch.

**Winner:**  hydebar

### Waybar

**Theming:** Manual CSS
```css
/* style.css */
#window {
  background: #1e1e2e;
  color: #cdd6f4;
}
```

**Pros:**
- Full CSS control

**Cons:**
- Manual color management
- No preset themes
- Tedious for theme changes

### HyprPanel

**Preset themes:**  Yes
- Catppuccin
- Dracula
- Gruvbox
- Nord

---

## Development Experience

### Contributing

| Aspect | hydebar | Waybar | HyprPanel |
|--------|---------|--------|-----------|
| **Language** | Rust | C++ | TypeScript |
| **Learning curve** | Medium | High | Low |
| **Type safety** |  Strong |  Manual |  Strong |
| **Build time** | ~5min | ~2min | ~1min |
| **Hot reload** |  Yes |  No |  Yes |
| **Test coverage** |  Extensive |  Partial |  Partial |
| **Documentation** |  Growing |  Good |  Good |

**Best for contributors:**
- **Beginners:** HyprPanel (TypeScript)
- **Systems programmers:** hydebar (Rust)
- **C++ experts:** Waybar

---

## Stability & Maintenance

### hydebar
- **Status:** Active development 
- **Maturity:** Beta
- **Breaking changes:** Possible before v1.0.0
- **Community:** Growing
- **Updates:** Frequent

### Waybar
- **Status:** Mature, stable 
- **Maturity:** Production (v0.9+)
- **Breaking changes:** Rare
- **Community:** Large, active
- **Updates:** Regular

### HyprPanel
- **Status:** Active development 
- **Maturity:** Beta
- **Breaking changes:** Moderate
- **Community:** Growing
- **Updates:** Frequent

---

## Unique Selling Points

### hydebar 

**Why choose:**
1.  **Blazing fast** - ~53ms startup, ~0.5% idle CPU, event-driven
2.  **Memory safe** - Zero segfaults, data race free
3.  **Typed config** - Catch errors before runtime
4.  **Well tested** - Extensive test suite
5.  **Modern UX** - Preset themes, HyDE following, animations, settings window
6.  **Extensible** - Waybar-style custom modules

**Best for:**
- Performance enthusiasts
- Rust developers
- HyDE users (session bar, theme following, HyDE menu)
- Reliability-focused users

---

### Waybar 

**Why choose:**
1.  **Battle-tested** - Years of production use
2.  **Well-documented** - Extensive wiki
3.  **Large community** - Easy to find help
4.  **Highly customizable** - CSS + script modules
5.  **Multi-compositor** - Sway, Hyprland, river, etc.

**Best for:**
- Users wanting stability
- Those with existing Waybar configs
- Multi-compositor users
- CSS customization lovers

---

### HyprPanel 

**Why choose:**
1.  **Beautiful out-of-box** - Preset themes, polish
2.  **GUI configuration** - No file editing
3.  **Smooth animations** - Polished feel
4.  **Full-featured** - Weather, notifications, calendar
5.  **Modern stack** - TypeScript, hot reload

**Best for:**
- Users wanting beauty first
- TypeScript developers
- Those who prefer GUI config
- Feature-rich setup lovers

---

## Migration Guide

### From Waybar to hydebar

**Pros:**
- Better performance
- Type-safe config
- Memory safety

**Cons:**
- Different config format (TOML vs JSON)
- Some modules may differ
- Beta software

**Steps:**
1. Install hydebar
2. Convert config (script TBD)
3. Test module parity
4. Customize theme

---

### From HyprPanel to hydebar

**Pros:**
- Much faster (Rust vs TS)
- Simpler config (TOML vs TS)
- Notifications, weather, calendar and screenshot included

**Cons:**
- Settings window covers layout and appearance, not every option
- Beta software

**Steps:**
1. Use a preset theme or let the bar follow HyDE
2. Convert config manually

---

## Roadmap Comparison

### hydebar
- Preset themes and animations — delivered
- Performance work with a measured baseline — delivered
- Notification center, screenshot, weather, calendar — delivered
- Settings window growth, documentation site — see [ROADMAP.md](../ROADMAP.md)

### Waybar
- Stable, incremental improvements
- Focus on compatibility
- Rare breaking changes

### HyprPanel
- Active development
- Regular feature additions
- TypeScript ecosystem improvements

---

## Conclusion

### Choose **hydebar** if you want:
- Maximum performance (startup, idle CPU)
- Memory safety (Rust)
- Modern UX: themes, HyDE following, animations
- Type-safe configuration
- Reliability (extensively tested)

### Choose **Waybar** if you want:
- Battle-tested stability
- Extensive documentation
- Large community support
- Multi-compositor support
- Full CSS customization

### Choose **HyprPanel** if you want:
- Beautiful out-of-box
- Full GUI configuration
- TypeScript development

---

**Our goal:** Combine Waybar's stability and performance with HyprPanel's beauty and UX.

---

**Last updated:** 2026-07
