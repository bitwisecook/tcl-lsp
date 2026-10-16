#!/usr/bin/env python3
"""Restore only the 158 inventoried .elf.gz blobs from a pinned raw ref.

Default is a read-only plan/preflight. --apply downloads to /tmp, verifies every
byte against Git blob SHA-1 plus the existing census SHA-256/length, then installs
files using a same-directory temp file and os.replace. It never invokes payloads,
changes the Git index, writes Git objects, edits the inventory, or refreshes hashes.
"""
from __future__ import annotations
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

REPO = Path('/workspace/tcl-lsp')
DIAGNOSTIC = Path('/tmp/naming-proof-census-path-diagnostic.json')
INVENTORY = REPO / 'docs/design/analysis/name-resolution-proofs/corpus-inventory.json'
STAGE = Path('/tmp/naming-proof-elf-gz-stage')
RESULT = Path('/tmp/naming-proof-elf-gz-restoration-result.json')
SOURCE_REF = '554085e162823ea8bd126898d0c27b7b2a9f8628'
RAW_BASE = 'https://raw.githubusercontent.com/bitwisecook/tcl-lsp/'


def git_blob_oid(data: bytes) -> str:
    header = f'blob {len(data)}\0'.encode()
    return hashlib.sha1(header + data).hexdigest()


def load_plan() -> tuple[list[dict], dict[str, dict]]:
    diagnostic = json.loads(DIAGNOSTIC.read_text())
    entries = diagnostic['missing_elf_gz']['records']
    inventory = json.loads(INVENTORY.read_text())
    inv = {row['path']: row for row in inventory['files']}
    if len(entries) != 158 or len({e['path'] for e in entries}) != 158:
        raise RuntimeError('expected exactly 158 unique archived paths')
    for e in entries:
        row = inv.get(e['path'])
        if row is None or row.get('candidate') is not False:
            raise RuntimeError(f"missing/non-binary inventory row: {e['path']}")
        if row.get('bytes') != e['compressed_bytes']:
            raise RuntimeError(f"cached size disagrees with inventory: {e['path']}")
        if not e['path'].endswith('.elf.gz'):
            raise RuntimeError(f"unexpected non-ELF path: {e['path']}")
    return entries, inv


def index_entry(path: str, expected_oid: str) -> int:
    result = subprocess.run(
        ['git', 'ls-files', '--stage', '--', path], cwd=REPO,
        check=True, text=True, capture_output=True,
    )
    rows = [line for line in result.stdout.splitlines() if line.endswith('\t' + path)]
    if len(rows) != 1:
        raise RuntimeError(f'expected one index row for {path!r}, got {len(rows)}')
    fields = rows[0].split()
    mode, oid = fields[0], fields[1]
    if oid != expected_oid:
        raise RuntimeError(f'index Git blob ID differs from retained tree metadata: {path}')
    if mode not in {'100644', '100755'}:
        raise RuntimeError(f'unexpected Git mode {mode} for {path}')
    return int(mode, 8)


def fetch_and_verify(entry: dict, census_row: dict, apply: bool) -> dict:
    rel = entry['path']
    target = REPO / rel
    mode = index_entry(rel, entry['git_blob_sha1'])
    if target.exists():
        data = target.read_bytes()
        verify_bytes(rel, data, entry, census_row)
        if stat.S_IMODE(target.stat().st_mode) != (mode & 0o777):
            raise RuntimeError(f'existing target mode differs from index: {rel}')
        return {'path': rel, 'state': 'already-present-and-verified', 'mode': oct(mode)}
    if not apply:
        return {'path': rel, 'state': 'missing; exact metadata and inventory preflight passed', 'mode': oct(mode)}

    stage_file = STAGE / rel
    stage_file.parent.mkdir(parents=True, exist_ok=True)
    url = RAW_BASE + SOURCE_REF + '/' + urllib.parse.quote(rel, safe='/')
    data = None
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'name-proof-bounded-restoration'})
            with urllib.request.urlopen(req, timeout=60) as response:
                data = response.read(entry['compressed_bytes'] + 1)
            verify_bytes(rel, data, entry, census_row)
            stage_file.write_bytes(data)
            break
        except Exception:
            if stage_file.exists():
                stage_file.unlink()
            if attempt == 3:
                raise
            time.sleep(attempt + 1)
    assert data is not None
    return {'path': rel, 'state': 'staged-and-verified', 'mode': oct(mode), 'bytes': len(data)}


