# HyDE theme switch: what the shell scripts do, and what hydebar has to do itself

Reverse-engineered from the HyDE installation on this machine (`~/.local/lib/hyde`,
`~/.local/share/wallbash`, `~/.local/bin/hyde-shell`). Line numbers refer to those files
as installed; they are stable enough to navigate by, but re-check them after a HyDE update.

The goal of this document is to make the `.sh` dependency removable step by step: it
records the exact sequence, what each step costs, which outputs hydebar actually consumes,
and which files hydebar must watch in order to recolour without a restart.

Command entry point used by the bar today:
`crates/hydebar-core/src/utils/hyde_shell.rs` → `scripts/theme-switch '<theme>'`,
which runs the sequence below with waybar's wallbash template excluded — see §6.

---

## 1. How `hyde-shell` resolves a command

`~/.local/bin/hyde-shell` is a dispatcher, not a program:

- `hyde-shell:498-508` — derives `BIN_DIR`, `LIB_DIR`, `SHARE_DIR` from its own path,
  prepends `$LIB_DIR/hyde`, `$BIN_DIR`, `~/.config/hyde/scripts` to `PATH`, and builds
  `HYDE_SCRIPTS_PATH` (`~/.config/hyde/scripts:~/.local/lib/hyde:~/.local/share/waybar/scripts:~/.config/waybar/scripts`).
- `hyde-shell:521-524` — `hyde-shell init` prints an env preamble and then `cat`s
  `globalcontrol.sh` (`hyde-shell:119-130`). Every HyDE script starts with
  `eval "$(hyde-shell init)"`, which is how the whole function library gets into scope.
- `hyde-shell:286-332` — `run_command` searches `HYDE_SCRIPTS_PATH` for `<name>.sh`,
  `<name>.lua`, `<name>.py` or an executable and `exec`s it. So `hyde-shell theme.switch`
  is literally `bash ~/.local/lib/hyde/theme.switch.sh`.
- `hyde-shell:176-207` — `hyde-shell wallbash <name>` finds `*/scripts/<name>.sh` inside
  the wallbash directories.

`globalcontrol.sh` is the shared library: XDG/HyDE path exports (`globalcontrol.sh:2-24`),
`print_log`, `get_hashmap` (`:99-176`), `get_themes` (`:177-207`), `export_hyde_config`
(`:208-214`, sources `~/.local/state/hyde/staterc`), `set_conf` (`:258-267`, writes
`staterc`), `get_hyprConf` (`:287-345`), `toml_write` (`:410-424`, `kwriteconfig6` with a
`sed` fallback). Sourcing it also queries Hyprland for `decoration:rounding` and
`general:border_size` (`:233-238`).

---

## 2. Full sequence of a theme switch

### Phase A — `theme.switch.sh` (the caller's process)

1. `theme.switch.sh:2` — `eval "$(hyde-shell init)"` loads the library and the current
   `staterc`.
2. `theme.switch.sh:4` — `get_themes` lists `~/.config/hyde/themes/*` (and repairs a broken
   `wall.set` symlink, `globalcontrol.sh:184-195`).
3. `theme.switch.sh:96-119` — option parsing: `-n`/`-p` pick the neighbouring theme
   (`Theme_Change`, `:6-19`), `-s` takes a name, `-q` silences the wallpaper step.
4. `theme.switch.sh:120-121` — validates the name and **writes**
   `HYDE_THEME="<name>"` into `~/.local/state/hyde/staterc` via `set_conf`.
   *This is the earliest observable signal of a theme switch.*
5. `theme.switch.sh:123-125` — `export reload_flag=1`, re-sources `globalcontrol.sh` (so
   `HYDE_THEME_DIR` now points at the new theme) and `~/.local/share/hyde/env-theme`
   (fallback GTK/icon/cursor/font values, including `BAR_FONT`).
6. `theme.switch.sh:126-137` — `hyprctl keyword misc:disable_autoreload 1` so the following
   file writes do not trigger a Hyprland reload per file.
