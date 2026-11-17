#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Write the registry's table of rows closed by source evidence.

Each row of `rust/tcl-registry/src/commands/closed_evidence/table.rs` names
a command (and subcommand) whose four purity domains are complete for the
windows its implementation's source closes it, from the evidence files that
`tcl_command_evidence.py` (Tcl releases' C) and `tcl_script_evidence.py`
(tcllib 2.0's Tcl) write. The candidate rows are
`tests/data/command_evidence/formerly_pure.tsv`, `group<TAB>command<TAB>subcommand`:
every row the registry authored `PURE` when purity became derived (issue
#1712). A candidate the evidence does not close stays impure.

Usage: scripts/dev/gen_closed_evidence.py
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "rust/tcl-registry/tests/data/command_evidence"
TABLE = ROOT / "rust/tcl-registry/src/commands/closed_evidence/table.rs"
TCLLIB = ROOT / "tmp/tcllib-2.0/modules"
LADDER = ["8.4", "8.5", "8.6", "9.0", "9.1"]
TCLLIB_LADDER = ["8.5", "8.6", "9.0", "9.1"]
# Rows whose authored spec deliberately leaves a domain open, which the
# evidence does not override, with the reason the row gives.
OPEN_BY_AUTHOR = {
    ("tcl", "dict", "create"): "an unknown key object still needs its string materialised",
    ("tcl", "::tcl::dict::create", ""): "an unknown key object still needs its string materialised",
    ("tcl", "binary", "format"): "an `a`/`A` cursor converts its operand to bytes in place",
}


def evidence_key(command: str, subcommand: str) -> str:
    """The evidence line a registry row reads: `::tcl::dict::get` is the
    `dict get` the ensemble map creates it as."""
    name = command.lstrip(":")
    if name.startswith("tcl::dict::") and not subcommand:
        return "dict " + name.removeprefix("tcl::dict::")
    return f"{name} {subcommand}" if subcommand else name


def closed_sets() -> dict[str, set[str]]:
    sets = {}
    for release in LADDER:
        lines = (EVIDENCE / f"tcl{release}.txt").read_text().splitlines()
        sets[release] = {
            line.split("\t")[0]
            for line in lines
            if not line.startswith("#") and line.split("\t")[1] == "closed"
        }
    return sets


def windows(releases: list[str], ladder: list[str]) -> list[tuple[str, str | None]]:
    """Contiguous runs of `releases` on `ladder`, each `(from, next)`."""
    runs = []
    start = None
    for i, release in enumerate(ladder + [None]):
        if release in releases:
            start = start or release
        elif start is not None:
            runs.append((start, release))
            start = None
    return runs


def tcllib_packages() -> dict[str, tuple[str, str]]:
    """Procedure namespace → (package, version) by `package provide`."""
    provided: dict[str, tuple[str, str]] = {}
    for path in sorted(TCLLIB.rglob("*.tcl")):
        text = path.read_text(encoding="utf-8", errors="replace")
        m = re.search(r"^\s*package provide\s+(\S+)\s+([0-9][0-9.]*)", text, re.MULTILINE)
        if not m:
            continue
        for ns in re.findall(r"^\s*namespace eval\s+:*([\w:]+)", text, re.MULTILINE):
            provided.setdefault(ns, (m.group(1), m.group(2)))
    return provided


def rust_window(window: tuple[str, str | None]) -> str:
    start, end = window
    return f'("{start}", {"None" if end is None else f"Some({chr(34)}{end}{chr(34)})"})'


def main() -> int:
    rows = set()
    for line in (EVIDENCE / "formerly_pure.tsv").read_text().splitlines():
        if line.startswith("#"):
            continue
        group, command, subcommand = (line.split("\t") + ["", ""])[:3]
        rows.add((group, command, subcommand))
    closed = closed_sets()
    operators = {
        line.split("\t")[0]
        for release in LADDER
        for line in (EVIDENCE / f"tcl{release}.txt").read_text().splitlines()
        if line.startswith("tcl::mathop::")
    }
    tcllib = {
        line.split("\t")[0]
        for line in (EVIDENCE / "tcllib.txt").read_text().splitlines()
        if not line.startswith("#")
    }
    packages = tcllib_packages()
    out = []
    for group, command, subcommand in sorted(rows):
        if (group, command, subcommand) in OPEN_BY_AUTHOR:
            continue
        if group == "tcl":
            key = evidence_key(command, subcommand)
            # A bare operator row is the `tcl::mathop` command a
            # `namespace path` reaches it through.
            if not subcommand and f"tcl::mathop::{key}" in operators:
                key = f"tcl::mathop::{key}"
            releases = [r for r in LADDER if key in closed[r]]
            if not releases:
                continue
            # One surface per window: SpecTcl spells one window per
            # `-complete`, so each round-trips on its own.
            surfaces = ", ".join(
                f"SpecSurface::core_in(Family::Tcl, &[{rust_window(w)}])"
                for w in windows(releases, LADDER)
            )
        elif group == "tcllib" and not subcommand:
            name = command.lstrip(":")
            ns = name.rpartition("::")[0]
            if name not in tcllib or ns not in packages:
                continue
            package, version = packages[ns]
            core = ", ".join(rust_window(w) for w in windows(TCLLIB_LADDER, LADDER))
            surfaces = (
                f"SpecSurface::core_in(Family::Tcl, &[{core}]), "
                f'SpecSurface::package_in("{package}", &[("{version}", Some("{version}.1"))])'
            )
        else:
            continue
        out.append(f'    ("{group}", "{command}", "{subcommand}", &[{surfaces}]),')
    TABLE.parent.mkdir(parents=True, exist_ok=True)
    TABLE.write_text(
        "// Generated by scripts/dev/gen_closed_evidence.py from\n"
        "// rust/tcl-registry/tests/data/command_evidence; do not edit.\n\n"
        "use tcl_dialect::model::{Family, SpecSurface};\n\n"
        "/// `(group, command, subcommand, surfaces)`: the row's four purity domains\n"
        "/// are complete for `surfaces`, where its source evidence closes it.\n"
        "pub(super) const CLOSED: &[(&str, &str, &str, &[SpecSurface])] = &[\n"
        + "\n".join(out)
        + "\n];\n"
    )
    subprocess.run(["rustfmt", "--edition", "2024", str(TABLE)], check=True)
    print(f"{len(out)} closed rows of {len(rows)} named")
    return 0


if __name__ == "__main__":
    sys.exit(main())
