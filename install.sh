#!/bin/bash
#
# Build the bar and install it, together with the helper scripts it looks for
# at runtime. With --hyde the bar is also registered as the session bar of a
# HyDE install, replacing waybar at the next session start.

set -euo pipefail

readonly SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${PREFIX:-/usr}"

# The command HyDE starts the session bar with. Written verbatim: the
# variable inside it is expanded by the shell that executes the line at
# session start, exactly as HyDE's own waybar launch line is.
readonly HYDE_LAUNCH='hyde-shell app -u hyde-$XDG_SESSION_DESKTOP-bar.scope -t scope -- hydebar'

REGISTER_HYDE=0
for argument in "$@"; do
  case "$argument" in
    --hyde) REGISTER_HYDE=1 ;;
    *)
      echo "unknown option: $argument" >&2
      echo "usage: $0 [--hyde]" >&2
      exit 2
      ;;
  esac
done

# Registers the bar as the one HyDE starts with the session.
#
# HyDE reads the launch line from ~/.config/hyde/config.toml: the Lua config
# chain applies the [desktop.start] table, the classic .conf chain the
# [hyprland-start] table, so both are written and the same install works
# whichever chain the machine runs. Nothing is edited when the file already
# declares a bar of its own — the keys to change are printed instead.
register_with_hyde() {
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/hyde"
  local config="$config_dir/config.toml"

  if [ ! -d "$config_dir" ]; then
    echo "no HyDE install found at $config_dir; skipping session registration" >&2
    return 0
  fi

  if [ -f "$config" ] && grep -Eq '^[[:space:]]*bar[[:space:]]*=' "$config"; then
    echo "$config already declares a bar; set these keys yourself to switch:" >&2
    printf '  [desktop.start]\n  bar = "%s"\n\n  [hyprland-start]\n  bar = "%s"\n' \
      "$HYDE_LAUNCH" "$HYDE_LAUNCH" >&2
    return 0
  fi

  if [ -f "$config" ]; then
    cp "$config" "$config.before-hydebar"
    echo "previous HyDE configuration kept as $config.before-hydebar"
  fi

  {
    printf '\n[desktop.start]\nbar = "%s"\n' "$HYDE_LAUNCH"
    printf '\n[hyprland-start]\nbar = "%s"\n' "$HYDE_LAUNCH"
  } >>"$config"

  echo "hydebar registered as the HyDE session bar in $config"
  echo "it starts with the next session; to switch this one over now:"
  echo '  systemctl --user stop "hyde-$XDG_SESSION_DESKTOP-bar.service" "hyde-$XDG_SESSION_DESKTOP-bar.scope"'
  echo "  $HYDE_LAUNCH"
}

cargo build --release

# The theme switch script is installed next to the binary, which is the first
# place the bar looks for it. 'hydebar-theme-switch' is the name it carries in
# a shared binary directory; inside a checkout the same file is
# 'scripts/theme-switch'.
sudo install -Dm755 "$SOURCE_DIR/target/release/hydebar-app" "$PREFIX/bin/hydebar"
sudo install -Dm755 "$SOURCE_DIR/scripts/theme-switch" "$PREFIX/bin/hydebar-theme-switch"

# Let the bus start the bar when a notification arrives and nothing is serving
# them. The name has a single owner, so this only takes effect in a session that
# runs no notification daemon of its own; where one is already started, the
# session decides, not this file.
sed "s|^Exec=.*|Exec=$PREFIX/bin/hydebar|" \
  "$SOURCE_DIR/assets/dbus/org.freedesktop.Notifications.service" |
  sudo install -Dm644 /dev/stdin \
    "$PREFIX/share/dbus-1/services/org.freedesktop.Notifications.service"

if [ "$REGISTER_HYDE" -eq 1 ]; then
  register_with_hyde
fi
