"""Extract the first playable Skate 3 wheel effect from an owned game install.

The Xbox 360 bank stores XMA2 streams.  vgmstream decodes the format to PCM
WAV; the generated file remains local to the user's prepared installation and
is never part of a release archive.
"""
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tools.owned_game.big import BigArchive

ARCHIVE = Path("data/audio/audiofiles.big")
ENTRY = "data/audio/WHEEL_SKID_BANK.abk"
OUTPUT = Path("private/audio/wheel-skid.wav")


def decoder() -> str:
    configured = os.environ.get("VGMSTREAM_CLI")
    candidate = configured or shutil.which("vgmstream-cli")
    if not candidate:
        raise RuntimeError(
            "Audio extraction needs vgmstream-cli. Install it with `brew install vgmstream` "
            "or set VGMSTREAM_CLI to its executable path."
        )
    return candidate


def extract(game_root: Path, assets: Path) -> Path:
    archive_path = game_root / ARCHIVE
    if not archive_path.is_file():
        raise RuntimeError(f"Missing owned audio bank: {archive_path}")
    target = assets / OUTPUT
    if target.is_file() and target.stat().st_size > 44:
        return target
    archive = BigArchive(archive_path)
    entry = next((entry for entry in archive.entries if entry.path == ENTRY), None)
    if entry is None:
        raise RuntimeError(f"Audio bank does not contain {ENTRY}")
    target.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="skate3-audio-") as directory:
        source = Path(directory) / "wheel-skid.abk"
        source.write_bytes(archive.read(entry))
        temporary = Path(directory) / "wheel-skid.wav"
        subprocess.run(
            [decoder(), "-i", "-s", "1", "-o", str(temporary), str(source)],
            check=True,
        )
        if not temporary.is_file() or temporary.stat().st_size <= 44:
            raise RuntimeError("vgmstream did not produce a playable wheel WAV")
        temporary.replace(target)
    return target


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game-root", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    args = parser.parse_args()
    print(extract(args.game_root, args.assets))


if __name__ == "__main__":
    main()
