#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Run the existing evidence gate, memoising unchanged large JSON bytes only."""

import hashlib
import importlib.util
import json
import sys
from collections import OrderedDict
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[6]
SPEC = importlib.util.spec_from_file_location(
    "proof_gate", ROOT / "scripts/dev/check-name-resolution-proofs.py"
)
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)
original_loads = json.loads
cache = OrderedDict()


def unchanged_json(value, *args, **kwargs):
    if args or kwargs or len(value) < 1_000_000:
        return original_loads(value, *args, **kwargs)
    data = value.encode() if isinstance(value, str) else value
    key = (len(data), hashlib.sha256(data).digest())
    if key not in cache:
        cache[key] = original_loads(value)
        if len(cache) > 2:
            cache.popitem(last=False)
    cache.move_to_end(key)
    return cache[key]


if __name__ == "__main__":
    json.loads = unchanged_json
    errors, summary = gate.validate(ROOT, True)
    print(json.dumps(summary, sort_keys=True))
    for error in errors:
        print(error)
    raise SystemExit(int(bool(errors)))
