#!/bin/bash
#
# Build the bar and install it, together with the helper scripts it looks for
# at runtime.

set -euo pipefail

readonly SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${PREFIX:-/usr}"

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
