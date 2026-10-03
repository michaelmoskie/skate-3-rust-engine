#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PYTHON="${PYTHON:-/opt/homebrew/bin/python3.13}"
CARGO="${CARGO:-cargo}"
RUSTC="${RUSTC:-rustc}"
ARCH="$(uname -m)"
case "$ARCH" in
  arm64) RELEASE_TARGET="macos-arm64" ;;
  x86_64) RELEASE_TARGET="macos-x64" ;;
  *) echo "Unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
esac

PACKAGE_NAME="skate3rust-$RELEASE_TARGET"
STAGE="$PROJECT_ROOT/target/release-packages/$PACKAGE_NAME"
APP="$STAGE/Skate 3 Rust Engine.app"
CONTENTS="$APP/Contents"
VENV="$PROJECT_ROOT/target/package-venv-macos"
PACKAGE_PYTHON="$VENV/bin/python"
export PYINSTALLER_CONFIG_DIR="$PROJECT_ROOT/target/pyinstaller-config"
BUILD_NUMBER="${GITHUB_RUN_NUMBER:-0}"
RELEASE_TAG="${RELEASE_TAG:-development}"

if [[ "$(uname -s)" != Darwin ]]; then
  echo "This release script must run on macOS." >&2
  exit 1
fi
if [[ "$PYTHON" != */* ]]; then
  PYTHON="$(command -v "$PYTHON")"
fi
if [[ ! -x "$PYTHON" ]]; then
  echo "Python 3.13 is required: $PYTHON" >&2
  exit 1
fi
if ! "$PYTHON" -c 'import tkinter' >/dev/null 2>&1; then
  echo "Python Tk support is required (Homebrew: brew install python-tk@3.13)." >&2
  exit 1
fi

cd "$PROJECT_ROOT"
rm -rf "$STAGE" "$PROJECT_ROOT/target/macos-setup-source"
mkdir -p "$PYINSTALLER_CONFIG_DIR"
"$PYTHON" -m venv "$VENV"
"$PACKAGE_PYTHON" -m pip install -r tools/requirements-setup.txt
"$PACKAGE_PYTHON" -m unittest tools.test_setup_assets tools.asset_pipeline.test_marquee_assets \
  tools.asset_pipeline.test_customiser_setup tools.asset_pipeline.test_setup_recovery \
  tools.asset_pipeline.test_versions tools.asset_pipeline.test_optional_content

"$CARGO" build --release --locked -p skate-game --bin skate3rust --no-default-features
"$CARGO" build --release --locked -p skate-steam-relay
mkdir -p "$CONTENTS/MacOS/steam-relay" "$CONTENTS/Resources/support" "$CONTENTS/Resources/docs" \
  "$CONTENTS/Resources/licenses" "$PROJECT_ROOT/target/native"
cp packaging/macos/Info.plist "$CONTENTS/Info.plist"
cp target/release/skate3rust "$CONTENTS/MacOS/skate3rust"
cp target/release/skate-steam-relay "$CONTENTS/MacOS/steam-relay/skate-steam-relay"
STEAM_LIBRARY="$(find target/release/build -path '*/out/libsteam_api.dylib' -print -quit)"
test -n "$STEAM_LIBRARY"
cp "$STEAM_LIBRARY" "$CONTENTS/MacOS/steam-relay/libsteam_api.dylib"

"$RUSTC" --edition 2024 --crate-type cdylib -C opt-level=3 -C panic=abort \
  tools/asset_pipeline/refpack_native.rs -o target/native/refpack.dylib

SOURCE_STAGE="$PROJECT_ROOT/target/macos-setup-source"
mkdir -p "$SOURCE_STAGE"
git ls-files -z tools | rsync -a --from0 --files-from=- ./ "$SOURCE_STAGE/"
FBX_TOOL="$PROJECT_ROOT/target/importer-runtime-macos/FBX2glTF"
mkdir -p "$(dirname "$FBX_TOOL")"
curl -fL https://github.com/facebookincubator/FBX2glTF/releases/download/v0.9.7/FBX2glTF-darwin-x64 -o "$FBX_TOOL"
echo 'f82383ae4185c39f991b479b04ecce104f02e70c12a035ed31fc469e6f74a3fd  '"$FBX_TOOL" | shasum -a 256 -c -
chmod +x "$FBX_TOOL"

"$PACKAGE_PYTHON" -m PyInstaller --noconfirm --clean --onefile --name skate3setup \
  --paths "$PROJECT_ROOT" --hidden-import numpy --hidden-import PIL.Image --hidden-import tkinter \
  --add-binary "$PROJECT_ROOT/target/native/refpack.dylib:tools/asset_pipeline" \
  --add-binary "$FBX_TOOL:tools/mixamo_to_skate/tools" \
  --exclude-module bpy --exclude-module mathutils --copy-metadata numpy --copy-metadata Pillow \
  --copy-metadata PyInstaller --add-data "$SOURCE_STAGE/tools:tools" \
  --add-data "$PROJECT_ROOT/docs/images/skating-crab.png:docs/images" \
  --distpath "$CONTENTS/Resources/support" --workpath target/setup-build-macos/work \
  --specpath target/setup-build-macos tools/setup.py
"$PACKAGE_PYTHON" -m PyInstaller --noconfirm --clean --onefile --name skate3update \
  --paths "$PROJECT_ROOT/tools" --distpath "$CONTENTS/Resources/support" \
  --workpath target/updater-build-macos/work --specpath target/updater-build-macos tools/updater_macos.py

cp README.md "$CONTENTS/Resources/"
cp docs/macos.md "$CONTENTS/Resources/docs/"
cp tools/mixamo_to_skate/licenses/FBX2glTF.txt "$CONTENTS/Resources/licenses/"
cp tools/vendor/utt/LICENSE "$CONTENTS/Resources/licenses/UTT.txt"
cp tools/vendor/university/LICENSE-PROJECT.md "$CONTENTS/Resources/licenses/CustomEngineLayer.txt"
cp vendor/bevy_pbr/LICENSE-MIT "$CONTENTS/Resources/licenses/Bevy-MIT.txt"
cp vendor/bevy_pbr/LICENSE-APACHE "$CONTENTS/Resources/licenses/Bevy-APACHE.txt"
chmod +x "$CONTENTS/MacOS/skate3rust" "$CONTENTS/MacOS/steam-relay/skate-steam-relay" \
  "$CONTENTS/Resources/support/skate3setup" "$CONTENTS/Resources/support/skate3update"

# Sign nested code first, then seal the complete app. A Developer ID identity can
# be supplied by CI; '-' creates a local ad-hoc signature.
SIGN_IDENTITY="${CODESIGN_IDENTITY:--}"
SIGN_ARGS=(--force --sign "$SIGN_IDENTITY")
if [[ "$SIGN_IDENTITY" != - ]]; then
  SIGN_ARGS+=(--timestamp --options runtime)
fi
codesign "${SIGN_ARGS[@]}" "$CONTENTS/MacOS/steam-relay/libsteam_api.dylib"
codesign "${SIGN_ARGS[@]}" "$CONTENTS/MacOS/steam-relay/skate-steam-relay"
codesign "${SIGN_ARGS[@]}" "$CONTENTS/MacOS/skate3rust"
codesign "${SIGN_ARGS[@]}" "$CONTENTS/Resources/support/skate3setup"
codesign "${SIGN_ARGS[@]}" "$CONTENTS/Resources/support/skate3update"
codesign "${SIGN_ARGS[@]}" "$APP"
codesign --verify --deep --strict "$APP"

if [[ -n "${NOTARY_PROFILE:-}" ]]; then
  NOTARY_ARCHIVE="$PROJECT_ROOT/target/$PACKAGE_NAME-notary.zip"
  ditto -c -k --keepParent "$APP" "$NOTARY_ARCHIVE"
  xcrun notarytool submit "$NOTARY_ARCHIVE" --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$APP"
  codesign --verify --deep --strict "$APP"
  rm -f "$NOTARY_ARCHIVE"
fi

"$PACKAGE_PYTHON" tools/package_macos_release.py --project "$PROJECT_ROOT" --stage "$STAGE" \
  --target "$RELEASE_TARGET" --build "$BUILD_NUMBER" --tag "$RELEASE_TAG"
ZIP="$PROJECT_ROOT/target/$PACKAGE_NAME.zip"
(cd "$(dirname "$STAGE")" && /usr/bin/zip -qry "$ZIP" "$PACKAGE_NAME")
shasum -a 256 "$ZIP" | sed "s|$ZIP|$PACKAGE_NAME.zip|" > "$ZIP.sha256"
cp "$STAGE/release.json" "$PROJECT_ROOT/target/release.json"
echo "Release package: $ZIP"
