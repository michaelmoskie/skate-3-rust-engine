"""Finalize a staged macOS release directory and write its update manifest."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import sys


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--project', type=Path, required=True)
    parser.add_argument('--stage', type=Path, required=True)
    parser.add_argument('--target', choices=('macos-arm64', 'macos-x64'), required=True)
    parser.add_argument('--build', type=int, default=0)
    parser.add_argument('--tag', default='development')
    args = parser.parse_args()
    project = args.project.resolve()
    stage = args.stage.resolve()
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=project, text=True).strip()
    versions = json.loads(subprocess.check_output([
        sys.executable, project/'tools/asset_pipeline/versions.py', '--tools', project/'tools'
    ], text=True))
    customiser = subprocess.check_output([
        sys.executable, '-m', 'tools.asset_pipeline.customiser_setup', '--fingerprint'
    ], cwd=project, text=True).strip()
    equivalence_path = project/'tools/asset_pipeline/pipeline-equivalence.json'
    equivalence = json.loads(equivalence_path.read_text(encoding='utf-8')) if equivalence_path.is_file() else {}
    files = {}
    for path in sorted(stage.rglob('*')):
        if path.is_file() and path.name != 'release.json':
            files[path.relative_to(stage).as_posix()] = digest(path)
    manifest = {
        'schema': 1,
        'repository': 'SK8-ENGINE/skate-3-rust-engine',
        'target': args.target,
        'build': args.build,
        'tag': args.tag,
        'revision': revision,
        'files': files,
        'asset_pipelines': versions,
        'character_customiser': customiser,
        'pipeline_equivalence': equivalence,
    }
    (stage/'release.json').write_text(json.dumps(manifest, indent=2, sort_keys=True)+'\n', encoding='utf-8')


if __name__ == '__main__':
    main()
