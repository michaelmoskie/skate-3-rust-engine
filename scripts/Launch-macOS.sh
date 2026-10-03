#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXECUTABLE="$PROJECT_ROOT/bin/macos/skate3rust"

if [[ ! -x "$EXECUTABLE" ]]; then
  echo "Game is not built. Run scripts/Build-macOS.sh first." >&2
  exit 1
fi

# The game automatically uses the active installation created by setup under
# ~/Library/Application Support/Skate3RustEngine/data. Keep an explicit asset
# directory as an optional override for development or test fixtures.
if [[ -n "${SKATE3_ASSETS:-}" && ! -d "$SKATE3_ASSETS" ]]; then
  echo "SKATE3_ASSETS points to a missing directory: $SKATE3_ASSETS" >&2
  exit 1
fi

export SKATE3_MODS="${SKATE3_MODS:-$PROJECT_ROOT/mods}"
exec "$EXECUTABLE" "$@"
