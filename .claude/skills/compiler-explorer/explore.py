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

"""Compiler-explorer debugging front-end — always from source, never a built binary.

A thin wrapper around ``tcl explore`` (the Rust compiler-explorer surface in
``rust/tcl-cli``) plus three views the CLI's text renderer does not offer, all
aimed at the kind of off-by-one range bug that hides inside the parser → IR
pipeline:

* ``slices`` — every IR statement's source range *and the literal source slice
  it covers*.  A statement whose slice reads ``return {}}`` instead of
  ``return {}`` makes a one-byte range overshoot obvious at a glance (this is
  exactly how issue #527 was found).
* ``tokens`` — the CST's terminal nodes with absolute offsets and per-node
  source slices, so a mis-placed ``endOffset`` shows up directly.
* ``cst`` — the parse tree as an indented tree with offsets.

The parser builds a single red-green CST directly, so there is no separate lexer
token stream and no standalone "green tree" to dump: ``tokens`` reports the CST's
*leaf* nodes, which are the terminal spans the compiler actually consumes.
``greentree`` is accepted as an alias for ``cst``.

``cst``/``tokens`` are rendered here rather than forwarded because ``tcl explore
--text`` has no renderer for the ``cst`` and ``segments`` views — they exist only
in the ``--json`` contract.  Every other verb (``ir``, ``cfg``, ``ssa``, ``asm``,
``opt``, ``taint``, …) is forwarded to ``tcl explore --show <verb>`` with
``--text --no-colour`` so output is stable and greppable.

Offsets in the explorer's JSON are **half-open**: ``[startOffset, endOffset)``,
so a slice is ``source[start:end]``.

By default the wrapper invokes ``cargo run -p tcl-cli --bin tcl`` so what you
inspect is the live working tree, never a stale build.  Set ``TCL_EXPLORE_BIN``
to a prebuilt ``tcl`` binary to skip the rebuild when you know it is current.

Usage (run from the repo root)::

    python .claude/skills/compiler-explorer/explore.py <verb> [--source S | FILE | -] [opts]

Examples::

    # The issue #527 reproducer — slice shows the overshoot instantly
    python .claude/skills/compiler-explorer/explore.py slices --source 'if {1} {return {}}'

    # CST leaf spans with offsets + slices
    python .claude/skills/compiler-explorer/explore.py tokens --source 'set x {}'

    # Forwarded explorer views
    python .claude/skills/compiler-explorer/explore.py cst --source 'proc f {} {}'
    python .claude/skills/compiler-explorer/explore.py ir --source 'set x 1; puts $x'
    python .claude/skills/compiler-explorer/explore.py asm --source 'return 42'
    echo 'set x 1' | python .claude/skills/compiler-explorer/explore.py ir -
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import Any

# The repo root is three levels up from .claude/skills/compiler-explorer/.
_REPO_ROOT = Path(__file__).resolve().parents[3]

# A convenience verb that prints the three low-level views back to back;
# every other verb except the local views (slices / tokens / cst) is forwarded.
# Those three are rendered here because `tcl explore --text` has no renderer for
# `cst` — it exists only in the `--json` contract.
_OVERVIEW = "lowlevel"


def _tcl_command() -> list[str]:
    """The argv prefix that runs the `tcl` CLI from the working tree.

    Defaults to `cargo run` so the explorer always reflects live source.
    `TCL_EXPLORE_BIN` overrides it with a prebuilt binary when you know the tree
    is already built.
    """
    override = os.environ.get("TCL_EXPLORE_BIN")
    if override:
        return [override]
    return ["cargo", "run", "-q", "-p", "tcl-cli", "--bin", "tcl", "--"]


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
    """``repr`` of the half-open source span ``[start, end)``.

    A node whose span is empty at end-of-input is the virtual end-of-input
    marker, not a bug, so it renders as ``<eof>``.  Anything reaching genuinely
    past the end is flagged loudly — that *is* the kind of range overshoot this
    view exists to surface.
    """
    n = len(source)
    if start == n and end == n:
        return "<eof>"
    if start < 0 or end < start or end > n:
        return f"<OUT-OF-RANGE start={start} end={end} len={n}>"
    return repr(source[start:end])


def _explorer_json(source: str, dialect: str) -> dict[str, Any]:
    """Run `tcl explore --json` over `source` and return the parsed contract."""
    cmd = [
        *_tcl_command(),
        "explore",
        "--source",
        source,
        "--dialect",
        dialect,
        "--json",
    ]
    proc = subprocess.run(
        cmd, cwd=_REPO_ROOT, capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit(f"tcl explore --json failed (exit {proc.returncode})")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:  # pragma: no cover — CLI contract break
        sys.stderr.write(proc.stdout[:2000])
        raise SystemExit(f"tcl explore --json emitted non-JSON: {exc}") from exc


def _iter_ir_statements(nodes: list[dict[str, Any]]) -> Iterator[dict[str, Any]]:
    """Yield every IR statement in `nodes`, depth-first, parents before children.

    Statement nodes carry a `kind`; the intermediate clause nodes of `IRIf` /
    `IRSwitch` carry only a `label` and a `body`, so they are recursed into but
    not reported as statements.
    """
    for node in nodes:
        if "kind" in node and "range" in node:
            yield node
        for key in ("children", "body"):
            child = node.get(key)
            if isinstance(child, list):
                yield from _iter_ir_statements(child)


def _view_slices(source: str, dialect: str) -> int:
    """Print every IR statement's range and the source slice it covers.

    A correct range's slice is exactly the command text; a one-byte overshoot
    (e.g. a trailing empty ``{}`` argument swallowing the body's ``}``) shows up
    as an extra closing delimiter in the slice.
    """
    contract = _explorer_json(source, dialect)
    ir = contract["ir"]
    starts = _line_starts(source)

    def emit(label: str, statements: Iterator[dict[str, Any]]) -> None:
        rows = []
        for stmt in statements:
            r = stmt["range"]
            s, e = r["startOffset"], r["endOffset"]
            sl, sc = _line_col(s, starts)
            el, ec = _line_col(min(e, len(source)), starts)
            rows.append(
                (
                    f"{stmt['kind']:<14}",
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
    emit("top-level", _iter_ir_statements(ir["topLevel"]))
    for name, proc in ir["procedures"].items():
        emit(f"proc {name}", _iter_ir_statements(proc["body"]))
    return 0


def _iter_cst_leaves(node: dict[str, Any]) -> Iterator[dict[str, Any]]:
    """Yield the CST's terminal nodes, left to right."""
    children = node.get("children") or []
    if not children:
        yield node
        return
    for child in children:
        yield from _iter_cst_leaves(child)


def _view_tokens(source: str, dialect: str) -> int:
    """Dump the CST's leaf spans with absolute offsets and source slices.

    The Rust parser has no separate lexer token stream — it builds a red-green
    CST directly — so the leaves are the terminal spans the compiler consumes.
    """
    contract = _explorer_json(source, dialect)
    print("cst-leaf-spans")
    for node in _iter_cst_leaves(contract["cst"]):
        s, e = node["startOffset"], node["endOffset"]
        label = node.get("label") or ""
        text = f"{node['kind']}{f' {label}' if label else ''}"
        print(f"  {text:<30} off {s}-{e:<4} slice={_slice_repr(source, s, e)}")
    return 0


def _view_cst(source: str, dialect: str) -> int:
    """Render the parse tree as an indented tree with offsets."""
    contract = _explorer_json(source, dialect)
    print("cst")

    def walk(node: dict[str, Any], depth: int) -> None:
        s, e = node["startOffset"], node["endOffset"]
        label = node.get("label")
        tags = " ".join(node.get("tags") or [])
        bits = [f"{'  ' * (depth + 1)}{node['kind']}"]
        if label:
            bits.append(repr(label))
        bits.append(f"[{s}-{e}]")
        if tags:
            bits.append(f"({tags})")
        print(" ".join(bits))
        for child in node.get("children") or []:
            walk(child, depth + 1)

    walk(contract["cst"], 0)
    return 0


def _forward_to_explorer(view: str, source: str, dialect: str) -> int:
    """Run `tcl explore --show <view>` from the working tree for a rich view."""
    cmd = [
        *_tcl_command(),
        "explore",
        "--source",
        source,
        "--show",
        view,
        "--dialect",
        dialect,
        "--text",
        "--no-colour",
    ]
    return subprocess.run(cmd, cwd=_REPO_ROOT, check=False).returncode


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Debug the Tcl compiler from source.  Local views: "
            "slices, tokens, cst, lowlevel.  All other verbs forward to "
            "`tcl explore --show` (ir, cfg, ssa, asm, opt, taint, ...)."
        ),
    )
    parser.add_argument(
        "view",
        help="View to render: slices | tokens | cst | lowlevel | any `tcl explore --show` name.",
    )
    parser.add_argument(
        "path", nargs="?", help="Tcl file to inspect, or '-' for stdin."
    )
    parser.add_argument("--source", help="Inline Tcl source to inspect.")
    parser.add_argument(
        "--dialect", default="tcl8.6", help="Dialect profile (default: tcl8.6)."
    )
    args = parser.parse_args(argv)

    source = _read_source(args.path, args.source)
    view = args.view

    if view == "greentree":
        # One red-green CST; there is no separate green tree to dump.
        print("note: `greentree` is an alias for `cst`", file=sys.stderr)
        view = "cst"

    if view == "slices":
        return _view_slices(source, args.dialect)
    if view == "tokens":
        return _view_tokens(source, args.dialect)
    if view == "cst":
        return _view_cst(source, args.dialect)
    if view == _OVERVIEW:
        rc = _view_tokens(source, args.dialect)
        print()
        _view_cst(source, args.dialect)
        print()
        return rc or _view_slices(source, args.dialect)

    return _forward_to_explorer(view, source, args.dialect)


if __name__ == "__main__":
    raise SystemExit(main())
