#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXECUTABLE="$PROJECT_ROOT/bin/macos/skate3rust"
ASSETS="${SKATE3_ASSETS:-$PROJECT_ROOT/assets}"

if [[ ! -x "$EXECUTABLE" ]]; then
  echo "Game is not built. Run scripts/Build-macOS.sh first." >&2
  exit 1
fi
if [[ ! -d "$ASSETS" ]]; then
  echo "Prepared assets are missing: $ASSETS" >&2
  echo "Set SKATE3_ASSETS to an installed assets directory." >&2
  exit 1
fi

export SKATE3_MODS="${SKATE3_MODS:-$PROJECT_ROOT/mods}"
exec "$EXECUTABLE" --assets "$ASSETS" "$@"
