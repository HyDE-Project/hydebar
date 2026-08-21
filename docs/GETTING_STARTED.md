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

Or run `./install.sh`, which builds the release binary and installs it as
`hydebar` together with the `hydebar-theme-switch` script; `./install.sh
--hyde` also registers the bar as the HyDE session bar (see
[hyde-session.md](hyde-session.md)).

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

An empty (or absent) config also works: the bar then follows the theme
published by the HyDE Project — colours, font and corner radius — and
repaints on every desktop theme switch.

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
appearance = "catppuccin-mocha"      # Purple/pink
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
- `SystemInfo` - CPU/RAM/temperatures/GPU/disk/network
- `Clock` - Date and time, with a calendar menu and optional weather readout
- `Battery` - Battery status with power profiles
- `MediaPlayer` - Music controls (MPRIS)
- `Tray` - System tray icons
- `Updates` - Package updates and, on HyDE, the HyDE clone itself (needs `[updates]` commands)
- `Cpu` / `Memory` - Standalone processor and memory readouts over the same sampler as `SystemInfo`
- `Clipboard` - Clipboard history picker
- `AppLauncher` - Application launcher button
- `KeyboardLayout` / `KeyboardSubmap` - Layout and active submap
- `Privacy` - Camera/mic/screenshare indicators
- `Notifications` - Notification center with DND mode
- `Screenshot` - Screenshot and screen recording
- `IdleInhibitor` - Toggle keeping the session awake (caffeine)
- `ControlCenter` - Quick settings panel: audio, network, bluetooth, power
- `Audio` / `Network` / `Bluetooth` / `PowerProfile` - Standalone readouts from the control center
- `Settings` - The bar's own settings window (layout and appearance)
- `Themes` / `Wallpaper` / `HydeMenu` - HyDE desktop theme switcher, wallpaper cycling, HyDE menu tree
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

### Updates

```toml
[updates]
check_cmd = "checkupdates; paru -Qua"
update_cmd = "hyde-shell pm --no-confirm fetch && hyde-shell pm --no-confirm upgrade"
check_interval = 3600    # seconds between checks
hyde_branch = "Master"   # or "Dev"; also switchable from the settings window
```

Pressing **Update** runs `update_cmd` without a terminal: the output streams
into the menu as the tail of its last lines, anything that needs elevation
asks through the desktop's polkit agent, and when the command ends the
pending count is re-checked rather than assumed. Pick a command that asks no
questions — the button press is the confirmation. A command that opens a
terminal of its own still works; it just has nothing to narrate.

On a HyDE desktop the same menu watches the HyDE clone itself: it names the
branch it follows, unfolds the upstream commits it is missing, and
**Update HyDE** brings the clone up to date the way upstream documents it —
fetch, hard reset, restore — narrating in place. A clone carrying
uncommitted work is refused; a clean clone standing on another branch is
switched, and the branch it left keeps its commits.

### Weather

The weather readout rides with the clock:

```toml
[clock]
show_weather = true

[weather]
location = "London"
api_key = "..."               # OpenWeatherMap API key
use_celsius = true
update_interval_minutes = 30
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

## Greeting

The first bar of a session greets you mid-screen for a few seconds and gets
out of the way; reloads and restarts within the same session stay silent.
On by default:

```toml
[appearance]
greeting = false   # turn it off
```

## The desk

The bar and the desk are one thing in two shapes. While a window is mapped on
a screen the bar is the strip along its edge; the moment that workspace is
cleared the islands leave the strip and travel down the screen, and once they
have come to rest each one writes out everything it knows. A window maps again
and they fly back onto the strip from beyond the edges of the screen. They stay
as clickable as they ever were. Off by default:

```toml
[desk]
enabled = true
```

There is no second layout to write: the three sections of `[modules]` become
the three columns of the canvas, and the place a module takes in its column
follows how near the middle of the strip it stood — the nearer the higher, so
the far ends of the bar reach for the bottom corners. Rearranging the bar
rearranges the desk with it.

A floating window is a visitor: it sits over the canvas without folding it
away. Only a window tiled into the workspace takes the screen back.

The whole bar leaves at one instant: nothing waits its turn, so the unfolding
takes as long as one island's flight however many islands there are. They keep
clear of one another by where they go rather than by when — each drops to its
own level straight down the line it stood on, and only then closes in along
its own lane, the nearer the middle of the strip the nearer the middle of the
screen. An island that
carried a group of modules travels whole, under the one pill the strip drew
around it, and opens only once it has come to rest. How fast that goes is the
theme's to say:

```toml
[appearance.animations]
desk_block_ms = 620   # time an island takes to cross the screen and open, and so the whole unfolding
```

Every screen answers for itself: a second monitor running a browser keeps its
strip while the first one, cleared, unfolds. A special workspace pulled up
over a screen counts as something on it.

## Next Steps

- [Theme Showcase](THEMES.md) - Preview all themes
- [Troubleshooting](TROUBLESHOOTING.md) - Common issues
- [FAQ](FAQ.md) - Frequently asked questions
- [README](../README.md) - Custom modules, system info and power menu configuration

## Quick Tips

1. **Config reloads automatically** - Edit and save, changes appear instantly
2. **Use preset themes** - Easier than manual colors
3. **Group modules** - Use nested arrays: `["Clock", "Battery"]`
4. **Check logs** - Run with `RUST_LOG=debug hydebar` (or set `log_level = "debug"` in the config); log files land in `/tmp/hydebar/`

## Getting Help

- [GitHub Issues](https://github.com/RAprogramm/hydebar/issues) - Bug reports
- [Discussions](https://github.com/RAprogramm/hydebar/discussions) - Questions
- [ROADMAP.md](../ROADMAP.md) - Planned features

---

**Welcome to hydebar!** Enjoy your beautiful new status bar.
