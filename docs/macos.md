# macOS

macOS uses Metal rendering and Apple's GameController framework. Xbox,
PlayStation and compatible MFi controllers, including recent wired Xbox
controllers that expose vendor-defined USB HID reports, are translated into
the engine's existing Xbox packet format. Windows continues to use raw XInput.

## Development

Install the stable Rust toolchain, then build and launch with:

```sh
scripts/Build-macOS.sh
scripts/Launch-macOS.sh
```

The launcher automatically uses the active installation under
`~/Library/Application Support/Skate3RustEngine/data`. Set `SKATE3_ASSETS` only
when overriding that location for development or test assets. The game assets
are not part of this repository.

## Audio

The first gameplay-audio pass decodes the owned `WHEEL_SKID_BANK.abk` bank into
a local PCM WAV and plays it while the skater is moving. Install the decoder
once, then extract the asset into a prepared installation:

```sh
brew install vgmstream
python3 tools/asset_pipeline/audio_extract.py \
  --game-root "/path/to/extracted/Skate 3" \
  --assets "$HOME/Library/Application Support/Skate3RustEngine/data/installations/<id>/assets"
```

The decoded WAV stays in the local installation and is not included in source
control or release packages. More gameplay banks can be added through the same
owned-asset extraction path.

## Packaged application

Run `scripts/Build-macOS-Release.sh`. It creates an Apple Silicon application
bundle and ZIP in `target/`. Homebrew Python 3.13 and `python-tk@3.13` are used for the self-contained setup
and updater helpers. First launch accepts either an owned Skate 3 ISO or an
extracted folder containing `default.xex` and its adjacent `data` directory.

Player data is stored below
`~/Library/Application Support/Skate3RustEngine`, including installed mods and
custom characters. The release script applies an
ad-hoc signature for local testing. Public distribution can set
`CODESIGN_IDENTITY` to a Developer ID Application identity and `NOTARY_PROFILE`
to a configured `notarytool` keychain profile; the build then signs, notarizes
and staples the application before creating the final ZIP.

The bundled FBX converter is currently Intel-only upstream, so importing FBX
custom characters on Apple Silicon requires Rosetta. GLB imports and the game
itself run natively.
