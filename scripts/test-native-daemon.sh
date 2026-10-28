#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export FACET_PRIVATE_TEST=1
export FACET_TEST_LIBS="${LD_LIBRARY_PATH:-}"
exec env -u LD_LIBRARY_PATH dbus-run-session -- env LD_LIBRARY_PATH="$FACET_TEST_LIBS" python3 scripts/test-native-daemon.py
