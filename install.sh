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
