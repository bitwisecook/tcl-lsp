#!/usr/bin/env python3
"""Split core/analysis/analyser.py into _analyser/ mixin package.

Usage: python _split_analyser.py
"""
from pathlib import Path

SRC = Path("core/analysis/analyser.py")
DST = Path("core/analysis/_analyser")
DST.mkdir(exist_ok=True)

text = SRC.read_text()
lines = text.splitlines(keepends=True)
total = len(lines)
print(f"Source: {total} lines")


def get(start: int, end: int) -> str:
    """Return lines start..end inclusive (1-indexed)."""
    return "".join(lines[i - 1] for i in range(start, min(end, total) + 1))


# ---------------------------------------------------------------------------
# Shared import header (all imports from original analyser.py, depth +1)
# ---------------------------------------------------------------------------

SHARED_IMPORTS = """\
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
"""

# ---------------------------------------------------------------------------
# _utils.py  — module-level code (lines 1–241)
# ---------------------------------------------------------------------------

utils_body = get(1, 241)
# Adjust import depths: single-dot → double-dot, double-dot → triple-dot
# The original has `from ..xxx` (going up from core/analysis/),
# the new file is at core/analysis/_analyser/ so needs `from ...xxx`.
utils_body = utils_body.replace("from ..commands.", "from ...commands.")
utils_body = utils_body.replace("from ..common.", "from ...common.")
utils_body = utils_body.replace("from ..compiler.", "from ...compiler.")
utils_body = utils_body.replace("from ..parsing.", "from ...parsing.")
utils_body = utils_body.replace("from .proc_arg_traits", "from ..proc_arg_traits")
utils_body = utils_body.replace("from .semantic_model", "from ..semantic_model")
utils_body = utils_body.replace("from .stub_comments", "from ..stub_comments")
# Also need widen_argv_tokens_to_word_spans (from ..parsing.argv →
# from ...parsing.argv) — already covered by the ..parsing. replacement above.

(DST / "_utils.py").write_text(utils_body)
print(f"wrote _utils.py ({len(utils_body.splitlines())} lines)")

# ---------------------------------------------------------------------------
# _snapshot.py  — AnalyserSnapshot dataclass (lines 242–266)
# ---------------------------------------------------------------------------

SNAPSHOT_HEADER = """\
from __future__ import annotations

from dataclasses import dataclass, field

from ..semantic_model import AnalysisResult, Range, Scope

"""