7. `theme.switch.sh:139-149` — sanitises `"$HYDE_THEME_DIR/hypr.theme"` (drops `exec`,
   shadow keys, `HYPR_CONFIG_SANITIZE` patterns) into `~/.config/hypr/themes/theme.conf`;
   on a Lua config it dumps to `~/.local/state/hyde/lua_state/hypr_theme.lua` instead.
8. `theme.switch.sh:150-151` — `hyq` reads the theme variables (`$GTK_THEME`, `$ICON_THEME`,
   `$CURSOR_*`, `$FONT*`, `$MONOSPACE_*`) from `hypr.theme`, then from
   `~/.local/state/hyde/hyprland.conf` (user override wins).
9. `theme.switch.sh:155-157` — `dconf write /org/gnome/desktop/interface/icon-theme`.
10. `theme.switch.sh:164-183` — Qt/KDE writes through `toml_write`/`kwriteconfig6`:
    `~/.config/qt5ct/qt5ct.conf`, `~/.config/qt6ct/qt6ct.conf`, `~/.config/kdeglobals`,
    `~/.local/share/icons/default/index.theme`, `~/.icons/default/index.theme`.
11. `theme.switch.sh:184-194` — GTK2/3: `sed` over `~/.gtkrc-2.0`, `toml_write` into
    `~/.config/gtk-3.0/settings.ini`.
12. `theme.switch.sh:195-206` — GTK4: `rm -rf ~/.config/gtk-4.0` and re-`ln -s` the theme's
    `gtk-4.0` directory (falls back to `Wallbash-Gtk`).
13. `theme.switch.sh:207-217` — if flatpak is installed: `flatpak --user override …` plus a
    backgrounded `flatpak remote-add`.
14. `theme.switch.sh:218-248` — `~/.config/xsettingsd/xsettingsd.conf`, `~/.themes/<theme>`
    symlink, `~/.Xresources`, `~/.Xdefaults`, removal of `~/.config/gtk-4.0/settings.ini`.
15. `theme.switch.sh:250-257` — backgrounded: for every `~/.cache/hyde/wallpapers/*.png`
    whose basename is an installed package, re-link the per-backend wallpaper
    (`wallpaper.sh --link --backend <name>`).
16. `theme.switch.sh:258-262` — the actual handover:
    `wallpaper.sh -s "$(readlink "$HYDE_THEME_DIR/wall.set")" --global`.

### Phase B — `wallpaper.sh --global` (colour pipeline)

17. `wallpaper.sh:63-93` — `setup_wallpaper_targets` resolves the global targets:
    `~/.cache/hyde/wall.set`, `wall.sqre`, `wall.thmb`, `wall.blur`, `wall.quad`,
    **`wall.dcol`**.
18. `wallpaper.sh:168-175` — `-s` branch: `get_hashmap "<wallpaper>"` (sha1 of the file) →
    `Wall_Cache`.
19. `wallpaper/core.sh:60-66` — `Wall_Cache` sets `reload_flag=1` and re-points
    `~/.cache/hyde/wall.set` and `"$HYDE_THEME_DIR/wall.set"` at the new image.
20. `wallpaper/core.sh:69` — **synchronous** `wallpaper/cache.sh commence -w <wallpaper>`:
    `get_hashmap` + `parallel fn_wallcache` (`cache.sh:38-61`). For an uncached image this
    runs four `magick` passes (thmb/sqre/blur/quad) and then `wallbash.sh` to produce
    `~/.cache/hyde/dcols/<sha1>.dcol`. For a cached image every step is skipped.
21. `wallpaper/core.sh:70` — **backgrounded** `color.set.sh <wallpaper> &` — the entire
    recolouring runs detached from the switch script.
22. `wallpaper/core.sh:71-75` — re-points `~/.cache/hyde/wall.sqre|thmb|blur|quad|**dcol**`
    at `thumbs/<sha1>.*` and `dcols/<sha1>.dcol`.
23. `wallpaper.sh:218-228` — runs the wallpaper backend, here
    `wallpaper.awww.sh` → `awww img … --transition-duration 0.4` (backgrounded).

