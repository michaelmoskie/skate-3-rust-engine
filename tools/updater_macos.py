"""macOS entry point for the shared release updater."""
import os
import platform

machine = platform.machine().lower()
os.environ['SKATE_UPDATE_TARGET'] = 'macos-arm64' if machine in ('arm64', 'aarch64') else 'macos-x64'

from updater import main

if __name__ == '__main__':
    main()
