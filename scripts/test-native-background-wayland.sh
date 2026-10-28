#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --manifest-path native/linux/Cargo.toml --example background_cover
cp tests/native/panel-shell-test.js .data/panel-shell/share/gnome-shell/extensions/cstimer-panel-test@local/extension.js
export FACET_BACKGROUND_TEST=1
export FACET_PRIVATE_TEST=1
export FACET_TEST_LIBS="${LD_LIBRARY_PATH:-}"
exec env -u LD_LIBRARY_PATH dbus-run-session -- env LD_LIBRARY_PATH="$FACET_TEST_LIBS" python3 scripts/test-native-background-wayland.py
