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

"""One-time cached diagnostics dump for fast precision audits.

Runs get_diagnostics over the corpus ONCE and writes every diagnostic with
source context to tmp/diag_dump.jsonl (one JSON object per diagnostic):
  {file, code, sev, line, char, eline, echar, msg, line_text, window}
`window` is the diagnostic line plus the next 40 lines (for loop-body / scope
heuristics).  Audits then read this file instantly instead of re-analysing.

  python bench/diag_dump.py [--limit N]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics  # noqa: E402

ROOTS = [
    "tmp/tcllib-2.0/modules",
    "tmp/tcl9.0.3/library",
    "tmp/tklib-0.9/modules",
    "tmp/tdom-master",
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=None)
    ap.add_argument("--out", default="tmp/diag_dump.jsonl")
    args = ap.parse_args()

    files: list[Path] = []
    for r in ROOTS:
        b = Path(r)
        if b.exists():
            files += [p for p in sorted(b.rglob("*.tcl")) if 0 < p.stat().st_size <= 600_000]
    files.sort()
    if args.limit:
        files = files[: args.limit]

    n = 0
    with open(args.out, "w") as fh:
        for i, p in enumerate(files, 1):
            try:
                src = p.read_text(errors="replace")
                lines = src.split("\n")
                # Pass the file path as the URI so URI-gated diagnostics
                # (e.g. the pkgIndex.tcl ``dir`` exemption) match real LSP usage.
                diags = get_diagnostics(src, uri=str(p))
            except Exception:
                continue
            for d in diags:
                ln = d.range.start.line
                rec = {
                    "file": str(p),
                    "code": d.code,
                    "sev": str(getattr(d, "severity", "")),
                    "line": ln,
                    "char": d.range.start.character,
                    "eline": d.range.end.line,
                    "echar": d.range.end.character,
                    "msg": d.message,
                    "line_text": lines[ln] if 0 <= ln < len(lines) else "",
                    "window": "\n".join(lines[ln : ln + 40]),
                    "nfixes": len(getattr(d, "fixes", None) or ()),
                }
                fh.write(json.dumps(rec) + "\n")
                n += 1
            if i % 150 == 0:
                fh.flush()
                print(f"... {i}/{len(files)} files, {n} diagnostics", file=sys.stderr)
    print(f"wrote {n} diagnostics from {len(files)} files -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
