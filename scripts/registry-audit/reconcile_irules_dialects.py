#!/usr/bin/env python3
"""Reconcile the Rust command registry's per-command ``dialects`` fields
with the Python registry (the source of truth).

The Rust port of the command registry (``rust/tcl-registry/src/commands``)
encoded Tcl-*version* availability and Tk membership directly in the
``DialectSet`` (``Some(ALL_TCL)``, ``Some(TK)``, …) where the Python
registry uses ``dialects=None`` (universal) or an explicit per-command
frozenset. As a result a large swathe of core-Tcl / tcllib / Tk commands
that Python treats as valid in every dialect (including ``f5-irules``)
failed Rust's ``supports_dialect(IRULES)`` check, and the math-operator
ensemble — which Python marks valid in every dialect *except* iRules —
was wrongly universal. That broke the iRules event/command cross-product
(`commands_for_event`) used by ``f5 irule event-info`` and
``registry-dump``.

This script rewrites each Rust ``CommandSpec``'s spec-level ``dialects:``
field to mirror the Python membership over the dialects the Rust
``DialectSet`` models (the four Tcl versions plus iRules / iApps / Tk /
Expect / the five EDA dialects). It is idempotent and safe to re-run
whenever the Python baseline moves.

Run from the repo root with the project venv active:

    python scripts/registry-audit/reconcile_irules_dialects.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Dialects the Rust DialectSet models, in the canonical bit order, mapped
# to their Rust constant names.
DIALECT_BITS: list[tuple[str, str]] = [
    ("tcl8.4", "TCL84"),
    ("tcl8.5", "TCL85"),
    ("tcl8.6", "TCL86"),
    ("tcl9.0", "TCL90"),
    ("f5-irules", "IRULES"),
    ("f5-iapps", "IAPPS"),
    ("tk", "TK"),
    ("expect", "EXPECT"),
    ("synopsys-eda-tcl", "SYNOPSYS"),
    ("cadence-eda-tcl", "CADENCE"),
    ("xilinx-eda-tcl", "XILINX"),
    ("intel-quartus-eda-tcl", "QUARTUS"),
    ("mentor-eda-tcl", "MENTOR"),
]
ALL_DIALECTS = frozenset(name for name, _ in DIALECT_BITS)
BIT_OF = dict(DIALECT_BITS)

# Named DialectSet aggregates, longest-match-first so we collapse a
# membership set to the most readable constant the Rust source already
# uses.
NAMED_SETS: list[tuple[str, frozenset[str]]] = [
    # Longest-match first so a membership set collapses to the most
    # readable aggregate constant.
    ("NON_IRULES_OPERATORS", ALL_DIALECTS - {"f5-irules", "tk"}),
    ("ALL_TCL", frozenset({"tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"})),
    ("TCL85_PLUS", frozenset({"tcl8.5", "tcl8.6", "tcl9.0"})),
    ("TCL86_PLUS", frozenset({"tcl8.6", "tcl9.0"})),
]


def py_membership() -> dict[str, frozenset[str]]:
    """Return ``name -> membership`` over the modelled dialects, from the
    Python registry (with every dialect loaded, matching the f5 CLI)."""
    import tooling.f5.main  # noqa: F401  (loads the CLI registry state)
    from compiler.registry.command_registry import REGISTRY

    for dialect, _ in DIALECT_BITS:
        try:
            REGISTRY._ensure_dialect_loaded(dialect)
        except Exception:  # noqa: BLE001  (best-effort load)
            pass

    out: dict[str, frozenset[str]] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        member: set[str] = set()
        for spec in specs:
            if spec.dialects is None:
                member |= ALL_DIALECTS
            else:
                member |= set(spec.dialects) & ALL_DIALECTS
        out[name] = frozenset(member)
    return out


def literal_for(member: frozenset[str]) -> str | None:
    """Render a membership set as a Rust ``dialects`` field value
    (``None`` for universal, else ``Some(DialectSet::…)``)."""
    if member == ALL_DIALECTS:
        return None
    remaining = set(member)
    parts: list[str] = []
    for const_name, const_set in NAMED_SETS:
        if const_set <= remaining:
            parts.append(const_name)
            remaining -= const_set
    # Whatever individual bits are left, in canonical order.
    for name, bit in DIALECT_BITS:
        if name in remaining:
            parts.append(bit)
            remaining.discard(name)
    expr = " | ".join(f"DialectSet::{p}" for p in parts)
    return f"Some({expr})"


_NAME_RE = re.compile(r'^(\s*)name:\s*"((?:[^"\\]|\\.)*)"\s*,')
_DIALECTS_RE = re.compile(r"^(\s*)dialects:\s*.*,\s*$")


def _unescape(name: str) -> str:
    return name.encode("utf-8").decode("unicode_escape")


def reconcile_file(path: Path, target: dict[str, str | None]) -> int:
    """Rewrite spec-level ``dialects:`` fields in *path* to match *target*
    (keyed by command name). Returns the number of edits made."""
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    out: list[str] = []
    edits = 0
    i = 0
    n = len(lines)
    while i < n:
        line = lines[i]
        m = _NAME_RE.match(line)
        if m is None:
            out.append(line)
            i += 1
            continue
        indent = m.group(1)
        cmd = _unescape(m.group(2))
        if cmd not in target:
            out.append(line)
            i += 1
            continue
        want = target[cmd]
        want_field = "None" if want is None else want
        out.append(line)
        i += 1
        # Scan this spec block: lines until the next same-indent `name:`
        # (a sibling spec) or a dedent below `indent`. The spec-level
        # `dialects:` is the one at exactly `indent`.
        replaced = False
        block: list[str] = []
        while i < n:
            cur = lines[i]
            nm = _NAME_RE.match(cur)
            if nm is not None and len(nm.group(1)) <= len(indent):
                break  # next sibling/parent spec
            cur_indent_match = re.match(r"^(\s*)\S", cur)
            cur_indent = cur_indent_match.group(1) if cur_indent_match else ""
            dm = _DIALECTS_RE.match(cur)
            if dm is not None and cur_indent == indent and not replaced:
                if cur.strip() != f"dialects: {want_field},":
                    block.append(f"{indent}dialects: {want_field},\n")
                    edits += 1
                else:
                    block.append(cur)
                replaced = True
            else:
                block.append(cur)
            i += 1
        if not replaced:
            # No spec-level dialects field (defaulted via
            # `..CommandSpec::DEFAULT`); insert one right after `name:`.
            block.insert(0, f"{indent}dialects: {want_field},\n")
            edits += 1
        out.extend(block)
    if edits:
        path.write_text("".join(out), encoding="utf-8")
    return edits


def load_rust_membership(path: Path) -> dict[str, frozenset[str]]:
    """Parse the Rust membership dump (``name<TAB>dialect<TAB>…`` per line)
    produced by the ``dump_membership`` test."""
    out: dict[str, frozenset[str]] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        parts = raw.split("\t")
        if parts:
            out[parts[0]] = frozenset(parts[1:]) & ALL_DIALECTS
    return out


def main() -> int:
    repo = Path(__file__).resolve().parents[2]
    commands_dir = repo / "rust" / "tcl-registry" / "src" / "commands"
    if not commands_dir.is_dir():
        print(f"error: {commands_dir} not found", file=sys.stderr)
        return 2

    pym = py_membership()

    # Scope edits to commands whose *current* Rust membership diverges from
    # Python (keeps the diff to the genuine fixes rather than re-rendering
    # every spec). The dump path is the first CLI arg or the default temp
    # location written by the `dump_membership` test.
    rs_dump = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("/tmp/rs_membership.txt")
    if not rs_dump.is_file():
        print(
            f"error: Rust membership dump {rs_dump} not found; run the "
            "`dump_membership` test first (see the script header)",
            file=sys.stderr,
        )
        return 2
    rsm = load_rust_membership(rs_dump)

    target: dict[str, str | None] = {}
    for name, member in pym.items():
        if name in rsm and rsm[name] != member:
            target[name] = literal_for(member)
    print(f"divergent commands to reconcile: {len(target)}")

    total = 0
    touched = 0
    for path in sorted(commands_dir.rglob("*.rs")):
        edits = reconcile_file(path, target)
        if edits:
            touched += 1
            total += edits
            print(f"  {edits:3d}  {path.relative_to(repo)}")
    print(f"reconciled {total} dialect field(s) across {touched} file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
