# Getting Started with hydebar

This guide will help you install and configure hydebar in just a few minutes.

## Prerequisites

- **Hyprland** compositor
- **Wayland** session
- **Rust** toolchain (for building from source)

## Installation

### Arch Linux (Recommended)

The easiest way to install on Arch:

```bash
paru -S hydebar
```

Or for the latest development version:

```bash
paru -S hydebar-git
```

### Other Distributions

See [README.md](../README.md#installation) for Nix, ALT Linux, and other options.

### Building from Source

```bash
# Clone repository
git clone https://github.com/RAprogramm/hydebar.git
cd hydebar

# Build release version
cargo build --release

# Binary will be at: target/release/hydebar-app
```

## First Run

### Basic Setup

1. Create config directory:
```bash
mkdir -p ~/.config/hydebar
```

2. Create minimal config file `~/.config/hydebar/config.toml`:
```toml
# Use a preset theme
appearance = "catppuccin-mocha"
```

3. Run hydebar:
```bash
hydebar
```

That's it! You should see a beautiful status bar with the Catppuccin Mocha theme.

### Auto-start with Hyprland

Add to your `~/.config/hypr/hyprland.conf`:

```conf
exec-once = hydebar
```

## Choosing a Theme

hydebar includes 11 beautiful preset themes. Try them by editing your config:

```toml
# Dark themes
appearance = "catppuccin-mocha"      # Purple/pink (default)
appearance = "dracula"               # Purple/pink
appearance = "nord"                  # Cool blue
appearance = "gruvbox-dark"          # Warm retro
appearance = "tokyo-night"           # Neon accents

# Light themes
appearance = "catppuccin-latte"      # Pastel light
appearance = "gruvbox-light"         # Warm light
appearance = "tokyo-night-light"     # Clean light
```

Changes apply instantly - no restart needed!

## Customizing Layout

Configure which modules appear and where:

```toml
[modules]
left = ["Workspaces"]
center = ["WindowTitle"]
right = ["SystemInfo", "Clock", "Battery", "Settings"]
```

Available modules:
- `Workspaces` - Hyprland workspaces
- `WindowTitle` - Active window
- `SystemInfo` - CPU/RAM/temp/network
- `Clock` - Date and time
- `Battery` - Battery status with power profiles
- `MediaPlayer` - Music controls (MPRIS)
- `Tray` - System tray icons
- `Privacy` - Camera/mic/screenshare indicators
- `Notifications` - Notification center with DND mode
- `Screenshot` - Screenshot and screen recording
- `IdleInhibitor` - Toggle keeping the session awake (caffeine)
- `Settings` - Comprehensive settings panel
- Custom modules (see Advanced section)

## Common Configurations

### Minimal Setup

```toml
appearance = "nord"

[modules]
left = ["Workspaces"]
center = []
right = ["Clock"]
```

### Full-Featured

```toml
appearance = "catppuccin-mocha"

[modules]
left = ["Workspaces"]
center = ["WindowTitle"]
right = [
    "SystemInfo",
    ["Privacy", "Notifications", "Screenshot"],
    ["Clock", "Battery", "Settings"]
]

# Show system info. The readouts are found automatically; listing them
# here is only needed to pin the selection or the order.
[system]
indicators = ["Cpu", "Memory", "CpuTemperature", "DownloadSpeed"]

# Configure clock
[clock]
format = "%a %d %b %H:%M"
```

## Alternative Formats

Like waybar's `format-alt`, a module can carry a list of alternative formats and
cycle through them on a left click, wrapping back to the primary format after
the last alternative. A module that declares no alternative keeps working
exactly as before.

A module that opens a menu keeps it on the left click while it has no
alternative; as soon as one is declared, the left click cycles the format and
the menu moves to the right click.

```toml
# Clock: time, then date, then back to time
[clock]
format = "%I:%M %p"
format-alt = ["%R %d·%m·%y"]

# System info: gigabytes in use, then percentage, then back to gigabytes
[system.memory]
format = "Bytes"
format-alt = ["Percentage"]
```

### System Readouts

The system module finds its own readouts. Load and memory come from every
machine; `CpuTemperature`, `GpuTemperature` and `GpuUsage` appear only where
the hardware reports them, so a machine without a graphics sensor shows no
graphics readout at all and a virtual machine without any sensor shows just
load and memory. `Temperature` remains a valid spelling of `CpuTemperature`.

The graphics reading is taken from the die where the driver publishes one,
from the package edge otherwise, and from the memory as a last resort. A card
is preferred over the graphics built into the processor; when the built-in one
is shown it is tagged `iGPU`. Clicking the module lists any readout the
machine cannot report, together with the reason.

```toml
[system]
hide = ["GpuUsage"]          # drop one readout, keep the rest automatic

[system.gpu]
device = "nvidia"            # pin the device on a machine with several
```

The memory readout applies to the `Memory` and `MemorySwap` indicators;
`Percentage` shows `50%` and `Bytes` shows `7.8GB`. Both keys also accept their
snake_case spelling (`format_alt`).

### Custom Colors

Instead of a preset theme, you can customize every color:

```toml
[appearance]
style = "Islands"
opacity = 0.95

background_color = "#1e1e2e"
primary_color = "#cba6f7"
secondary_color = "#11111b"
success_color = "#a6e3a1"
danger_color = "#f38ba8"
text_color = "#cdd6f4"
```

## Animations

Control menu animations:

```toml
[appearance.animations]
enabled = true
menu_fade_duration_ms = 200  # Fade duration in milliseconds
hover_duration_ms = 100      # Hover effect duration
```

Disable animations entirely:

```toml
[appearance.animations]
enabled = false
```

## Next Steps

- [Full Configuration Guide](CONFIGURATION.md) - All options explained
- [Theme Showcase](THEMES.md) - Preview all themes
- [Troubleshooting](TROUBLESHOOTING.md) - Common issues
- [Module Reference](MODULES.md) - Per-module settings

## Quick Tips

1. **Config reloads automatically** - Edit and save, changes appear instantly
2. **Use preset themes** - Easier than manual colors
3. **Group modules** - Use nested arrays: `["Clock", "Battery"]`
4. **Check logs** - Run with `RUST_LOG=debug hydebar` for debugging

## Getting Help

- [GitHub Issues](https://github.com/RAprogramm/hydebar/issues) - Bug reports
- [Discussions](https://github.com/RAprogramm/hydebar/discussions) - Questions
- [ROADMAP.md](../ROADMAP.md) - Planned features

---

**Welcome to hydebar!** Enjoy your beautiful new status bar.