`wallbash.sh` (colour extraction, only on a cache miss): `magick -kmeans 4` over the 1000px
thumbnail (`wallbash.sh:101-106`), dark/light decision from mean luminance (`:107-115`),
then for each of the 4 primaries a text colour and a 9-step accent ramp
(`:122-155`) written as `dcol_*` shell assignments — 89 lines total.

### Phase C — `color.set.sh` (runs detached, this is the "theme change" everyone sees)

24. `color.set.sh:4-17` — disables Hyprland autoreload again and installs
    `trap 'hyprctl reload -q' EXIT`.
25. `color.set.sh:206-211` — resolves `~/.cache/hyde/dcols/<sha1>.dcol`, regenerates it if
    missing, and sources it (`dcol_mode`, `dcol_pry1..4`, `dcol_txt1..4`, `dcol_<n>xa1..9`,
    plus `_rgba` variants).
26. `color.set.sh:212-217` — when `enableWallDcol=0` and the theme ships
    `"$HYDE_THEME_DIR/theme.dcol"`, that file **overrides** the wallpaper colours;
    `dcol_invt` is the opposite of `dcol_mode`.
27. `color.set.sh:105-109, 58-104` — `preprocess_substitutions` builds two giant `sed`
    scripts (normal and inverted) mapping `<wallbash_pryN>`, `<wallbash_txtN>`,
    `<wallbash_NxaM>` and their `_rgba(<alpha>)`/`_rgb` forms to concrete values.
28. `color.set.sh:221-225` — `revert_colors` decision: with `enableWallDcol=0` the colours
    are inverted when the theme's `COLOR_SCHEME` disagrees with `dcol_mode`.
29. `color.set.sh:226` → `load_dconf_kdeglobals` (`:31-57`):
    - 11 × `toml_write` into `~/.config/kdeglobals` (`Colors:*`) and
      `~/.config/Kvantum/wallbash/wallbash.kvconfig`;
    - `color/hypr.sh` — writes `~/.config/hypr/themes/wallbash.conf` (hyprlang) or
      `~/.local/state/hyde/lua_state/ui.lua` (Lua config; contains `bar_font`, `font_size`,
      `icon_theme`, `wallbash.mode`);
    - `color/dconf.lua` — gsettings/dconf writes and `hyprctl setcursor`;
    - `shaders.lua --reload` — recompiles `~/.local/state/hyde/compiled.cache.glsl`.
30. `color.set.sh:249-256` — template deployment, **theme scope**. With `enableWallDcol=0`
    the theme's own `*.theme` files are used; with `enableWallDcol>0` (this machine:
    `enableWallDcol="1"`) it is every `*/theme*/**.dcol` found in
    `WALLBASH_DIRS` (`globalcontrol.sh:224-231`), deduplicated by basename, rendered with
    `parallel fn_wallbash`. 14 templates here.
31. `color.set.sh:261` — template deployment, **always scope**: every `*/always*/**.dcol`,
    43 templates here (of which 26 are the `00-icons/*` SVG icons).
32. `color.set.sh:110-163` — `fn_wallbash` per template: line 1 of the template is
    `target_path|exec_command`; the body is `sed`-substituted into a temp file, `mv`d onto
    the target, and `exec_command` is run **backgrounded and disowned**.
33. `color.set.sh` EXIT trap — `hyprctl reload -q`, which is what makes Hyprland pick up
    `~/.config/hypr/themes/theme.conf`, `colors.conf` and `wallbash.conf` at once.

### Phase D — the consumers woken up by the templates

