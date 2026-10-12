#!/usr/bin/env python3
"""Replay to a new directory after whole original input verification; changes no old capture."""
from pathlib import Path
import argparse
import hashlib
import json

parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, required=True, help='New absolute capture directory')
args = parser.parse_args()
here = Path(__file__).resolve().parent
out = args.output
assert out.is_absolute() and not out.exists(), 'A new absolute output is mandatory'
original = Path('/workspace/.proofs/root-native-encoding-source643')
assert not out.is_relative_to(original) and not original.is_relative_to(out)
assert not out.is_relative_to(here) and not here.is_relative_to(out)
def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for data in iter(lambda: stream.read(1024 * 1024), b''): h.update(data)
    return h.hexdigest()
mapping = json.loads((here / 'pinned-input-map.json').read_text())
for name, expected in mapping['original_path_to_sha256'].items():
    assert Path(name).is_file() and sha(Path(name)) == expected, ('Restore exact original input separately', name)
receipt = json.loads((here / 'REPLAY-PATH-ADAPTATION.json').read_text())
scope = {'__name__': '__main__', '__replay_request': str(here), '__replay_output': str(out), '__replay_version_source': str(here / 'library-version.c')}
for row in receipt['scripts']:
    path = here / row['path']
    assert sha(path) == row['original_sha256']
    text = path.read_text()
    for change in row['literal_replacements']:
        assert text.count(change['old']) == 1
        text = text.replace(change['old'], change['new'], 1)
    exec(compile(text, str(path), 'exec'), scope)
print(json.dumps({'new_capture': str(out), 'scope': 'New experiment; original receipts remain unchanged. Rebuilt bytes are not attributed to old Native643 outputs.'}))
