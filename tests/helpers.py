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

"""Shared test helpers.

Canonical implementations of helper functions that were previously
duplicated across multiple test files.
"""

from __future__ import annotations

from analyser import analyse
from compiler.cfg import build_cfg
from compiler.core_analyses import analyse_function
from compiler.lowering import lower_to_ir
from compiler.parsing.lexer import TclLexer
from compiler.ssa import build_ssa
from compiler.types import TypeLattice
from shared.tokens import Token, TokenType

# Lexer helpers


def lex(source: str) -> list[Token]:
    """Return non-SEP, non-EOL tokens from *source*."""
    return [
        t for t in TclLexer(source).tokenise_all() if t.type not in (TokenType.SEP, TokenType.EOL)
    ]


# Diagnostic helpers


def diag_codes(source: str) -> list[str]:
    """Return diagnostic codes produced by analysing *source*."""
    result = analyse(source)
    return [d.code for d in result.diagnostics]


# Compiler / type-inference helpers


def analyse_types(source: str):
    """Lower source, build CFG/SSA, run full analysis including types."""
    ir = lower_to_ir(source)
    cfg_module = build_cfg(ir)
    ssa = build_ssa(cfg_module.top_level)
    return analyse_function(cfg_module.top_level, ssa)


def var_type(analysis, var_name: str, version: int | None = None) -> TypeLattice | None:
    """Find the inferred type of a variable, defaulting to the highest version."""
    if version is not None:
        return analysis.types.get((var_name, version))
    best_ver = 0
    best_type = None
    for (name, ver), t in analysis.types.items():
        if name == var_name and ver > best_ver:
            best_ver = ver
            best_type = t
    return best_type
