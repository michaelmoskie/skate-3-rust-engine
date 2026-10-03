#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-release}"
CARGO="${CARGO:-cargo}"

case "$PROFILE" in
  debug) GAME_ARGS=(--no-default-features); RELAY_ARGS=() ;;
  release) GAME_ARGS=(--release --no-default-features); RELAY_ARGS=(--release) ;;
  *) echo "Usage: $0 [debug|release]" >&2; exit 2 ;;
esac

cd "$PROJECT_ROOT"
"$CARGO" build --locked -p skate-game --bin skate3rust "${GAME_ARGS[@]}"
"$CARGO" build --locked -p skate-steam-relay "${RELAY_ARGS[@]}"

OUTPUT="$PROJECT_ROOT/bin/macos"
mkdir -p "$OUTPUT/steam-relay"
cp "$PROJECT_ROOT/target/$PROFILE/skate3rust" "$OUTPUT/skate3rust"
cp "$PROJECT_ROOT/target/$PROFILE/skate-steam-relay" "$OUTPUT/steam-relay/skate-steam-relay"

STEAM_LIBRARY="$(find "$PROJECT_ROOT/target/$PROFILE/build" -path '*/out/libsteam_api.dylib' -print -quit)"
if [[ -z "$STEAM_LIBRARY" ]]; then
  echo "Cargo did not stage libsteam_api.dylib" >&2
  exit 1
fi
cp "$STEAM_LIBRARY" "$OUTPUT/steam-relay/libsteam_api.dylib"
chmod +x "$OUTPUT/skate3rust" "$OUTPUT/steam-relay/skate-steam-relay"
echo "Ready: $OUTPUT/skate3rust"
