#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Refresh the clean six/two-file split, optionally checking the toggled tree."""

import argparse
import difflib
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("root", type=Path)
parser.add_argument("--base", default="HEAD")
parser.add_argument("--snapshot", type=Path)
parser.add_argument("--check-toggled", action="store_true")
args = parser.parse_args()
root = args.root.resolve()
perf = root / "docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf"
paths = [
    "rust/tcl-lexer/src/lib.rs",
    "rust/tcl-lexer/src/parse_cut.rs",
    "rust/tcl-lexer/src/source_map.rs",
    "rust/tcl-compiler/src/parsing/spectcl_source/original_word.rs",
    "rust/tcl-compiler/src/command_binding/original_name_value.rs",
    "rust/tcl-compiler/src/command_binding/formal_call_advice.rs",
    "rust/tcl-compiler/src/command_binding/incomplete_body_metadata.rs",
    "rust/tcl-compiler/src/realm.rs",
    "rust/tcl-compiler/src/interprocedural.rs",
]
original = {p: subprocess.check_output(["git", "show", f"{args.base}:{p}"], cwd=root, text=True)
            for p in paths}


def prepare(destination, toggled):
    assert destination != root and not root.is_relative_to(destination)
    for relative, text in original.items():
        path = destination / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    subprocess.run(["python3", str(perf / "batch/apply_batch.py"), str(destination),
                    *(["--zz"] if toggled else [])], check=True)
    subprocess.run(["rustfmt", "--edition", "2024", "--config", "skip_children=true",
                    *[str(destination / p) for p in paths]], check=True)


with tempfile.TemporaryDirectory(prefix="zz_perf_refresh_") as temporary:
    snapshot = args.snapshot.resolve() if args.snapshot else Path(temporary) / "clean"
    prepare(snapshot, False)
    owned, routed = [], []
    for relative in paths[1:]:
        after = (snapshot / relative).read_text()
        target = routed if relative.endswith(("/realm.rs", "/interprocedural.rs")) else owned
        target.extend(difflib.unified_diff(original[relative].splitlines(True), after.splitlines(True),
                                          fromfile=f"a/{relative}", tofile=f"b/{relative}"))
    assert sum(line.startswith("--- a/") for line in owned) == 6
    assert sum(line.startswith("--- a/") for line in routed) == 2
    assert "ZZ_" not in "".join(owned + routed)
    owned = ["\n" if line == " \n" else line for line in owned]
    routed = ["\n" if line == " \n" else line for line in routed]
    (perf / "perf-owned.patch").write_text("".join(owned))
    (perf / "perf-realm-interprocedural.patch").write_text("".join(routed))
    (perf / "perf.patch").write_text("".join(owned + routed))
    if args.check_toggled:
        toggled = Path(temporary) / "toggled"
        prepare(toggled, True)
        differences = [p for p in paths if (toggled / p).read_bytes() != (root / p).read_bytes()]
        assert not differences, f"toggled helper and proof sources differ: {differences}"
print("Refreshed six owned / two routed files; no temporary switch in the patches")
