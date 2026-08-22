# Frequently Asked Questions

## General

### What is hydebar?

hydebar is a fast, beautiful Wayland status bar built specifically for Hyprland. It provides all the features you need in a modern desktop panel: workspaces, system info, media controls, and more.

### Why use hydebar instead of Waybar or HyprPanel?

**vs Waybar:**
- Faster (100% Rust vs C++)
- Better Hyprland integration
- Built-in themes
- Smooth animations
- Lower memory usage

**vs HyprPanel:**
- Much faster (Rust vs TypeScript/GTK)
- Lower resource usage
- Native Wayland (no GTK overhead)
- More stable
- Weather, calendar and notification center included

### Is it stable for daily use?

Yes! hydebar is actively developed and tested, and is stable for daily use. Report any bugs on [GitHub Issues](https://github.com/RAprogramm/hydebar/issues).

---

## Installation

### How do I install on Arch Linux?

```bash
paru -S hydebar
```

Or for latest development version:
```bash
paru -S hydebar-git
```

### How do I install on other distros?

See [README.md](../README.md#installation) for:
- Nix/NixOS
- ALT Linux
- Building from source

### How do I auto-start with Hyprland?

Add to `~/.config/hypr/hyprland.conf`:
```conf
exec-once = hydebar
```

---

## Configuration

### Where is the config file?

`~/.config/hydebar/config.toml`

Create it if it doesn't exist. See [Getting Started](GETTING_STARTED.md) for examples.

### Do I need to restart after config changes?

No! hydebar automatically reloads when you save config changes.

### Can I use multiple config files?

You can pass a custom config path:
```bash
hydebar --config-path ~/my-config.toml
```

Note that only one bar runs per user: starting hydebar again replaces the
running instance, so `--config-path` selects which configuration the single
bar reads rather than starting a second one.

### How do I reset to defaults?

Delete or rename your config file:
```bash
mv ~/.config/hydebar/config.toml ~/.config/hydebar/config.toml.backup
```

hydebar will use built-in defaults.

---

## Themes

### How many themes are included?

11 preset themes:
- Catppuccin (4 variants)
- Dracula
- Nord
- Gruvbox (2 variants)
- Tokyo Night (3 variants)

See [THEMES.md](THEMES.md) for previews.

### How do I change themes?

Edit `~/.config/hydebar/config.toml`:
```toml
appearance = "catppuccin-mocha"
```

Changes apply instantly!

### Can I create custom themes?

Yes! Either:
1. Customize an existing theme
2. Define all colors manually

See [THEMES.md](THEMES.md#creating-custom-themes) for details.

### Can I submit new themes?

Yes! See [Contributing](#contributing) below.

---

## Modules

### What modules are available?

- Workspaces
- Window Title
- System Info (CPU, RAM, temperatures, GPU, disk, network)
- Clock (with calendar menu and optional weather)
- Battery
- Network (WiFi, VPN)
- Audio
- Bluetooth
- Brightness (inside the control center)
- Power Profile
- Media Player
- Tray
- Updates
- Clipboard
- Privacy (camera/mic/screenshare indicators)
- Keyboard Layout / Keyboard Submap
- App Launcher
- Notifications (with DND mode and selectable source)
- Screenshot / screen recording
- Idle Inhibitor
- Control Center (quick settings + power menu)
- Settings (the bar's own settings window)
- Themes / Wallpaper / HyDE Menu (drive the HyDE desktop)
- Custom modules

### Can I reorder modules?

Yes:
```toml
[modules]
left = ["Workspaces"]
center = ["WindowTitle"]
right = ["SystemInfo", "Clock", "Battery", "Settings"]
```

### Can I hide modules?

Yes, just remove them from your config:
```toml
[modules]
right = ["Clock"]  # Only show clock
```

### How do I create custom modules?

See configuration example:
```toml
[[CustomModule]]
name = "MyModule"
icon = ""
command = "notify-send 'Clicked!'"
```

To refresh a module on a timer, add `exec` and `interval` (seconds); add
`signal = N` to also refresh it on demand with `pkill -RTMIN+N hydebar`:
```toml
[[CustomModule]]
name = "cpuinfo"
command = ""
exec = "hyde-shell cpuinfo"
interval = 5
```

Advanced custom modules can update dynamically. See [README.md](../README.md#custom-modules).

---

## Performance

### How much RAM does hydebar use?

Around 127MB resident in a release build. Almost all of it is the GPU
rendering stack (wgpu over Vulkan) — the price of the renderer, not of the
modules. See [perf-baseline-2026-07.md](perf-baseline-2026-07.md).

### How much CPU does it use?

Around 0.5% idle; a menu opening and settling costs the same. The bar is
event-driven and wakes only for its pollers.

### How fast is startup?

Around 53ms from launch to a mapped surface (release build, measured).

### How can I reduce resource usage?

1. Disable animations:
```toml
[appearance.animations]
enabled = false
```

2. Use fewer modules — a module absent from the layout starts no background
work at all:
```toml
[modules]
right = ["Clock"]
```

---

## Troubleshooting

### Transparency isn't working

Try forcing OpenGL:
```bash
WGPU_BACKEND=gl hydebar
```

### Icons show as boxes

The symbols font (Nerd Font glyphs) ships inside the binary, so the built-in
icons need no font package. Boxes usually mean a custom `font_name` without
the glyphs you configured in `[icons]` or in a custom module — pick glyphs
your font carries, or drop the override.

### Battery module doesn't appear

Check UPower:
```bash
systemctl status upower
```

Or force show:
```toml
[battery]
show_when_unavailable = true
```

### More issues?

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for detailed solutions.

---

## Features

### Does it support multi-monitor?

Yes! hydebar automatically spawns on all outputs, or you can specify:
```toml
outputs = "All"  # Default
# outputs = "Active"
# outputs = { Targets = ["DP-1", "HDMI-1"] }
```

### Does it work on other Wayland compositors?

Partially. Some features require Hyprland:
- Workspaces
- Window title
- Keyboard layout

Generic modules (Clock, System Info, Tray) should work on other compositors, but this isn't officially supported yet.

### Is there a notification center?

Yes. Add `Notifications` to your layout. The source is selectable — the bar's
own popups, Hyprland's native notifications, or the session's daemon:

```toml
[notifications]
source = "Daemon"   # or "Builtin", "Compositor"
```

See [notification-source.md](notification-source.md) for how the three modes
work.

### Can I have a vertical panel?

Not yet. Planned for future versions.

### Can I auto-hide the panel?

Not yet. Planned for future versions.

---

## Development

### Is hydebar actively maintained?

Yes! Check [ROADMAP.md](../ROADMAP.md) for planned features and timeline.

### Can I contribute?

Yes! Contributions welcome. See [Contributing](#contributing) section below.

### What's the development stack?

- **Language:** 100% Rust (edition 2024, rust-version 1.97)
- **GUI:** iced via the `iced_layershell` fork (Wayland layer-shell)
- **IPC:** Hyprland socket (hyprland-rs)
- **D-Bus:** zbus for system integration
- **Build:** Cargo workspace (`hydebar-proto`, `hydebar-core`, `hydebar-gui`, `hydebar-app`)

### Where's the source code?

[GitHub: RAprogramm/hydebar](https://github.com/RAprogramm/hydebar)

---

## Contributing

### How can I contribute?

Several ways:
1. **Report bugs** - [Open an issue](https://github.com/RAprogramm/hydebar/issues/new)
2. **Request features** - [Start a discussion](https://github.com/RAprogramm/hydebar/discussions)
3. **Submit themes** - Create PR with new preset theme
4. **Write code** - Check [ROADMAP.md](../ROADMAP.md) for planned features
5. **Improve docs** - Fix typos, add examples

### What should I work on?

Check [ROADMAP.md](../ROADMAP.md) for:
- High priority features
- Good first issues
- Planned milestones

### How do I submit changes?

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

### Coding standards?

- Follow Rust conventions
- Run `cargo fmt` before committing
- Add tests for new features
- Update documentation

---

## Licensing

### What license is hydebar under?

The GNU General Public License, version 3 or later. See
[LICENSE](../LICENSE) for the full text. Parts of the code began in the ashell
project under the MIT License, and that notice is kept in
[LICENSE.MIT](../LICENSE.MIT).

### Can I use it commercially?

Yes. The licence says nothing about charging for the software; what it asks is
that anyone you hand a binary to can get the source on the same terms.

### Can I fork/modify it?

Yes, and it is encouraged. A fork carries the same licence: publish your
changes under the GPL, version 3 or later, and keep both notices with the
code.

---

## Support

### Where do I get help?

1. Check this FAQ
2. Read [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
3. Search [existing issues](https://github.com/RAprogramm/hydebar/issues)
4. Ask in [Discussions](https://github.com/RAprogramm/hydebar/discussions)
5. Open a [new issue](https://github.com/RAprogramm/hydebar/issues/new)

### How do I report bugs?

Open an issue with:
- hydebar version
- System info (OS, Hyprland version)
- Config file (sanitized)
- Steps to reproduce
- Debug logs if relevant

### Can I request features?

Yes! Open a discussion or issue describing:
- What you want
- Why it's useful
- How it might work

---

## Roadmap

### What's planned for the future?

See [ROADMAP.md](../ROADMAP.md). The notification center, weather, calendar,
screenshot tools and the performance work have already landed.

**Future:**
- More of the configuration editable from the settings window
- Documentation website
- Vertical panel, auto-hide, plugin system (ideas)

---

## Comparison

### hydebar vs Waybar

| Feature | hydebar | Waybar |
|---------|---------|--------|
| Language | Rust | C++ |
| Startup | ~53ms | ~100ms |
| Idle CPU | ~0.5% | ~2% |
| Memory | ~127MB (GPU stack) | ~10MB (GTK) |
| Themes | 11 built-in + HyDE following | Manual CSS |
| Animations | Yes, smooth | Limited |
| Hyprland | Deep integration | Generic |

### hydebar vs HyprPanel

| Feature | hydebar | HyprPanel |
|---------|---------|-----------|
| Language | Rust | TypeScript |
| Performance | Fast | Moderate |
| Startup | ~53ms | ~500ms |
| Widgets | Full set incl. weather, calendar, notifications | More |
| Stability | High | Moderate |

---

**Have more questions?** Ask in [Discussions](https://github.com/RAprogramm/hydebar/discussions)!
