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

"""Phase 1 benchmark — shared loop-nesting forest.

Two jobs on the real-world corpus (tcllib/stdlib/tklib/tdom under tmp/):

1. **Oracle cross-check** — for every compiled function, assert the
   ``LoopForest`` (compiler/loops.py) finds exactly the same loop headers and
   per-header body blocks as an independent brute-force back-edge scan over the
   SSA dominator tree.  Reports loop statistics across the corpus.

       python bench/phase1_loops.py --oracle

2. **O106 byte-identical** — capture/compare the loop-invariant (O106) GVN
   diagnostics so the loop-forest extraction is proven output-preserving.

       python bench/phase1_loops.py --capture tmp/p1_before.json
       # ...switch to the new code...
       python bench/phase1_loops.py --capture tmp/p1_after.json
       python bench/phase1_loops.py --compare tmp/p1_before.json tmp/p1_after.json
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
        if not base.exists():
            continue
        for p in sorted(base.rglob("*.tcl")):
            try:
                if 0 < p.stat().st_size <= 400_000:
                    files.append(p)
            except OSError:
                continue
    files.sort()
    if limit and len(files) > limit:
        step = len(files) / limit
        files = [files[int(i * step)] for i in range(limit)]
    return files


def _brute_force_headers(cfg, ssa, executable):
    """Independent oracle: back edge = succ dominates tail (no LoopForest)."""
    from compiler.loops import cfg_successors, dominates

    headers: dict[str, set[str]] = {}
    for tail in executable:
        for succ in cfg_successors(cfg, tail):
            if succ not in executable:
                continue
            if dominates(ssa, succ, tail):
                headers.setdefault(succ, set()).add(tail)
    return headers


def oracle(limit: int) -> int:
    from compiler.compilation_unit import compile_source
    from compiler.loops import build_loop_forest

    files = _corpus_files(limit)
    funcs = loops = nested = mismatches = 0
    for p in files:
        try:
            cu = compile_source(p.read_text(errors="replace"))
        except Exception:
            continue
        if cu is None:
            continue
        for fu in [cu.top_level, *cu.procedures.values()]:
            if fu.ssa is None or fu.cfg is None or fu.analysis is None:
                continue
            funcs += 1
            executable = set(fu.cfg.blocks) - fu.analysis.unreachable_blocks
            forest = build_loop_forest(fu.cfg, fu.ssa, executable)
            oracle_headers = _brute_force_headers(fu.cfg, fu.ssa, executable)
            # Same set of headers?
            if set(forest.by_header) != set(oracle_headers):
                mismatches += 1
                continue
            loops += len(forest.by_header)
            # nesting: a header dominated by another header
            from compiler.loops import dominates

            hs = list(forest.by_header)
            for h in hs:
                if any(h2 != h and dominates(fu.ssa, h2, h) for h2 in hs):
                    nested += 1
                    break
    print(f"functions={funcs}  loops={loops}  funcs_with_nested_header={nested}")
    if mismatches:
        print(f"ORACLE MISMATCH in {mismatches} functions")
        return 1
    print("ORACLE OK: LoopForest headers == brute-force back-edge oracle on all functions")
    return 0


def capture(out_path: str, limit: int) -> None:
    from compiler.gvn import find_redundant_computations

    result: dict[str, list] = {}
    for p in _corpus_files(limit):
        try:
            src = p.read_text(errors="replace")
            o106 = [
                [
                    r.code,
                    r.expression_text,
                    r.message,
                    getattr(r.range.start, "line", None),
                    getattr(r.range.start, "character", None),
                ]
                for r in find_redundant_computations(src)
                if getattr(r, "code", None) == "O106"
            ]
        except Exception as exc:  # noqa: BLE001
            result[str(p)] = [["__EXC__", repr(exc)]]
            continue
        result[str(p)] = sorted(o106)
    Path(out_path).write_text(json.dumps(result, sort_keys=True))
    total = sum(len(v) for v in result.values())
    print(f"captured {total} O106 hints across {len(result)} files -> {out_path}")


def compare(a_path: str, b_path: str) -> int:
    a = json.loads(Path(a_path).read_text())
    b = json.loads(Path(b_path).read_text())
    keys = sorted(set(a) | set(b))
    mismatches = sum(1 for k in keys if a.get(k) != b.get(k))
    if mismatches == 0:
        print(f"BYTE-IDENTICAL: O106 hints match across {len(keys)} files")
        return 0
    for k in keys:
        if a.get(k) != b.get(k):
            print(f"DIFF {k}\n  before {a.get(k)}\n  after  {b.get(k)}")
    print(f"MISMATCH: {mismatches}/{len(keys)} files differ")
    return 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--oracle", action="store_true")
    ap.add_argument("--capture", metavar="OUT")
    ap.add_argument("--compare", nargs=2, metavar=("A", "B"))
    ap.add_argument("--limit", type=int, default=300)
    args = ap.parse_args()
    if args.oracle:
        return oracle(args.limit)
    if args.capture:
        capture(args.capture, args.limit)
        return 0
    if args.compare:
        return compare(args.compare[0], args.compare[1])
    ap.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
