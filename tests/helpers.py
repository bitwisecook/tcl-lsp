"""Shared test helpers.

Canonical implementations of helper functions that were previously
duplicated across multiple test files.
"""

from __future__ import annotations

from core.analysis.analyser import analyse
from core.compiler.cfg import build_cfg
from core.compiler.core_analyses import analyse_function
from core.compiler.lowering import lower_to_ir
from core.compiler.ssa import build_ssa
from core.compiler.types import TypeLattice
from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token, TokenType

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
