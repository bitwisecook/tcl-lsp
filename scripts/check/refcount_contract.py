#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Refcount contract lint — S0.1 deliverable.

Walks every ``pub export fn`` in ``runtime/zig/`` and warns when an
export is missing a row in the refcount contract doc.

Exit codes:

- 0: every export has a contract row.
- 1: some exports are missing rows (warning by default; escalates to
  error when ``--strict`` is passed).

Usage:

    uv run python scripts/check/refcount_contract.py
    uv run python scripts/check/refcount_contract.py --strict
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RUNTIME = REPO / "runtime" / "zig"
CONTRACT = REPO / "docs" / "design" / "runtime" / "refcount-contract.md"

# Matches ``pub export fn name(`` — captures the function name.
_FN_RE = re.compile(r"\bpub\s+export\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")

# Matches a table row that starts with ``| `funcname(...)`` or
# ``| ``funcname(...) ``.  We only care about the leading function
# name; whatever follows is free-form documentation.
_ROW_RE = re.compile(r"^\|\s*[`]+([A-Za-z_][A-Za-z0-9_]*)")


def collect_exports() -> dict[str, list[Path]]:
    """Walk runtime/zig and return ``{export_name: [files containing it]}``."""
    out: dict[str, list[Path]] = {}
    for zig in RUNTIME.rglob("*.zig"):
        # Skip the vendored regex C-bridge headers that happen to live
        # under the zig tree.
        if "vendor/" in str(zig).replace("\\", "/"):
            continue
        text = zig.read_text(encoding="utf-8")
        for m in _FN_RE.finditer(text):
            out.setdefault(m.group(1), []).append(zig.relative_to(REPO))
    return out


def collect_rows() -> set[str]:
    """Parse the contract doc and return every function name with a row."""
    if not CONTRACT.exists():
        return set()
    out: set[str] = set()
    for line in CONTRACT.read_text(encoding="utf-8").splitlines():
        m = _ROW_RE.match(line)
        if m:
            out.add(m.group(1))
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--strict",
        action="store_true",
        help="exit 1 on any missing row (default: warning only)",
    )
    args = parser.parse_args()

    exports = collect_exports()
    rows = collect_rows()

    missing = sorted(set(exports) - rows)
    extra = sorted(rows - set(exports))

    if missing:
        print(
            f"WARNING: {len(missing)} runtime exports missing from {CONTRACT.relative_to(REPO)}:",
            file=sys.stderr,
        )
        for name in missing:
            files = ", ".join(str(f) for f in exports[name])
            print(f"  - {name}  ({files})", file=sys.stderr)

    if extra:
        print(
            f"WARNING: {len(extra)} contract rows reference functions "
            "not found in runtime/zig/ — stale rows or typos:",
            file=sys.stderr,
        )
        for name in extra:
            print(f"  - {name}", file=sys.stderr)

    if (missing or extra) and args.strict:
        return 1
    if missing or extra:
        print(
            f"\n{len(rows)} of {len(exports)} runtime exports have "
            f"contract rows ({100 * len(rows & set(exports)) // max(1, len(exports))}%).",
            file=sys.stderr,
        )
        return 0
    print(
        f"OK: every runtime export has a contract row ({len(exports)} exports / {len(rows)} rows).",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
