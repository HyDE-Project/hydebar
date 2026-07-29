# Running hydebar as the HyDE session bar

HyDE does not hardcode waybar. The bar it starts with the session is one
launch line, read from `~/.config/hyde/config.toml`, and replacing it makes
hydebar the session bar: started at login, restarted with the session,
recoloured by every theme switch through the watchers the bar already has.

## How HyDE starts its bar

Both of HyDE's configuration chains end in the same `exec-once`:

- the **Lua chain** (`~/.local/share/hypr/hyde.lua`) reads
  `hyde.config.start.bar`, filled from the `[desktop.start]` table of
  `~/.config/hyde/config.toml`;
- the **classic `.conf` chain** reads `$start.BAR`, whose default lives in
  `~/.local/share/hypr/variables.conf` and whose override comes from the
  `[hyprland-start]` table of the same `config.toml`, translated by
  `hyde-config` into `~/.local/state/hyde/hyprland.conf`.

The stock value on both chains is
`hyde-shell app -u hyde-$XDG_SESSION_DESKTOP-bar.scope -t scope -- waybar.py --watch`:
a supervisor scope that in turn keeps waybar alive in a
`hyde-<desktop>-bar.service` transient unit with `Restart=always`. That is why
killing waybar brings it back — the unit, not the process, has to be stopped.

## Registering hydebar

`./install.sh --hyde` writes the launch line into both tables:

```toml
[desktop.start]
bar = "hyde-shell app -u hyde-$XDG_SESSION_DESKTOP-bar.scope -t scope -- hydebar"

[hyprland-start]
bar = "hyde-shell app -u hyde-$XDG_SESSION_DESKTOP-bar.scope -t scope -- hydebar"
```

The previous `config.toml` is kept beside the new one as
`config.toml.before-hydebar`, and a file that already declares a `bar` key is
left untouched — the keys above are printed for a manual edit instead.

The change takes effect at the next session start. To switch a running
session over:

```sh
systemctl --user stop "hyde-$XDG_SESSION_DESKTOP-bar.service" "hyde-$XDG_SESSION_DESKTOP-bar.scope"
hyde-shell app -u hyde-$XDG_SESSION_DESKTOP-bar.scope -t scope -- hydebar
```

To go back, restore the backup or set both `bar` keys to the stock waybar
line quoted above.

## What HyDE keys on the bar's name

A few HyDE defaults refer to waybar by name rather than by role. None of them
break hydebar, but they are worth knowing:

| Place | Effect |
| --- | --- |
| `~/.local/share/hypr/lua/layer_rules.lua` (Lua chain) | blur and ignore-alpha layer rules target the `waybar` namespace; hydebar asks the compositor for its own blur rules at startup, so nothing is needed, but a hand-written rule should target the `hydebar-main-layer` namespace |
| `~/.config/hypr/workflows/*.conf` | the workflow `match:namespace` lists name `waybar`; add `hydebar-main-layer` to keep those workflows treating the bar the same way |
| `SUPER+CTRL+W` style keybindings | `hyde-shell waybar --hide` toggles waybar's unit, not hydebar |
| `hyde-shell waybar -n` layout cycling | waybar-only; hydebar layouts are edited in its own settings window |

## Colours, fonts and the theme

No wallbash template is needed. The bar reads the palette straight from
`~/.cache/hyde/wall.dcol`, the theme selection and fonts from
`~/.local/state/hyde/staterc` and the theme's `hypr.theme`, and repaints when
they change — whether the switch came from the bar's own theme module or from
`hyde-shell themeswitch` outside it.
