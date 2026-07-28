<div align = center>
    <a href="https://discord.gg/AYbJ9MJez7">
<img alt="Dynamic JSON Badge" src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fdiscordapp.com%2Fapi%2Finvites%2FmT5YqjaJFh%3Fwith_counts%3Dtrue&query=%24.approximate_member_count&suffix=%20members&style=for-the-badge&logo=discord&logoSize=auto&label=The%20HyDe%20Project&labelColor=ebbcba&color=c79bf0">
    </a>
</div>
<div align = center><img src="https://raw.githubusercontent.com/prasanthrangan/hyprdots/main/Source/assets/hyde_banner.png"><br><br></div>


# hydebar

**A fast, beautiful Wayland status bar for Hyprland**

[![Packaging status](https://repology.org/badge/vertical-allrepos/hydebar.svg)](https://repology.org/project/hydebar/versions)

> ⚡ Blazing fast • 🎨 Beautiful themes • 🔧 Easy configuration

---

## Features

### Core Modules
- 🪟 **Workspaces** - Hyprland workspace integration
- 📝 **Window Title** - Active window information
- ⏰ **Clock** - Customizable date/time format
- 📊 **System Info** - CPU, RAM, temperature, disk, network speeds
- 🔋 **Battery** - Battery status and power profiles
- 📡 **Network** - WiFi with signal strength %, VPN, connection management
- 🔊 **Audio** - Volume control with inline sliders, sink/source selection
- 🎵 **Media Player** - MPRIS integration with playback controls
- 💡 **Brightness** - Screen brightness control with inline slider
- 🔵 **Bluetooth** - Device management with quick connect/disconnect, battery levels
- 📋 **Tray** - System tray support
- 🔄 **Updates** - Package update notifications
- 🔒 **Privacy** - Camera/microphone/screenshare indicators
- ⌨️ **Keyboard Layout** - Layout switching with custom labels
- 🚀 **App Launcher** - Quick app launcher button
- 🔔 **Notifications** - Notification center with D-Bus integration, DND mode
- 📸 **Screenshot** - Screenshot and screen recording (grim/slurp/wf-recorder)
- ☕ **Idle Inhibitor** - One click toggle keeping the session awake
- ⚙️ **Settings Panel** - Comprehensive settings menu

### Visual Features
- 🎨 **11 Built-in Themes** - Catppuccin, Dracula, Nord, Gruvbox, Tokyo Night
- ✨ **Smooth Animations** - Menu fade in/out, hover effects
- 🏝️ **Multiple Styles** - Islands, Solid, Gradient
- 🎭 **Opacity Control** - Transparent backgrounds and menus

### Customization
- 📦 **Custom Modules** - Extend with your own scripts
- 🎨 **Full Color Control** - Customize every color
- 📐 **Flexible Layout** - Position modules left/center/right
- 🔄 **Hot Reload** - Config changes apply instantly

---

## Quick Start

### Installation

#### Arch Linux
```bash
# Stable release
paru -S hydebar

# Development version
paru -S hydebar-git
```

#### ALT Linux
```bash
sudo apt-get install hydebar
```

#### Nix
```bash
# Stable
nix profile install github:RAprogramm/hydebar?ref=0.6.7

# Latest
nix profile install github:RAprogramm/hydebar
```

See [Installation Guide](https://raprogramm.github.io/hydebar/docs/installation) for more options.

### Basic Configuration

Create `~/.config/hydebar/config.toml`:

```toml
# Use a preset theme
appearance = "catppuccin-mocha"

# Or customize colors
[appearance]
style = "Islands"
opacity = 0.95
background_color = "#1e1e2e"
primary_color = "#cba6f7"
text_color = "#cdd6f4"

# Configure animations
[appearance.animations]
enabled = true
menu_fade_duration_ms = 200

# Module layout
[modules]
left = ["Workspaces"]
center = ["WindowTitle"]
right = [["Privacy", "Notifications", "Screenshot"], "Clock", "Settings"]
```

### Available Themes

```toml
# Catppuccin variants
appearance = "catppuccin-mocha"      # Dark purple
appearance = "catppuccin-macchiato"  # Dark blue
appearance = "catppuccin-frappe"     # Lighter purple
appearance = "catppuccin-latte"      # Light theme

# Other popular themes
appearance = "dracula"          # Dark purple/pink
appearance = "nord"             # Cool blue
appearance = "gruvbox-dark"     # Warm retro dark
appearance = "gruvbox-light"    # Warm retro light
appearance = "tokyo-night"      # Dark with neon accents
appearance = "tokyo-night-storm"
appearance = "tokyo-night-light"
```

---

## Screenshots

### Themes

| Catppuccin Mocha | Dracula |
|------------------|---------|
| ![Mocha](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/hydebar.png) | ![Dracula](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/hydebar-gradient.png) |

### Menus

| Settings Panel | Power Menu |
|----------------|------------|
| ![Settings](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/settings-panel.png) | ![Power](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/power-menu.png) |

| Network Menu | Bluetooth Menu |
|--------------|----------------|
| ![Network](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/network-menu.png) | ![Bluetooth](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/bluetooth-menu.png) |

---

## Documentation

- 📖 [Configuration Guide](https://raprogramm.github.io/hydebar/docs/configuration) - All configuration options
- 🎨 [Theme Guide](https://raprogramm.github.io/hydebar/docs/themes) - Creating custom themes
- 🔧 [Module Reference](https://raprogramm.github.io/hydebar/docs/modules) - Module-specific settings
- 🐛 [Troubleshooting](https://raprogramm.github.io/hydebar/docs/troubleshooting) - Common issues

---

## Advanced Configuration

### Custom Modules

```toml
[[CustomModule]]
name = "CustomNotifications"
icon = ""
command = "swaync-client -t -sw"
command_right = "swaync-client -d -sw"
command_middle = "swaync-client -C"
listen_cmd = "swaync-client -swb"
icons.'dnd.*' = ""
alert = ".*notification"
```

`listen_cmd` keeps a process alive and reads one JSON object per line. When the
data comes from a command that exits instead, use `exec` together with
`interval` (seconds) and, optionally, `signal`:

```toml
[[CustomModule]]
name = "cpuinfo"
command = ""
exec = "hyde-shell cpuinfo"
interval = 5

[[CustomModule]]
name = "updates"
command = "hyde-shell app system.update.sh up"
exec = "hyde-shell system.update"
interval = 86400
signal = 20
```

`exec` runs once at startup and again on every `interval` tick. `signal = 20`
registers `SIGRTMIN+20`, so `pkill -RTMIN+20 hydebar` refreshes the module
immediately — the same contract Waybar scripts use.

#### Context Menus

A module can open a menu on a right press instead of running a command. Each
entry carries a label, the command it runs and optionally a glyph:

```toml
[[CustomModule]]
name = "power"
icon = ""
command = "hyde-shell logoutlaunch 1"

[[CustomModule.menu]]
label = "Lock"
icon = "󰍁"
command = "hyde-shell lockscreen.sh"

[[CustomModule.menu]]
label = "Shutdown"
icon = "󰚦"
command = "systemctl poweroff"
```

The menu is anchored under the module, and selecting an entry runs its command
and closes the menu. Declaring at least one entry takes precedence over
`command_right`, which is then ignored: one press cannot both open a menu and
run a command.

### System Information

```toml
[system]
indicators = ["Cpu", "Memory", "Temperature", {"disk" = "/"}, "DownloadSpeed"]

[system.cpu]
warn_threshold = 60
alert_threshold = 80
```

### Power Management

```toml
[settings]
lock_cmd = "hyprlock &"
shutdown_cmd = "shutdown now"
suspend_cmd = "systemctl suspend"
reboot_cmd = "systemctl reboot"
logout_cmd = "loginctl kill-user $(whoami)"
```

Full configuration reference at [docs/configuration](https://raprogramm.github.io/hydebar/docs/configuration).

---

## Performance

- 🚀 **Fast Startup** - < 50ms first paint
- 💾 **Low Memory** - < 5MB idle
- ⚡ **Efficient** - < 1% CPU when idle
- 🦀 **100% Rust** - Memory-safe, zero-cost abstractions

See [PERFORMANCE.md](PERFORMANCE.md) for benchmarks.

---

## Development

### Building from Source

```bash
git clone https://github.com/RAprogramm/hydebar.git
cd hydebar
cargo build --release
./target/release/hydebar-app
```

### Contributing

Contributions are welcome! See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for detailed guidelines.

Quick links:
- 🎨 [Submit new themes](docs/CONTRIBUTING.md#theme-development)
- 🐛 [Report bugs](docs/CONTRIBUTING.md#report-bugs)
- ✨ [Request features](docs/CONTRIBUTING.md#request-features)
- 💻 [Development workflow](docs/CONTRIBUTING.md#development-workflow)
- 📋 [Roadmap](ROADMAP.md)

---

## Troubleshooting

### Graphics Issues

If you experience transparency or rendering issues:

```bash
WGPU_BACKEND=gl hydebar
```

This forces OpenGL instead of Vulkan.

### Only One Bar Per User

Starting hydebar while another copy is running does not add a second bar: the
new process asks the running one to quit, waits for it to take its surfaces off
the screen and then takes over. If the running bar does not go away within two
seconds the newcomer prints an error and exits without drawing anything.

Ownership is recorded in `$XDG_RUNTIME_DIR/hydebar/instance.lock`, or in
`/tmp/hydebar-$UID/instance.lock` when the session exports no runtime
directory. The identity is the user, not the configuration file, because a bar
claims the surfaces of every requested output; `--config-path` therefore selects
which configuration the single instance reads rather than starting a second bar.
A lock file left behind by a crashed bar never blocks a restart.

### Hyprland-Only Features

Currently relies on [hyprland-rs](https://github.com/hyprland-community/hyprland-rs) for:
- Active window information
- Workspace management
- Keyboard layout

Support for other compositors is planned but not yet implemented.

---

## Acknowledgements

hydebar evolved from ideas initially explored in the ashell project. The current architecture benefits from those early prototypes.

---

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

**Made with ❤️ for the Hyprland community**

[Website](https://raprogramm.github.io/hydebar) • [Issues](https://github.com/RAprogramm/hydebar/issues) • [Discussions](https://github.com/RAprogramm/hydebar/discussions)
