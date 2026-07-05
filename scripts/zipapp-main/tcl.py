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

"""Unified Tcl tools zipapp entry point.

Usage: tcl <verb> [args...]

Verbs:
  opt         optimise source text
  diag        run diagnostics across inputs
  lint        run lint diagnostics across inputs
  validate    validate syntax/error diagnostics
  format      format source text
  symbols     emit symbol definitions
  diagram     extract control-flow diagram data
  callgraph   build procedure call graph data
  symbolgraph build symbol relationship graph data
  dataflow    build taint/effect data-flow graph data
  command-info lookup command registry metadata
  completion  emit a bash/fish/zsh completion script
  find-legacy detect legacy modernisation patterns (detection only)
  dis         emit bytecode disassembly
  compwasm    compile to WASM binary
  highlight   emit syntax-highlighted output
  diff        diff two sources via AST/IR/CFG
  explore     compiler-explorer views (IR/CFG/SSA/optimiser/etc.)
  help        search KCS docs

iRules-specific verbs (event-order, event-info) live on the f5 CLI
under `f5 irule <subverb>`.
"""

from __future__ import annotations

import sys
from pathlib import Path


def _infer_prog_name(argv0: str) -> str:
    raw_name = Path(argv0).name.strip()
    if not raw_name:
        return "tcl"

    stem = Path(raw_name).stem
    if not stem:
        return "tcl"

    lowered = stem.lower()
    if lowered.startswith("python"):
        return "tcl"
    if lowered.startswith("tcl-"):
        return "tcl"
    return stem


def main() -> int:
    prog_name = _infer_prog_name(sys.argv[0])

    if len(sys.argv) <= 1 or sys.argv[1] in {"-h", "--help"}:
        try:
            from shared._build_info import BUILD_TIMESTAMP, FULL_VERSION
        except ImportError:
            FULL_VERSION = "dev"
            BUILD_TIMESTAMP = ""
        version = FULL_VERSION
        if BUILD_TIMESTAMP:
            version += f" ({BUILD_TIMESTAMP})"
        print(f"{prog_name} {version}")
        print("Unified Tcl toolchain CLI")
        print()
        print(f"Usage: {prog_name} <verb> [args...]")
        print()
        print("Verbs:")
        print("  opt         optimise source text")
        print("  diag        run diagnostics across inputs")
        print("  lint        run lint diagnostics across inputs")
        print("  validate    validate syntax/error diagnostics")
        print("  format      format source text")
        print("  symbols     emit symbol definitions")
        print("  diagram     extract control-flow diagram data")
        print("  callgraph   build procedure call graph data")
        print("  symbolgraph build symbol relationship graph data")
        print("  dataflow    build taint/effect data-flow graph data")
        print("  command-info lookup command registry metadata")
        print("  completion  emit a bash/fish/zsh completion script")
        print("  find-legacy detect legacy modernisation patterns (detection only)")
        print("  dis         emit bytecode disassembly")
        print("  compwasm    compile to WASM binary")
        print("  highlight   emit syntax-highlighted output")
        print("  diff        diff two sources via AST/IR/CFG")
        print("  explore     compiler-explorer views")
        print("  help        search KCS docs")
        print()
        print(f"Run `{prog_name} <verb> --help` for verb-specific options.")
        return 0

    from tooling.tcl.main import main as tcl_main

    return tcl_main(sys.argv[1:], prog_name=prog_name)


sys.exit(main())