| template | target file | exec command |
| --- | --- | --- |
| `~/.local/share/wallbash/theme/waybar.dcol:1` | `~/.config/waybar/theme.css` | `pgrep -x waybar && hyde-shell waybar --update` |
| `~/.local/share/wallbash/theme/swaync.dcol:1` | `~/.config/swaync/theme.css` | `swaync-client -R && swaync-client -rs` |
| `~/.local/share/wallbash/theme/kitty.dcol:1` | `~/.config/kitty/theme.conf` | `scripts/kitty.sh` → `killall -SIGUSR1 kitty` |
| `~/.local/share/wallbash/always/dunst.dcol:1` | `~/.cache/hyde/wallbash/dunst.conf` | `scripts/dunst.sh` → rewrites `~/.config/dunst/dunstrc`, `killall dunst; dunst &` |
| `~/.local/share/wallbash/always/qtct.dcol:1` | `~/.cache/hyde/wallbash/qtct.conf` | `scripts/qtct.sh` |
| `~/.local/share/wallbash/always/hyprlock_background.dcol:1` | `/tmp/null` | `hyprlock.sh --background` → copies the 3 MB `~/.cache/hyde/wall.set.png` |
| `~/.local/share/wallbash/always/hyprcolors.dcol:1` | `~/.config/hypr/themes/colors.conf` | — (picked up by the final `hyprctl reload`) |
| `~/.local/share/wallbash/always/lua.dcol:1` | `~/.local/state/hyde/lua_state/colors.lua` | — |
| `~/.local/share/wallbash/always/rasi.dcol`, `scss.dcol`, `gtk-css.dcol`, `shell-colors.dcol` | `~/.cache/hyde/wallbash/{wallbash.rasi,colors.scss,gtk.css,shell-colors}` | — |
| `~/.config/hyde/wallbash/always/wayle.dcol:1` | `/dev/null` | `scripts/wayle-theme.sh` → pushes `dcol_*` into another Rust bar via its own CLI |
| `~/.config/hyde/wallbash/theme/code.dcol`, `always/{vim,chrome,spotify,discord,cava}.dcol` | app configs | app-specific reload scripts |

`hyde-shell waybar --update` (`waybar.py:1285-1329`) runs
`update_icon_size` → `update_border_radius` → `generate_includes` → `update_global_css`
**twice** (once for `--update`, once in the fall-through `else` branch) and finishes with
`update_style` + `restart_waybar` (`waybar.py:555-562`: `SIGUSR2`, else
`systemctl --user restart`/`pkill` + start). It writes:
`~/.config/waybar/includes/global.css`, `includes/border-radius.css`, `includes/includes.json`,
`style.css`, and `WAYBAR_*` keys in `staterc`.

Font resolution inside `waybar.py` (worth mirroring, `waybar.py:888-925`):
- family: `config.toml WAYBAR_FONT` → `hypr.theme $BAR_FONT` → `staterc BAR_FONT` →
  `"JetBrainsMono Nerd Font"`;
- size: `config.toml WAYBAR_SCALE` → `staterc BAR_FONT_SIZE` → `hypr.theme $BAR_FONT_SIZE` → `10`;
- icon size: `WAYBAR_ICON_SIZE` → `WAYBAR_SCALE` → `staterc BAR_ICON_SIZE` → `hypr.theme $BAR_ICON_SIZE` → `10`.

---

## 3. Cost per step, and whether the bar needs it

Measured on this machine with a warm cache (12 themes, 9 wallpapers in the active theme,
57 wallbash templates totalling 900 KB). Entries marked *(not run)* would have mutated the
live session, so they are engineering estimates, not measurements.