snapshot_body = get(242, 266)
(DST / "_snapshot.py").write_text(SNAPSHOT_HEADER + snapshot_body)
print(f"wrote _snapshot.py ({len((SNAPSHOT_HEADER + snapshot_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _core.py  — _AnalyserBase
# Lines 271–562, 612–775, 1197–1265, 4489–4525
# (_set_const_string etc. at 563–611 go to _scope.py instead)
# ---------------------------------------------------------------------------

CORE_HEADER = SHARED_IMPORTS + """\
from ._snapshot import AnalyserSnapshot
from ._utils import (
    _irules_top_level_only,
    _NOQA_ALL,
    _UNUSED_VAR_RE,
    _possible_paste_fingerprint,
    _format_literal_for_message,
    _argv_with_word_spans,
)

log = logging.getLogger(__name__)

"""

core_body = (
    "class _AnalyserBase:\n"
    '    """Core analysis loop — init, public API, body/expr traversal."""\n\n'
)
core_body += get(271, 562)
core_body += get(612, 775)
core_body += get(1197, 1265)
core_body += get(4489, 4525)

(DST / "_core.py").write_text(CORE_HEADER + core_body)
print(f"wrote _core.py ({len((CORE_HEADER + core_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _scope.py  — _AnalyserScopeMixin
# Lines 563–611, 1882–1926, 2053–2084, 3316–3339, 4526–4558
# ---------------------------------------------------------------------------

SCOPE_HEADER = SHARED_IMPORTS + """\
log = logging.getLogger(__name__)

"""

scope_body = "class _AnalyserScopeMixin:\n    \"\"\"Variable & scope tracking helpers.\"\"\"\n\n"
scope_body += get(563, 611)
scope_body += get(1882, 1926)
scope_body += get(2053, 2084)
scope_body += get(3316, 3339)
scope_body += get(4526, 4558)

(DST / "_scope.py").write_text(SCOPE_HEADER + scope_body)
print(f"wrote _scope.py ({len((SCOPE_HEADER + scope_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _recovery.py  — _AnalyserRecoveryMixin
# Lines 776–1196
# ---------------------------------------------------------------------------

RECOVERY_HEADER = SHARED_IMPORTS + """\
log = logging.getLogger(__name__)

"""

recovery_body = "class _AnalyserRecoveryMixin:\n    \"\"\"Parser-error recovery heuristics.\"\"\"\n\n"
recovery_body += get(776, 1196)

(DST / "_recovery.py").write_text(RECOVERY_HEADER + recovery_body)
print(f"wrote _recovery.py ({len((RECOVERY_HEADER + recovery_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _commands.py  — _AnalyserCommandsMixin
# Lines 1266–1776
# (_ORPHANED_KEYWORDS class constant at 1301 is inside this range)
# ---------------------------------------------------------------------------

COMMANDS_HEADER = SHARED_IMPORTS + """\
log = logging.getLogger(__name__)

"""

commands_body = (
    "class _AnalyserCommandsMixin:\n"
    '    """Main command dispatch: _process_command and special-case routing."""\n\n'
)
commands_body += get(1266, 1776)

(DST / "_commands.py").write_text(COMMANDS_HEADER + commands_body)
print(f"wrote _commands.py ({len((COMMANDS_HEADER + commands_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _handlers.py  — _AnalyserHandlersMixin
# Lines 1777–1881, 1927–2052, 2085–2103
# (1882–1926 _define_vars_from_list goes to _scope.py)
# ---------------------------------------------------------------------------

HANDLERS_HEADER = SHARED_IMPORTS + """\
log = logging.getLogger(__name__)

"""

handlers_body = "class _AnalyserHandlersMixin:\n    \"\"\"Special-case command handlers.\"\"\"\n\n"
handlers_body += get(1777, 1881)
handlers_body += get(1927, 2052)
handlers_body += get(2085, 2103)

(DST / "_handlers.py").write_text(HANDLERS_HEADER + handlers_body)
print(f"wrote _handlers.py ({len((HANDLERS_HEADER + handlers_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _proc.py  — _AnalyserProcMixin
# Lines 2104–2234, 2946–3315
# ---------------------------------------------------------------------------

PROC_HEADER = SHARED_IMPORTS + """\
log = logging.getLogger(__name__)

"""

proc_body = "class _AnalyserProcMixin:\n    \"\"\"Proc definition, resolution, call-arity checks, and low-level handlers.\"\"\"\n\n"
proc_body += get(2104, 2234)
proc_body += get(2946, 3315)

(DST / "_proc.py").write_text(PROC_HEADER + proc_body)
print(f"wrote _proc.py ({len((PROC_HEADER + proc_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _oo.py  — _AnalyserOOMixin
# Lines 2235–2751
# ---------------------------------------------------------------------------

OO_HEADER = SHARED_IMPORTS + """\
log = logging.getLogger(__name__)

"""

oo_body = "class _AnalyserOOMixin:\n    \"\"\"TclOO class/method analysis.\"\"\"\n\n"
oo_body += get(2235, 2751)

(DST / "_oo.py").write_text(OO_HEADER + oo_body)
print(f"wrote _oo.py ({len((OO_HEADER + oo_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _diagnostics.py  — _AnalyserDiagsMixin
# Lines 2752–2945, 3340–4488
# ---------------------------------------------------------------------------

DIAGS_HEADER = SHARED_IMPORTS + """\
from ._utils import (
    _irules_top_level_only,
    _NOQA_ALL,
    _UNUSED_VAR_RE,
    _possible_paste_fingerprint,
    _format_literal_for_message,
)

log = logging.getLogger(__name__)

"""

diags_body = "class _AnalyserDiagsMixin:\n    \"\"\"Diagnostic emission methods.\"\"\"\n\n"
diags_body += get(2752, 2945)
diags_body += get(3340, 4488)

(DST / "_diagnostics.py").write_text(DIAGS_HEADER + diags_body)
print(f"wrote _diagnostics.py ({len((DIAGS_HEADER + diags_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# __init__.py  — assembles Analyser and re-exports public API
# ---------------------------------------------------------------------------

INIT_CONTENT = """\
from __future__ import annotations

from ._core import _AnalyserBase
from ._commands import _AnalyserCommandsMixin
from ._diagnostics import _AnalyserDiagsMixin
from ._handlers import _AnalyserHandlersMixin
from ._oo import _AnalyserOOMixin
from ._proc import _AnalyserProcMixin
from ._recovery import _AnalyserRecoveryMixin
from ._scope import _AnalyserScopeMixin
from ._snapshot import AnalyserSnapshot
from ._utils import parse_param_list


class Analyser(
    _AnalyserBase,
    _AnalyserRecoveryMixin,
    _AnalyserScopeMixin,
    _AnalyserCommandsMixin,
    _AnalyserHandlersMixin,
    _AnalyserProcMixin,
    _AnalyserOOMixin,
    _AnalyserDiagsMixin,
):
    "Single-pass Tcl analyser assembled from mixin groups."


__all__ = ["Analyser", "AnalyserSnapshot", "parse_param_list"]
"""

(DST / "__init__.py").write_text(INIT_CONTENT)
print(f"wrote __init__.py ({len(INIT_CONTENT.splitlines())} lines)")

# ---------------------------------------------------------------------------
# Rewrite analyser.py as a re-export shim
# ---------------------------------------------------------------------------

SHIM = """\
\"\"\"Single-pass Tcl analyser — implementation split into _analyser/ package.\"\"\"
from ._analyser import Analyser, AnalyserSnapshot, parse_param_list

__all__ = ["Analyser", "AnalyserSnapshot", "parse_param_list"]


def analyse(source, cu=None):  # type: ignore[no-untyped-def]
    \"\"\"Convenience wrapper: create an Analyser and run it on *source*.\"\"\"
    return Analyser().analyse(source, cu=cu)
"""

SRC.write_text(SHIM)
print(f"rewrote analyser.py as shim ({len(SHIM.splitlines())} lines)")
print("Done.")
