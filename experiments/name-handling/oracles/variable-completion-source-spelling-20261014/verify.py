# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later

from pathlib import Path
import argparse
import hashlib
import json

parser = argparse.ArgumentParser()
parser.add_argument('--current-providers', action='store_true')
args = parser.parse_args()
root = Path(__file__).resolve().parent
index = json.loads((root / 'archive-index.json').read_text())


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


for row in index['artifacts']:
    path = root / row['path']
    assert path.is_file() and sha(path) == row['sha256'], row['path']
receipt = json.loads((root / 'manifest.json').read_text())
assert sha(root / 'manifest.json') == index['original_receipt_sha256']
assert sha(root / 'probe.tcl') == receipt['source_sha256']
mapping = index['original_path_map']
assert mapping[receipt['source']] == 'probe.tcl'
for row in receipt['runs']:
    assert row['exit_code'] == 0, row['provider']
    assert row['binary_sha256_before'] == row['binary_sha256_after'], row['provider']
    for channel in ['stdout', 'stderr']:
        path = root / mapping[row[channel]]
        assert sha(path) == row[channel + '_sha256'], row['provider']
    if args.current_providers:
        assert sha(Path(row['executable'])) == row['binary_sha256_before'], row['provider']
provenance = json.loads((root / 'provider-provenance.json').read_text())
assert all(row['matches_existing_extent_provider'] for row in provenance['providers'])
prior = root / provenance['existing_extent_receipt']
if prior.is_file():
    assert sha(prior) == provenance['existing_extent_receipt_sha256']
print('Verified original source, receipt, six provider hash pairs, and 12 output streams.')
