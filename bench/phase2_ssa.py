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

"""Phase 2 benchmark — semi-pruned SSA phi reduction.

Counts total phi nodes placed across the real-world corpus so the reduction
from semi-pruned phi placement can be measured before/after.  Diagnostic
output-equivalence is validated separately with bench/phase0_descend.py
(--capture / --compare on the full analyser diagnostics).

    python bench/phase2_ssa.py --phis tmp/p2_phis_after.json
    # stash the ssa.py change ...
    python bench/phase2_ssa.py --phis tmp/p2_phis_before.json
    python bench/phase2_ssa.py --compare tmp/p2_phis_before.json tmp/p2_phis_after.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

CORPUS_ROOTS = [
    "tmp/tcllib-2.0/modules",
    "tmp/tcl9.0.3/library",
    "tmp/tklib-0.9/modules",
    "tmp/tdom-master",
]


def _corpus_files(limit: int) -> list[Path]:
    files: list[Path] = []
    for root in CORPUS_ROOTS:
        base = Path(root)
        if base.exists():
            files += [p for p in sorted(base.rglob("*.tcl")) if 0 < p.stat().st_size <= 400_000]
    files.sort()
    if limit and len(files) > limit:
        step = len(files) / limit
        files = [files[int(i * step)] for i in range(limit)]
    return files


def count_phis(limit: int, out_path: str) -> None:
    from compiler.compilation_unit import compile_source

    total_phis = funcs = 0
    per_file: dict[str, int] = {}
    for p in _corpus_files(limit):
        try:
            cu = compile_source(p.read_text(errors="replace"))
        except Exception:
            continue
        if cu is None:
            continue
        n = 0
        for fu in [cu.top_level, *cu.procedures.values()]:
            if fu.ssa is None:
                continue
            funcs += 1
            n += sum(len(b.phis) for b in fu.ssa.blocks.values())
        per_file[str(p)] = n
        total_phis += n
    Path(out_path).write_text(
        json.dumps({"total": total_phis, "funcs": funcs, "per_file": per_file})
    )
    print(f"total phis={total_phis} across {funcs} functions -> {out_path}")


def compare(a_path: str, b_path: str) -> int:
    a = json.loads(Path(a_path).read_text())
    b = json.loads(Path(b_path).read_text())
    before, after = a["total"], b["total"]
    reduction = 100 * (before - after) / before if before else 0
    print(f"phis before={before}  after={after}  reduction={before - after} ({reduction:.1f}%)")
    # sanity: no function should GAIN phis
    gained = [k for k in a["per_file"] if b["per_file"].get(k, 0) > a["per_file"][k]]
    if gained:
        print(f"WARNING: {len(gained)} files gained phis (unexpected): {gained[:5]}")
        return 1
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--phis", metavar="OUT")
    ap.add_argument("--compare", nargs=2, metavar=("A", "B"))
    ap.add_argument("--limit", type=int, default=300)
    args = ap.parse_args()
    if args.phis:
        count_phis(args.limit, args.phis)
        return 0
    if args.compare:
        return compare(args.compare[0], args.compare[1])
    ap.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
