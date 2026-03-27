r"""W201 -- manual path concatenation instead of ``file join``.

Detects ``set`` assignments where the value is an interpolated string
containing both a literal path separator (``/`` or ``\``) and a variable
reference.  Values that are pure command substitutions (``[...]``) are
skipped because the slash/backslash is internal to the command, not
manual path construction.

This replaces the former syntactic check in ``_style.py`` by running
inside the taint system where token-level decomposition is already
available and forward-scan suppression via ``file normalize`` /
``file join`` is natural.
"""

from __future__ import annotations

import re

from ...analysis.checks._helpers import _build_file_join_fix
from ...analysis.semantic_model import CodeFix
from ...commands.registry.taint_hints import TaintColour
from ...common.codes import diag
from ...parsing.lexer import TclLexer
from ...parsing.tokens import TokenType
from ..cfg import CFGFunction
from ..ir import IRAssignValue
from ..ssa import SSAFunction, SSAValueKey
from ..value_shapes import is_pure_var_ref, parse_command_substitution
from ._lattice import TaintLattice
from ._types import TaintWarning

diag("W201", "Manual path concatenation \u2014 use `file join` instead.", section="warning")

# Known Tcl backslash escape sequences that are NOT path separators.
_TCL_ESCAPE_RE = re.compile(
    r"\\(?:"
    r"[abfnrtv\\{}\[\]$\"; ]"  # single-char escapes
    r"|x[0-9a-fA-F]{1,2}"  # \xNN
    r"|u[0-9a-fA-F]{1,4}"  # \uNNNN
    r"|U[0-9a-fA-F]{1,8}"  # \UNNNNNNNN
    r"|[0-7]{1,3}"  # \NNN octal
    r"|\n[ \t]*"  # line continuation
    r")"
)


def _has_path_backslash(text: str) -> bool:
    """Return True if *text* contains a backslash used as a path separator.

    Strips known Tcl escape sequences first so that ``\\n``, ``\\t``,
    etc. are not mistaken for path separators.
    """
    stripped = _TCL_ESCAPE_RE.sub("", text)
    return "\\" in stripped


def _value_has_path_concat(value: str) -> bool:
    """Return True if *value* mixes literal path separators with variables.

    Only literal (ESC/STR) tokens are checked for path separators;
    content inside command substitutions (CMD tokens) is opaque.
    """
    stripped = value.strip()
    if is_pure_var_ref(stripped):
        return False
    if parse_command_substitution(stripped) is not None:
        return False

    has_path_sep = False
    has_var = False

    lexer = TclLexer(stripped)
    while True:
        tok = lexer.get_token()
        if tok is None or tok.type in (TokenType.EOL, TokenType.EOF):
            break
        if tok.type is TokenType.VAR:
            has_var = True
        elif tok.type is TokenType.CMD:
            # Opaque: slashes inside command substitutions are not path concat.
            continue
        elif tok.type in (TokenType.SEP,):
            continue
        else:
            # ESC / STR / other literal tokens
            if "/" in tok.text or _has_path_backslash(tok.text):
                has_path_sep = True

    return has_path_sep and has_var


def _is_file_normalize_of(value: str, var_name: str) -> bool:
    """Return True if *value* is ``[file normalize $var_name]``."""
    parsed = parse_command_substitution(value)
    if parsed is None:
        return False
    cmd, args = parsed
    if cmd != "file" or not args or args[0] != "normalize":
        return False
    # Check if one of the remaining args is a reference to the variable.
    for arg in args[1:]:
        stripped = arg.strip().strip('"')
        if stripped == f"${var_name}" or stripped == f"${{{var_name}}}":
            return True
    return False


def _find_path_concat_warnings(
    cfg: CFGFunction,
    ssa: SSAFunction,
    taints: dict[SSAValueKey, TaintLattice] | None,
    executable_blocks: set[str],
) -> list[TaintWarning]:
    """Find ``set`` assignments that use manual path concatenation.

    Returns W201 warnings.
    """
    warnings: list[TaintWarning] = []

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue

        for idx, stmt in enumerate(block.statements):
            if not isinstance(stmt, IRAssignValue):
                continue

            value = stmt.value
            if not _value_has_path_concat(value):
                continue

            var_name = stmt.name

            # Suppression (a): taint colour — if the assigned SSA value
            # carries PATH_NORMALISED the path has already been normalised.
            taint_suppressed = False
            if taints is not None and idx < len(ssa_block.statements):
                ssa_stmt = ssa_block.statements[idx]
                for def_name, def_ver in ssa_stmt.defs.items():
                    key: SSAValueKey = (def_name, def_ver)
                    taint_val = taints.get(key)
                    if taint_val is not None and taint_val.colour & TaintColour.PATH_NORMALISED:
                        taint_suppressed = True
                        break

            if taint_suppressed:
                continue

            # Suppression (b): forward-scan — if the next assignment to the
            # same variable in this block is [file normalize $var], suppress.
            suppressed = False
            for later_idx in range(idx + 1, len(block.statements)):
                later_stmt = block.statements[later_idx]
                if isinstance(later_stmt, IRAssignValue) and later_stmt.name == var_name:
                    if _is_file_normalize_of(later_stmt.value, var_name):
                        suppressed = True
                    break  # Stop at first reassignment regardless.

            if suppressed:
                continue

            # Build optional code fix.
            fixes: tuple[CodeFix, ...] = ()
            replacement = _build_file_join_fix(value)
            if replacement is not None:
                fixes = (
                    CodeFix(
                        range=stmt.range,
                        new_text=replacement,
                        description="Rewrite as [file join ...]",
                    ),
                )

            warnings.append(
                TaintWarning(
                    range=stmt.range,
                    variable=var_name,
                    sink_command="set",
                    code="W201",
                    message=(
                        "Possible manual path concatenation. Use [file join] "
                        "for portable path construction."
                    ),
                    fixes=fixes,
                )
            )

    return warnings
