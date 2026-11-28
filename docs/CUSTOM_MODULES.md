# Custom Modules

A custom module is a bar entry driven by an external command — no Rust
needed. Place one with `custom/<name>` in `[modules]`, then describe it in
a `[[CustomModule]]` section carrying the same `name`.

## The Three Flavors

Pick by cost, cheapest first:

- **Button** (`command` only): shows its icon, runs a command on press,
  produces no readout. Costs nothing while idle.
- **Polling** (`exec` + `interval`): runs a short-lived command producing
  one JSON object per run, on a schedule, on demand via `signal`, or both.
- **Streaming** (`listen_cmd`): spawns one long-lived process and reads
  its JSON lines until it exits. For feeds that push rather than answer.

`signal = 20` refreshes on demand with `pkill -RTMIN+20 hydebar`, matching
the Waybar contract `HyDE` scripts rely on.

## Examples

A button running a command on press:

```toml
[modules]
right = ["custom/launcher"]

[[CustomModule]]
name = "launcher"
icon = ""
command = "notify-send 'Clicked!'"
```

A readout polled every few seconds:

```toml
[modules]
right = ["custom/weather"]

[[CustomModule]]
name = "weather"
command = ""
exec = "weather.sh"
interval = 300
```

A readout refreshed on demand instead of on a schedule:

```toml
[modules]
right = ["custom/updates"]

[[CustomModule]]
name = "updates"
command = "hyde-shell app system.update.sh up"
exec = "hyde-shell system.update"
signal = 20
```

A streaming feed, one JSON object per line:

```toml
[modules]
right = ["custom/notifications"]

[[CustomModule]]
name = "notifications"
command = "swaync-client -t -sw"
listen_cmd = "swaync-client -swb"
```

A button opening a context menu on right press:

```toml
[modules]
right = ["custom/power"]

[[CustomModule]]
name = "power"
command = "hyde-shell logoutlaunch 1"

[[CustomModule.menu]]
label = "Lock"
command = "hyde-shell lockscreen.sh"
```

## Debugging

- Start with `command` only and confirm the entry appears; add the
  producing command second.
- Test a signal module with `pkill -RTMIN+20 hydebar` and watch the log.
- A module whose producer exits immediately with no schedule shows its
  icon and stays quiet — that is the button flavor, not an error.