| # | Step | Cost | Needed by hydebar |
| --- | --- | --- | --- |
| 1 | `eval "$(hyde-shell init)"` + `globalcontrol.sh` | 9 ms | no — replaced by native path resolution |
| 2 | `get_themes` over 12 themes | 22 ms | yes (theme list) — already native, `hyde_state/themes.rs` |
| 4 | `set_conf HYDE_THEME` → `staterc` | < 1 ms | **yes — the switch signal** |
| 5 | re-source `globalcontrol` + `env-theme` | 9 ms | partially (font fallbacks) |
| 6, 24 | `hyprctl keyword misc:disable_autoreload` | ~2 ms per call | no |
| 7 | sanitise `hypr.theme` → `hypr/themes/theme.conf` | ~15 ms | no |
| 8 | `hyq` variable reads | 2 ms per query | yes (fonts) if we stop reading waybar CSS |
| 9-14 | dconf + 16 × `kwriteconfig6` + GTK/Qt/X writes | 75 ms measured for 16 `kwriteconfig6`; `flatpak --user override` *(not run)* dominates, ~200-500 ms | no — GTK/Qt/flatpak consumers only |
| 15 | per-backend wallpaper relink (backgrounded) | off critical path | no |
| 18, 20 | sha1 + `cache.sh commence -w` (warm cache) | 25 ms hashing + 86 ms `parallel` startup | no |
| 20 | `wallbash.sh` colour extraction (cold cache only) | **467 ms** + ~0.3-0.8 s of `magick` thumbnails | indirectly — it produces `wall.dcol` |
| 19, 22 | `wall.*` symlink re-point | < 1 ms | **yes — `wall.dcol` is the colour source** |
| 23 | `awww img` transition | 0.4 s, backgrounded | no |
| 25-28 | source `dcol` + build the two `sed` scripts | ~10 ms | conceptually yes (same mapping, in Rust) |
| 29 | kdeglobals/Kvantum + `hypr.sh` + `dconf.lua` + `shaders.lua --reload` | ~50 ms of writes; GLSL recompile *(not run)*, ~100-300 ms | no |
| 30, 31 | render 57 templates | 190 ms with `parallel`, 372 ms serial (+86 ms `parallel` startup × 2) | only `waybar.dcol` matters today |
| 32 | backgrounded exec commands (dunst restart, kitty signal, swaync reload, code/chrome/spotify/discord/cava/wayle) | each 50 ms-1 s, all detached | no |
| 32 | `hyde-shell waybar --update` → **waybar restart** | Python start 39 ms + the double `--update` work + a full waybar restart *(not run)*, typically 0.3-1.5 s and the visible flash | **only as a side effect**: it is what refreshes `global.css` and `border-radius.css` that hydebar reads |
| 33 | `hyprctl reload -q` | *(not run)*, typically 50-300 ms | no |

**Where the latency actually is.** Nothing in the shell plumbing is slow: the library, the
hashing, the `kwriteconfig6` writes and the whole 57-template render add up to well under
a second. The wall-clock delay comes from (a) `wallbash.sh` + `magick` when the wallpaper
has never been cached (~1 s), and (b) the consumer restarts fanned out in step 32 —
above all the waybar restart, plus dunst being killed and respawned and the final
`hyprctl reload`. A native bar pays none of (b) and can skip (a) by reading the `.dcol`
that already exists.

**The important asymmetry for hydebar:** `theme.css` is written by the template itself, but
`global.css` and `border-radius.css` are written by `waybar.py` and only when
`pgrep -x waybar` succeeds (`waybar.dcol:1`). The moment waybar is not running — the actual
goal of this project — hydebar's font and radius sources go stale forever. That alone
justifies stage 2 below.

---

## 4. What to move to Rust

Current state: stages 1–3 below have landed.
`crates/hydebar-proto/src/theme_source/` reads the palette from
`~/.cache/hyde/wall.dcol` (`dcol.rs`), resolves the font natively through the
HyDE chain (`font.rs`), takes the radius from the compositor
(`compositor_look.rs`) and keeps the waybar stylesheets (`waybar.rs`,
`css.rs`) only as a last resort; `hyde_state/` reads `staterc` and the themes
directory. A dedicated theme watcher
(`crates/hydebar-core/src/config/theme_watch/`) follows the HyDE files and
re-runs the config load on any change, so the bar repaints on a HyDE theme
switch without its own config being touched. The stage descriptions are kept
below as the record of why each step was taken.

### Stage 1 — read the colours from the source, not from waybar's stylesheet ✅ done

Add a `dcol` reader to `hydebar-proto`: parse `~/.cache/hyde/wall.dcol` (89 lines of
`key="value"`, the same grammar `staterc.rs` already handles) into a palette
`{ mode, pry[4], txt[4], accents[4][9] }`. Hex values are in `dcol_pryN` / `dcol_txtN` /
`dcol_NxaM` without `#`; the `_rgba` variants carry a literal `\1` alpha placeholder and
should be ignored — apply alpha in Rust.

