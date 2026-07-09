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

"""Compiler-explorer debugging front-end — always from source, never a pyz.

A thin wrapper around the ``tcl-explorer`` CLI (``python -m tooling.explorer``)
plus two views the raw CLI does not offer, both aimed at the kind of off-by-one
range bug that hides inside the lexer → segmenter → IR pipeline:

* ``slices`` — every IR statement's source range *and the literal source slice
  it covers*.  A statement whose slice reads ``return {}}`` instead of
  ``return {}`` makes a one-byte range overshoot obvious at a glance (this is
  exactly how issue #527 was found).
* ``tokens`` — the raw lexer token stream with absolute offsets and per-token
  source slices, so a mis-placed ``end.offset`` shows up directly.

Every other verb (``greentree``, ``ir``, ``cfg``, ``ssa``, ``asm``, ``opt``,
``all``, …) is forwarded to ``python -m tooling.explorer --show <verb>`` with
``--text --no-colour`` so output is stable and greppable.  The wrapper runs the
explorer **from the working tree** via ``python -m`` — it deliberately does not
touch any built ``.pyz`` / zipapp, so what you inspect is the live source.

Usage (run from the repo root)::

    python .claude/skills/compiler-explorer/explore.py <verb> [--source S | FILE | -] [opts]

Examples::

    # The issue #527 reproducer — slice shows the overshoot instantly
    python .claude/skills/compiler-explorer/explore.py slices --source 'if {1} {return {}}'

    # Raw tokens with offsets + slices
    python .claude/skills/compiler-explorer/explore.py tokens --source 'set x {}'

    # Forwarded explorer views
    python .claude/skills/compiler-explorer/explore.py greentree --source 'proc f {} {}'
    python .claude/skills/compiler-explorer/explore.py ir --source 'set x 1; puts $x'
    python .claude/skills/compiler-explorer/explore.py asm --source 'return 42'
    echo 'set x 1' | python .claude/skills/compiler-explorer/explore.py ir -
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

# The repo root is three levels up from .claude/skills/compiler-explorer/.
_REPO_ROOT = Path(__file__).resolve().parents[3]

# A convenience verb that prints the three low-level views back to back;
# every other verb except the local "slices" / "tokens" is forwarded.
_OVERVIEW = "lowlevel"


def _read_source(path: str | None, source_arg: str | None) -> str:
    """Resolve the Tcl input from --source, a file path, or stdin (mirrors the CLI)."""
    if source_arg is not None:
        return source_arg
    if path is not None and path != "-":
        return Path(path).read_text()
    if path == "-" or not sys.stdin.isatty():
        return sys.stdin.read()
    raise SystemExit("No Tcl input provided. Use a file path, --source, or pipe stdin.")


def _line_starts(source: str) -> list[int]:
    starts = [0]
    for i, ch in enumerate(source):
        if ch == "\n":
            starts.append(i + 1)
    return starts


def _line_col(offset: int, starts: list[int]) -> tuple[int, int]:
    """1-based (line, column) for an absolute offset, for human-readable output."""
    lo, hi = 0, len(starts) - 1
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if starts[mid] <= offset:
            lo = mid
        else:
            hi = mid - 1
    return lo + 1, offset - starts[lo] + 1


def _slice_repr(source: str, start: int, end: int) -> str:
    """``repr`` of the inclusive source span [start, end].

    A token/statement whose end sits exactly at ``len(source)`` is the virtual
    end-of-input marker (``EOL`` at EOF), not a bug, so it renders as ``<eof>``.
    Anything reaching genuinely past the end is flagged loudly — that *is* the
    kind of range overshoot this view exists to surface.
    """
    n = len(source)
    if start == n and end == n:
        return "<eof>"
    if start < 0 or end < start or end >= n:
        return f"<OUT-OF-RANGE start={start} end={end} len={n}>"
    return repr(source[start : end + 1])


def _view_slices(source: str, dialect: str) -> int:
    """Print every IR statement's range and the source slice it covers.

    A correct range's slice is exactly the command text; a one-byte overshoot
    (e.g. a trailing empty ``{}`` argument swallowing the body's ``}``) shows up
    as an extra closing delimiter in the slice.
    """
    from analyser.compiler_checks import iter_ir_statements
    from compiler.lowering import lower_to_ir
    from compiler.registry.dialect import dialect_scope

    starts = _line_starts(source)
    with dialect_scope(dialect):
        ir = lower_to_ir(source)

    def emit(label: str, statements) -> None:
        rows = []
        for stmt in statements:
            r = stmt.range
            s, e = r.start.offset, r.end.offset
            sl, sc = _line_col(s, starts)
            el, ec = _line_col(e, starts)
            kind = type(stmt).__name__
            rows.append(
                (
                    f"{kind:<14}",
                    f"off {s}-{e}",
                    f"[{sl}:{sc}-{el}:{ec}]",
                    _slice_repr(source, s, e),
                )
            )
        if not rows:
            print(f"  {label}: (none)")
            return
        print(f"  {label}:")
        for kind, off, lc, sl in rows:
            print(f"    {kind} {off:<12} {lc:<14} slice={sl}")

    print("ir-statement-slices")
    emit("top-level", iter_ir_statements(ir.top_level))
    for name, proc in ir.procedures.items():
        emit(f"proc {name}", iter_ir_statements(proc.body))
    return 0


def _view_tokens(source: str, dialect: str) -> int:
    """Dump the raw lexer token stream with absolute offsets and source slices."""
    from compiler.parsing.lexer import TclLexer
    from compiler.registry.dialect import dialect_scope

    print("lexer-tokens")
    with dialect_scope(dialect):
        tokens = TclLexer(source).tokenise_all()
    for tok in tokens:
        s, e = tok.start.offset, tok.end.offset
        print(
            f"  {tok.type.name:<5} text={tok.text!r:<24} "
            f"off {s}-{e:<4} slice={_slice_repr(source, s, e)}"
        )
    return 0


def _forward_to_explorer(view: str, source: str, dialect: str, opt: str) -> int:
    """Run ``python -m tooling.explorer`` from the working tree for a rich view."""
    cmd = [
        sys.executable,
        "-m",
        "tooling.explorer",
        "--source",
        source,
        "--show",
        view,
        "--dialect",
        dialect,
        "--opt",
        opt,
        "--text",
        "--no-colour",
    ]
    return subprocess.run(cmd, cwd=_REPO_ROOT, check=False).returncode


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Debug the Tcl compiler from source (no pyz).  Local views: "
            "slices, tokens, lowlevel.  All other verbs forward to tcl-explorer "
            "(greentree, ir, cfg, ssa, asm, opt, all, ...)."
        ),
    )
    parser.add_argument(
        "view",
        help="View to render: slices | tokens | lowlevel | any tcl-explorer --show name.",
    )
    parser.add_argument(
        "path", nargs="?", help="Tcl file to inspect, or '-' for stdin."
    )
    parser.add_argument("--source", help="Inline Tcl source to inspect.")
    parser.add_argument(
        "--dialect", default="tcl8.6", help="Dialect profile (default: tcl8.6)."
    )
    parser.add_argument(
        "--opt",
        choices=("off", "on", "diff"),
        default="off",
        help="Optimisation lens for forwarded views (ignored by local views).",
    )
    args = parser.parse_args(argv)

    source = _read_source(args.path, args.source)

    if args.view == "slices":
        return _view_slices(source, args.dialect)
    if args.view == "tokens":
        return _view_tokens(source, args.dialect)
    if args.view == _OVERVIEW:
        rc = _view_tokens(source, args.dialect)
        print()
        _forward_to_explorer("greentree", source, args.dialect, "off")
        print()
        return rc or _view_slices(source, args.dialect)

    return _forward_to_explorer(args.view, source, args.dialect, args.opt)


if __name__ == "__main__":
    raise SystemExit(main())
