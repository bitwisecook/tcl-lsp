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

"""Phase 0 benchmark — registry-aware green-tree descent.

Two jobs:

1. **Byte-identical diagnostics over a real-world corpus** (the hard bar for the
   Phase 0 migration of ``analyser/compiler_checks.py`` body recursion):
       python bench/phase0_descend.py --capture tmp/p0_before.json
       # ...make the code change...
       python bench/phase0_descend.py --capture tmp/p0_after.json
       python bench/phase0_descend.py --compare tmp/p0_before.json tmp/p0_after.json

2. **EXPR-divergence demonstration** — why EXPR args are NOT routed through the
   (script-lexing) green tree but stay on the expr lexer:
       python bench/phase0_descend.py --expr-divergence

Corpus: tcllib 2.0 / Tcl 9.0 stdlib / tklib 0.9 / tdom under tmp/ (same as the
algo_experiments harness).
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
        # deterministic spread across the (sorted) corpus
        step = len(files) / limit
        files = [files[int(i * step)] for i in range(limit)]
    return files


def _diag_key(d) -> list:
    r = getattr(d, "range", None)
    start = getattr(r, "start", None)
    end = getattr(r, "end", None)
    return [
        getattr(d, "code", None),
        getattr(d, "severity", None).__repr__(),
        getattr(start, "line", None),
        getattr(start, "character", None),
        getattr(end, "line", None),
        getattr(end, "character", None),
        getattr(d, "message", None),
    ]


def capture(out_path: str, limit: int) -> None:
    from analyser import analyse

    result: dict[str, list] = {}
    files = _corpus_files(limit)
    errors = 0
    for p in files:
        try:
            src = p.read_text(errors="replace")
            diags = analyse(src).diagnostics
        except Exception as exc:  # noqa: BLE001 - record, keep going
            result[str(p)] = [["__EXC__", repr(exc)]]
            errors += 1
            continue
        result[str(p)] = sorted(_diag_key(d) for d in diags)
    Path(out_path).write_text(json.dumps(result, sort_keys=True))
    total = sum(len(v) for v in result.values())
    print(
        f"captured {len(files)} files, {total} diagnostics -> {out_path} ({errors} analyse errors)"
    )


def compare(a_path: str, b_path: str) -> int:
    a = json.loads(Path(a_path).read_text())
    b = json.loads(Path(b_path).read_text())
    keys = sorted(set(a) | set(b))
    mismatches = 0
    for k in keys:
        if a.get(k) != b.get(k):
            mismatches += 1
            if mismatches <= 20:
                print(f"DIFF {k}")
                print(f"  before: {a.get(k)}")
                print(f"  after : {b.get(k)}")
    if mismatches == 0:
        print(f"BYTE-IDENTICAL: {len(keys)} files, diagnostics match exactly")
        return 0
    print(f"MISMATCH: {mismatches}/{len(keys)} files differ")
    return 1


def expr_divergence() -> None:
    """Show that script-lexing an expr (green tree) finds a DIFFERENT set of
    embedded command substitutions than the expr lexer — hence EXPR cannot be
    routed through the green tree byte-identically.
    """
    from compiler.parsing.expr_lexer import ExprTokenType, tokenise_expr
    from compiler.parsing.green_tree import Mode, node_for

    cases = [
        '"\\[" eq $x',  # literal [ inside an expr string
        "$a + [foo bar]",  # genuine command substitution
        '"[" ne "]" && [g]',  # bracket chars as data + a real command
        "[expr {1}] > 0",  # nested command
    ]
    print("expr text                | script-lex CMDs | expr-lex COMMANDs | diverge?")
    print("-" * 78)
    for expr in cases:
        node = node_for(expr, mode=Mode.SCRIPT)
        script_cmds = [t.text for t in node.tokens if t.type.name == "CMD"]
        expr_cmds = [t.text for t in tokenise_expr(expr) if t.type is ExprTokenType.COMMAND]
        diverge = script_cmds != expr_cmds
        print(f"{expr:24s} | {str(script_cmds):15s} | {str(expr_cmds):17s} | {diverge}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--capture", metavar="OUT")
    ap.add_argument("--compare", nargs=2, metavar=("A", "B"))
    ap.add_argument("--expr-divergence", action="store_true")
    ap.add_argument("--limit", type=int, default=300)
    args = ap.parse_args()
    if args.capture:
        capture(args.capture, args.limit)
        return 0
    if args.compare:
        return compare(args.compare[0], args.compare[1])
    if args.expr_divergence:
        expr_divergence()
        return 0
    ap.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