Reproduce waybar's mapping exactly so nothing changes visually
(`~/.local/share/wallbash/theme/waybar.dcol`):

| bar role | dcol key | alpha |
| --- | --- | --- |
| bar background | `dcol_pry1` | 0.01 |
| module background | `dcol_pry1` | 0.8 |
| text | `dcol_1xa8` | 0.8 |
| active background | `dcol_pry4` | 0.4 |
| active text | `dcol_4xa9` | 1.0 |
| hover background | `dcol_2xa3` | 0.4 |
| hover text | `dcol_3xa8` | 0.8 |

Also port the two rules that decide *which* colours apply
(`color.set.sh:212-217` and `:150-154`): with `enableWallDcol=0` prefer
`~/.config/hyde/themes/<theme>/theme.dcol` over the wallpaper `.dcol`, and invert the
palette (index `5-i`, `dcol_invt` instead of `dcol_mode`) when `revert_colors` applies —
`enableWallDcol=2` with `dcol_mode=light`, `enableWallDcol=3` with `dcol_mode=dark`, or
`enableWallDcol=0` with a theme `COLOR_SCHEME` that disagrees with `dcol_mode`.

Keep the waybar stylesheet reader as a fallback for setups without a `.dcol`.

### Stage 2 — watch, and repaint without a config reload ✅ done

Generalise `config/watch` from "one file" to "a set of paths", and add a theme watcher that
emits a palette message instead of a full config reload. See §5 for the exact paths and the
inotify subtleties. This is the change that removes the perceived delay entirely: the bar
repaints as soon as `wall.dcol` moves, without waiting for any consumer script.

### Stage 3 — fonts and radius natively, dropping the waybar files ✅ done

- Font family/size: implement the `waybar.py:888-911` chain in Rust —
  `~/.config/hyde/config.toml` (`WAYBAR_FONT`/`WAYBAR_SCALE`) →
  `~/.config/hyde/themes/<theme>/hypr.theme` (`$BAR_FONT`, `$BAR_FONT_SIZE`) →
  `staterc` (`BAR_FONT`, `BAR_FONT_SIZE`) → `~/.local/share/hyde/env-theme` →
  `"JetBrainsMono Nerd Font"` / `10`. A `hypr.theme` line is `$NAME = value`; a tiny
  key/value scan is enough, `hyq` is not needed.
  When the Hyprland config is Lua, `~/.local/state/hyde/lua_state/ui.lua` already carries
  `bar_font`/`font_size` in a flat table (`color/hypr.sh:84-120`).
- Radius: `compositor_look.rs` already reads `decoration:rounding` (2 ms). Prefer it over
  `border-radius.css`.

After this stage nothing under `~/.config/waybar` is required and waybar can be uninstalled.

### Stage 4 — become a first-class wallbash consumer

Ship `~/.config/hyde/wallbash/always/hydebar.dcol`, modelled on `wayle.dcol` (target
`/dev/null` plus a script, or a real target file). Two viable shapes:

- **passive** — render a `~/.config/hydebar/colors.toml` from the template and let the
  stage-2 watcher pick it up. No process is spawned, nothing has to be running.
- **active** — an exec script that pokes a running bar, the way `wayle-theme.sh` does.
  Only worth it once hydebar has an IPC surface.

Passive is the better default: it keeps the bar's colours correct even while the bar is not
running, and it is one `sed` render (~3 ms) on the wallbash side.

### Stage 5 — perform the switch from the bar

Done differently than sketched here: instead of a native `staterc` write, the
bar runs its own `scripts/theme-switch` wrapper (§6), which performs the stock
HyDE switch with the waybar templates excluded. The native split below remains
an option if the shell dependency should ever go entirely:

1. natively write `HYDE_THEME="<name>"` into `~/.local/state/hyde/staterc`
   (the `set_conf` semantics of `globalcontrol.sh:258-267`: replace the line if the key
   exists exactly once, otherwise append);
