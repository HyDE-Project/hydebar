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
- ⏰ **Clock** - Customizable date/time format, alternative formats cycled on click, calendar menu
- 🌤️ **Weather** - OpenWeatherMap readout attached to the clock (`clock.show_weather` + `[weather]`)
- 📊 **System Info** - CPU, RAM, temperature, GPU, disk, network speeds; readouts auto-detected
- 📈 **CPU / Memory** - Standalone processor and memory entries over the same sampler
- 🔋 **Battery** - Battery status and power profiles
- 📡 **Network** - WiFi with signal strength %, VPN, connection management
- 🔊 **Audio** - Volume control with inline sliders, sink/source selection
- 🎵 **Media Player** - MPRIS integration with playback controls
- 💡 **Brightness** - Screen brightness control with inline slider
- 🔵 **Bluetooth** - Device management with quick connect/disconnect, battery levels
- 📋 **Tray** - System tray support
- 🔄 **Updates** - Package updates applied right in the menu; on HyDE the clone itself is watched and updated alongside, branch selectable
- 📋 **Clipboard** - Clipboard history picker (cliphist by default)
- 🔒 **Privacy** - Camera/microphone/screenshare indicators
- ⌨️ **Keyboard Layout / Submap** - Layout switching with custom labels, active submap
- 🚀 **App Launcher** - Quick app launcher button
- 🔔 **Notifications** - Notification center with selectable source: built-in popups, Hyprland, or the session daemon
- 📸 **Screenshot** - Screenshot and screen recording (grim/slurp/wf-recorder)
- ☕ **Idle Inhibitor** - One click toggle keeping the session awake
- 🎛️ **Control Center** - Quick settings panel: audio, network, bluetooth, power profile, power menu
- ⚙️ **Settings** - The bar's own settings window: module layout and appearance, written back to the config
- 🖼️ **Themes / Wallpaper / HyDE Menu** - Drive the HyDE desktop theme, cycle the wallpaper, open the HyDE menu tree

### Visual Features
- 🎨 **11 Built-in Themes** - Catppuccin, Dracula, Nord, Gruvbox, Tokyo Night
- 🖥️ **HyDE Integration** - Follows the HyDE desktop theme by default (`follow_hyde`), colours, font and radius included
- ✨ **Smooth Animations** - Menu fade in/out, hover effects
- 🏝️ **Multiple Styles** - Islands, Solid, Gradient
- 🎭 **Opacity Control** - Transparent backgrounds and menus
- 🔍 **Auto Scale** - The bar magnifies itself for the screen it lands on (`auto_scale`)

### Customization
- 📦 **Custom Modules** - Extend with your own scripts
- 🎨 **Full Color Control** - Customize every color
- 🔣 **Icon Overrides** - Replace any built-in glyph via the `[icons]` table
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
hover_duration_ms = 100

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

Without any `appearance` setting the bar follows the theme published by the
HyDE Project: colours, font and corner radius are read from the HyDE state
directories and the bar repaints on every theme switch. Set
`follow_hyde = false` under `[appearance]` to opt out.

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

The module needs no configuration: it looks the machine over and draws load,
memory and whichever temperatures the hardware actually reports. A processor
temperature appears where a processor chip reports one, a graphics temperature
and a graphics load appear where a graphics device reports them, and a machine
that reports neither simply shows neither - no blank, no dash, no log line.
The reading of a graphics block built into the processor is tagged `iGPU`, so
it is never mistaken for a card.

Configuration only overrides that: `indicators` pins the readouts and their
order, `hide` drops one from the automatic selection, and `[system.gpu]` names
which device to watch on a machine with more than one.

```toml
[system]
# Optional: pin the readouts and their order.
indicators = ["Cpu", "Memory", "CpuTemperature", "GpuTemperature", {"disk" = "/"}]
# Optional: drop a readout the panel would otherwise draw.
hide = ["GpuUsage"]

[system.cpu]
warn_threshold = 60
alert_threshold = 80

[system.gpu]
# Optional: "amd", "intel", "nvidia", a driver name, "discrete" or "integrated".
device = "discrete"
warn_threshold = 70
alert_threshold = 85
```

### Power Management

```toml
[control_center]
lock_cmd = "hyprlock &"
shutdown_cmd = "shutdown now"
suspend_cmd = "systemctl suspend"
reboot_cmd = "systemctl reboot"
logout_cmd = "loginctl kill-user $(whoami)"
```

The section was previously named `[settings]`; that spelling is still accepted
as an alias.

Full configuration reference at [docs/configuration](https://raprogramm.github.io/hydebar/docs/configuration).

---

## Performance

Measured on a 4K output, release build (see
[docs/perf-baseline-2026-07.md](docs/perf-baseline-2026-07.md)):

- 🚀 **Fast Startup** - ~53ms from launch to mapped surface
- ⚡ **Efficient** - ~0.5% CPU when idle; a menu opening and settling costs the same
- 💾 **Memory** - ~127MB resident, dominated by the GPU rendering stack (wgpu over Vulkan)
- 🦀 **100% Rust** - Memory-safe, zero-cost abstractions

See [PERFORMANCE.md](PERFORMANCE.md) for the full numbers and methodology.

---

## Development

### Building from Source

```bash
git clone https://github.com/RAprogramm/hydebar.git
cd hydebar
cargo build --release
./target/release/hydebar-app
```

`./install.sh` builds the release binary and installs it as `hydebar`, together
with the `hydebar-theme-switch` script. `./install.sh --hyde` additionally
registers the bar as the HyDE session bar (see
[docs/hyde-session.md](docs/hyde-session.md)).

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
- Keyboard layout and submap

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
