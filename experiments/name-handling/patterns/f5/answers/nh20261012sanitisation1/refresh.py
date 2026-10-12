#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Refresh only retained path-resolved artifact descriptors after projection."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
BASE = Path(__file__).resolve().parents[1]
NAMES = ['repair2', 'harness1', 'harness2', 'collision1', 'nativecap1', 'aplrepair1', 'native1', 'native2', 'unreached1', 'unreached2', 'finish1', 'cleanup3', 'validation2', 'sanitisation1']


def main():
    paths = sorted(p for name in NAMES for p in (BASE / ('nh20261012' + name)).rglob('*.json'))
    paths.append(BASE.parent / 'questions.json')
    changed = set()
    for iteration in range(15):
        cache = {}
        def digest(path):
            if path not in cache:
                cache[path] = hashlib.sha256(path.read_bytes()).hexdigest()
            return cache[path]
        def visit(value):
            if isinstance(value, list):
                return [visit(v) for v in value]
            if not isinstance(value, dict):
                return value
            result = {k: visit(v) for k, v in value.items()}
            if isinstance(result.get('path'), str) and isinstance(result.get('sha256'), str):
                path = ROOT / result['path']
                if path.is_file():
                    result['sha256'] = digest(path)
            return result
        count = 0
        for path in paths:
            if not path.exists():
                continue
            original = json.loads(path.read_text())
            value = visit(original)
            if value != original:
                # Preserve compact large measurements.
                path.write_text(json.dumps(value, separators=(',', ':')) + '\n' if path.stat().st_size > 20000000 else json.dumps(value, indent=2) + '\n')
                cache.pop(path, None)
                count += 1; changed.add(str(path.relative_to(ROOT)))
        print('descriptor iteration', iteration, 'changed files', count)
        if not count:
            break
    else:
        raise RuntimeError('Descriptor graph did not converge')
    print('converged; changed', len(changed), 'files')


if __name__ == '__main__':
    main()