2. natively resolve the new theme's wallpaper (`~/.config/hyde/themes/<theme>/wall.set` →
   `readlink` → sha1 → `~/.cache/hyde/dcols/<sha1>.dcol`) and repaint immediately — this is
   sub-millisecond when the cache is warm;
3. hand the rest of the desktop (GTK/Qt/Hyprland/other apps) to the existing script in the
   background, ideally with the wallpaper step told not to redo what we just did.

The bar then recolours instantly and the rest of the session catches up on its own
schedule. Do not port steps 9-14 or 29-33: they exist for GTK, Qt, Hyprland, dunst, kitty
and waybar, none of which the bar owns.

### Stage 6 — optional, native colour extraction

Only if hydebar ever wants to change wallpaper by itself: reimplement `wallbash.sh`
(k-means over the thumbnail, luminance-based dark/light choice, the HSB accent ramp) in
Rust to remove the ~470 ms `magick` fan-out and write `dcols/<sha1>.dcol` in the same
format so the rest of HyDE stays compatible. Lowest priority — it is a cold-cache-only cost.

---

## 5. Files to watch

Written on every theme switch, in the order they change:

| # | Path | Written by | Why it matters |
| --- | --- | --- | --- |
| 1 | `~/.local/state/hyde/staterc` | `theme.switch.sh:121` (`set_conf`) | first signal; carries `HYDE_THEME`, `enableWallDcol`, `HYPR_SHADER`, `BAR_FONT*`, `WAYBAR_*` |
| 2 | `~/.cache/hyde/wall.set` (symlink) | `wallpaper/core.sh:65-66` | current wallpaper |
| 3 | **`~/.cache/hyde/wall.dcol`** (symlink → `~/.cache/hyde/dcols/<sha1>.dcol`) | `wallpaper/core.sh:75` | **the palette — the one file the bar really needs** |
| 4 | `~/.config/hypr/themes/{theme,colors,wallbash}.conf` | `theme.switch.sh:146`, `hyprcolors.dcol`, `color/hypr.sh:127` | Hyprland side; only interesting for `$BAR_FONT` fallbacks |
| 5 | `~/.local/state/hyde/lua_state/{ui,colors,hypr_theme}.lua` | `color/hypr.sh:84`, `lua.dcol`, `theme.switch.sh:143` | the Lua-config equivalent of #4 |
| 6 | `~/.config/waybar/theme.css` | `theme/waybar.dcol` | what hydebar reads today |
| 7 | `~/.config/waybar/includes/global.css`, `includes/border-radius.css`, `includes/includes.json`, `~/.config/waybar/style.css` | `waybar.py:812-846`, `:980-1043` | fonts/radius today — **only refreshed while waybar is running** |
| 8 | `~/.cache/hyde/wallbash/*` (`colors.scss`, `gtk.css`, `wallbash.rasi`, `shell-colors`, `dunst.conf`, `qtct.conf`, `colors.inc`) | `always/*.dcol` | other consumers; `shell-colors` is a usable secondary palette source |
| 9 | `~/.config/hyde/themes/<theme>/wall.set` | `wallpaper/core.sh:65` | per-theme current wallpaper |

**What hydebar should watch, minimally:**

1. `~/.cache/hyde/wall.dcol` — the palette. Because it is a **symlink that is replaced**
   (`ln -fs`), watch the *directory* `~/.cache/hyde` for `CREATE|MOVED_TO|DELETE` on the
   name `wall.dcol`, exactly like `config/watch/recipe.rs:74-80` already watches a parent
   directory, and re-`read_link` on every hit. Watching the symlink itself, or the resolved
   `dcols/<sha1>.dcol`, will miss the switch.
2. `~/.local/state/hyde/staterc` — theme name, `enableWallDcol`, shader, `BAR_FONT*`.
   Rewritten in place by `sed -i` and appended to by `waybar.py`, so `MODIFY` on the
   `~/.local/state/hyde` directory is the reliable trigger.
3. `~/.config/hyde/themes/<theme>/hypr.theme` — only when stage 3 lands (fonts); it does not
   change during a switch, only the *selection* does, so re-reading it on a `staterc` change
   is sufficient.
