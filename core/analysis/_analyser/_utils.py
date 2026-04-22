"""Single-pass Tcl analyser.

Walks the token stream from TclLexer, builds a semantic model of the source:
scopes, proc definitions, variable definitions, and emits diagnostics for
detectable errors.
"""

from __future__ import annotations

import logging
import re
import time
from dataclasses import dataclass, field

from ...commands.registry import REGISTRY
from ...commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    CommandSig,
    SubcommandSig,
    arg_indices_for_role,
    iter_body_arguments,
)
from ...commands.registry.signatures import Arity
from ...common.alias import detect_interp_alias, resolve_alias
from ...common.codes import diag
from ...common.dialect import active_dialect
from ...common.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from ...common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ...common.ranges import position_from_relative, range_from_token
from ...compiler.cfg import CFGBranch, CFGFunction
from ...compiler.compilation_unit import CompilationUnit, FunctionUnit, ensure_compilation_unit
from ...compiler.compiler_checks import iter_ir_statements, run_compiler_checks
from ...compiler.core_analyses import FunctionAnalysis, LatticeKind, LatticeValue
from ...compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRIncr,
    IRProcedure,
    IRStatement,
    IRSwitch,
    when_event_name,
)
from ...compiler.ssa import SSAFunction
from ...parsing.argv import widen_argv_tokens_to_word_spans
from ...parsing.command_segmenter import SegmentedCommand, UnclosedDelimiter
from ...parsing.expr_lexer import (
    BUILTIN_EXPR_OPS,
    BUILTIN_MATH_FUNCTIONS,
    IRULES_EXPR_OPS,
    ExprTokenType,
    tokenise_expr,
)
from ...parsing.known_commands import known_command_names
from ...parsing.lexer import TclLexer
from ...parsing.recovery import segment_with_recovery
from ...parsing.tokens import SourcePosition, Token, TokenType
from ..proc_arg_traits import infer_param_traits
from ..semantic_model import (
    AnalysisResult,
    AutoPathEntry,
    ClassDef,
    CodeFix,
    CommandInvocation,
    Diagnostic,
    MethodDef,
    NamespaceImport,
    PackageProvide,
    PackageRequire,
    ParamDef,
    ProcDef,
    PropertyDef,
    Range,
    RegexPattern,
    Scope,
    Severity,
    SourceTarget,
    UnknownProcInfo,
    VarDef,
)
from ..stub_comments import scan_source_for_stubs

log = logging.getLogger(__name__)

# iRules commands that are only valid at the top level of an iRule script.
# Derived from ``irules_top_level_only`` on CommandSpec at first use.
_IRULES_TOP_LEVEL_ONLY: frozenset[str] | None = None


def _irules_top_level_only() -> frozenset[str]:
    global _IRULES_TOP_LEVEL_ONLY  # noqa: PLW0603
    if _IRULES_TOP_LEVEL_ONLY is None:
        _IRULES_TOP_LEVEL_ONLY = REGISTRY.irules_top_level_only_commands()
    return _IRULES_TOP_LEVEL_ONLY


# Module-level registrations for codes emitted from class methods.
diag("E200", "Shimmer parse error — internal representation cannot be determined.", section="error")
diag("E101", "Syntax error — unclosed bracket.", section="error", internal=True)
diag("E103", "Syntax error — unexpected token.", section="error", internal=True)
diag(
    "H300",
    "Possible paste error — repeated assignment to same variable with same value.",
    section="hint",
)
diag("W113", "Procedure shadows built-in command.", section="warning")
diag("IRULE5006", "Top-level-only command used inside a nested body.", section="irules")
diag(
    "IRULE5007", "Event-context command used at top level outside a `when` block.", section="irules"
)
diag("W116", "Stub command shadows built-in command.", section="warning")
diag("W117", "Stub expression definition shadows built-in function or operator.", section="warning")
diag("W124", "Invalid IP address literal.", section="warning")
diag(
    "W125",
    "Orphaned control-flow keyword used as standalone command.",
    section="warning",
)
diag("W126", "Non-channel value in channel argument position.", section="warning")
diag("W210", "Variable read before set.", section="variable")
diag("W211", "Variable set but never used.", section="variable")
diag(
    "W213",
    "Variable may not exist — use `unset -nocomplain` to suppress the error.",
    section="variable",
)
diag(
    "W214",
    "Unused proc parameter — argument is declared but never read in the procedure body.",
    section="variable",
)
diag("W220", "Dead store — variable set but overwritten before use.", section="variable")
diag(
    "IRULE4005",
    "Potential race — `static::` variable written outside `RULE_INIT` and read in another event.",
    section="irules_variable",
)
diag(
    "IRULE5005",
    "Direct proc invocation without `call` — use `call proc_name`.",
    section="irules",
)
diag(
    "W123",
    "Unresolved command — not found in registry, user procs, or `unknown` handler.",
    section="hint",
    default=False,
)

# Short names: d = Diagnostic, m = regex Match, r = Range,
# s = Scope, t = Token, p = ParamDef.


_UNUSED_VAR_RE = re.compile(r"Variable '([^']+)' is set but never used")

# Inline suppression via Tcl comments: bare form suppresses all diagnostics,
# ``# <noqa>: W100,W101`` suppresses specific codes on the following command.
_NOQA_ALL = frozenset({"*"})  # sentinel for "suppress everything"


def _possible_paste_fingerprint(stmt: IRStatement) -> tuple[str, str] | None:
    """Return (variable, static_value) for static assignments worth heuristic checks."""
    if isinstance(stmt, IRAssignConst):
        return (stmt.name, stmt.value.strip())

    if isinstance(stmt, IRAssignValue):
        value = stmt.value.strip()
        if not value:
            return None
        if "$" in value or "[" in value or "]" in value:
            return None
        return (stmt.name, value)

    return None


def _format_literal_for_message(value: str) -> str:
    """Keep heuristic diagnostic literals short and single-line."""
    display = value.replace("\n", "\\n")
    if len(display) > 40:
        return display[:37] + "..."
    return display


def _argv_with_word_spans(argv: list[Token], all_tokens: list[Token]) -> list[Token]:
    """Return argv tokens widened to each Tcl word's full token span."""
    return widen_argv_tokens_to_word_spans(argv, all_tokens)


def parse_param_list(param_str: str) -> list[ParamDef]:
    """Parse a Tcl proc argument list string into ParamDef objects.

    Handles:  "a b c"  and  "a {b default} c"
    """
    params: list[ParamDef] = []
    # Simple word-level parse -- braced items contain defaults
    i = 0
    text = param_str.strip()
    while i < len(text):
        # skip whitespace
        while i < len(text) and text[i] in " \t\n\r":
            i += 1
        if i >= len(text):
            break

        if text[i] == "{":
            # Braced parameter with possible default
            level = 1
            i += 1
            start = i
            while i < len(text) and level > 0:
                if text[i] == "{":
                    level += 1
                elif text[i] == "}":
                    level -= 1
                i += 1
            inner = text[start : i - 1].strip()
            parts = inner.split(None, 1)
            if len(parts) == 2:
                params.append(ParamDef(name=parts[0], has_default=True, default_value=parts[1]))
            elif len(parts) == 1:
                params.append(ParamDef(name=parts[0]))
        else:
            # Bare word
            start = i
            while i < len(text) and text[i] not in " \t\n\r":
                i += 1
            word = text[start:i]
            if word:
                params.append(ParamDef(name=word))

    return params