def verify_bytes(path: str, data: bytes, entry: dict, census_row: dict) -> None:
    if len(data) != entry['compressed_bytes'] or len(data) != census_row['bytes']:
        raise RuntimeError(f'byte length mismatch: {path}')
    if hashlib.sha256(data).hexdigest() != census_row['sha256']:
        raise RuntimeError(f"existing inventory SHA-256 mismatch: {path}")
    if git_blob_oid(data) != entry['git_blob_sha1']:
        raise RuntimeError(f"Git blob SHA-1 mismatch: {path}")


def install_verified(entry: dict, census_row: dict) -> None:
    rel = entry['path']
    target = REPO / rel
    source = STAGE / rel
    data = source.read_bytes()
    verify_bytes(rel, data, entry, census_row)
    mode = index_entry(rel, entry['git_blob_sha1'])
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        verify_bytes(rel, target.read_bytes(), entry, census_row)
        return
    # Temp file is created in the destination directory so replace is same-filesystem atomic.
    fd, temp_name = tempfile.mkstemp(prefix='.proof-restore-', dir=target.parent)
    try:
        with os.fdopen(fd, 'wb') as out:
            out.write(data)
            out.flush()
            os.fsync(out.fileno())
        os.chmod(temp_name, mode & 0o777)
        os.replace(temp_name, target)
        dirfd = os.open(target.parent, os.O_RDONLY)
        try:
            os.fsync(dirfd)
        finally:
            os.close(dirfd)
    except BaseException:
        try:
            os.unlink(temp_name)
        except FileNotFoundError:
            pass
        raise
    verify_bytes(rel, target.read_bytes(), entry, census_row)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--apply', action='store_true', help='download, verify, then restore files')
    args = parser.parse_args()
    entries, inv = load_plan()
    STAGE.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=4) as pool:
        states = list(pool.map(lambda e: fetch_and_verify(e, inv[e['path']], args.apply), entries))
    if args.apply:
        # No destination is touched until every member of the bounded set has verified.
        if any(s['state'] not in {'staged-and-verified', 'already-present-and-verified'} for s in states):
            raise RuntimeError('incomplete staging; refusing installation')
        for e in entries:
            install_verified(e, inv[e['path']])
        states = [dict(s, state='restored-and-verified' if s['state'] == 'staged-and-verified' else s['state']) for s in states]
    report = {
        'source_ref': SOURCE_REF,
        'cache_tree': '52f8f1dbfa6b81226ebf44f2137653721fceb513',
        'files': states,
        'count': len(states),
        'bytes': sum(e['compressed_bytes'] for e in entries),
        'verified_against': ['cached Git blob SHA-1', 'existing corpus-inventory SHA-256', 'existing inventory bytes'],
        'execution': 'No restored binary was executed.',
        'index': 'No Git index/worktree flags or Git objects were changed; existing skip-worktree flags are untouched.',
        'inventory': 'No corpus-inventory or evidence record was changed.',
        'mode_source': 'Each path mode is read from its existing Git index row and applied to the restored file.',
        'atomicity': 'Each target is installed via same-directory temp file plus os.replace; set is fully staged and verified before first target write.',
        'apply_requested': args.apply,
    }
    RESULT.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'apply': args.apply, 'files': len(states), 'bytes': report['bytes'], 'result': str(RESULT)}))

if __name__ == '__main__':
    main()