4. Optional, once stage 4 lands: `~/.config/hydebar/colors.toml` written by our own wallbash
   template.

Debouncing matters: step 30-31 rewrites 57 files in parallel and several consumers fire
afterwards, so a switch produces a burst of events. The existing
`ready_chunks(10)` batching in `config/watch/recipe.rs:96` is the right shape; keep a short
coalescing window (~50 ms) before repainting.

**Do not watch** `~/.config/waybar/*` once stage 3 is done, and never depend on a consumer
script having run — every exec command in step 32 is backgrounded and disowned, so ordering
between them is not guaranteed.

---

## 6. `scripts/theme-switch` — the switch the bar performs

The bar no longer calls `hyde-shell theme.switch`. It calls `scripts/theme-switch`, which
performs the very same switch minus waybar.

**Why a script and not the dispatcher.** Nothing in `theme.switch.sh`, `wallpaper.sh`,
`wallbash.sh` or `color.set.sh` mentions waybar — grep them and the only hits are unrelated
(`gpuinfo.sh` help text, `hyprsunset.sh --sigproc`). The whole waybar involvement is one
wallbash template, `~/.local/share/wallbash/theme/waybar.dcol`, whose first line is

```
$HOME/.config/waybar/theme.css|pgrep -x waybar > /dev/null 2>&1 && hyde-shell waybar --update
```

Rendering it writes waybar's `theme.css` and, whenever waybar happens to be running, runs
`hyde-shell waybar --update`, which rewrites `style.css` and the `includes/` files twice and
then restarts waybar (`waybar.py:555-562`, `:1285-1329`). A theme that ships its own
`waybar.theme` reaches the same template through the `enableWallDcol=0` deploy list
(`color.set.sh:249-256`). Forking `theme.switch.sh` would therefore produce a byte-identical
copy: the difference has to be made where the template is rendered, not where the switch
starts.

**How it is made.** `color.set.sh:134-141` already honours `WALLBASH_SKIP_TEMPLATE`, a list
of bash regexes matched against each template path. The script exports

```
WALLBASH_SKIP_TEMPLATE='/waybar\.(dcol|theme)$'
```

and then execs the stock switch. A scalar environment variable is read back as a one-element
array, and it survives the whole chain — `theme.switch.sh` → `wallpaper.sh` →
`wallpaper/core.sh` → `color.set.sh` → `parallel` — down to `fn_wallbash`. Of the 63
templates on this machine exactly two match, both of them waybar's; the other 61, the palette,
`staterc`, the wallpaper, the GTK/Qt/dconf/Hyprland notifications and every other consumer are
untouched. The one way to lose the exclusion is a `WALLBASH_SKIP_TEMPLATE` of one's own in
`~/.local/state/hyde/{state,config}`, which `fn_wallbash` re-sources per template; the script
warns when it finds one.

On top of that the script validates the theme name against
`$XDG_CONFIG_HOME/hyde/themes` and prints the installed themes on a miss, where the stock
script silently falls back to the current theme, and it answers `-l` with the theme list in
HyDE's own order. It resolves HyDE through `hyde-shell` — `PATH` first, then
`~/.local/bin`, `/usr/local/bin`, `/usr/bin` — and derives `theme.switch.sh` from there, so
no path of a particular machine is baked in.

**Where the bar finds it.** `utils/hyde_shell/theme_script.rs` searches, in order:
`$HYDEBAR_THEME_SWITCH`, `<binary directory>/hydebar-theme-switch`,
`<binary directory>/../share/hydebar/scripts/theme-switch`,
`$XDG_DATA_HOME/hydebar/scripts/theme-switch`, the same path under `/usr/local/share` and
`/usr/share`, and finally `<binary directory>/../../scripts/theme-switch` for a checkout.
The first executable file wins; when none exists the switch is not attempted at all and the
log names every path that was tried. `install.sh` installs the script as
`$PREFIX/bin/hydebar-theme-switch`, next to the bar.
