r"""W201 -- manual path concatenation instead of ``file join``.

Detects ``set`` assignments where the rendered value contains both a
literal path separator (``/`` or ``\``) and a variable or path-returning
command result.  Escape sequences are resolved via ``backslash_subst()``
so that ``\n``, ``\t`` etc. are correctly recognised as non-path
characters.  Command substitution contents are opaque -- slashes inside
``[...]`` are never mistaken for path concatenation.

Detection uses two sources of information:

* **Taint lattice** -- ``PATH_NORMALISED`` suppresses the warning;
  ``PATH_PREFIXED`` on a CMD token indicates the command returns
  path-like content.
* **Rendered literal analysis** -- ``backslash_subst()`` renders escape
  sequences in ESC tokens (gated by ``IRAssignValue.value_needs_backsubst``)
  before checking for ``/`` or ``\``.

This replaces the former syntactic check in ``_style.py`` by running
inside the taint system where token-level decomposition, escape
rendering, and SSA-level suppression are natural.
"""

from __future__ import annotations

from ...analysis.checks._helpers import _build_file_join_fix
from ...analysis.semantic_model import CodeFix
from ...commands.registry.taint_hints import TaintColour
from ...common.codes import diag
from ...parsing.lexer import TclLexer
from ...parsing.substitution import backslash_subst
from ...parsing.tokens import TokenType
from ..cfg import CFGFunction
from ..core_analyses import LatticeValue
from ..ir import IRAssignValue
from ..ssa import SSAFunction, SSAValueKey
from ..value_shapes import is_pure_var_ref, parse_command_substitution
from ._lattice import TaintLattice
from ._types import TaintWarning

diag("W201", "Manual path concatenation \u2014 use `file join` instead.", section="warning")


def _rendered_text_has_path_sep(text: str, is_esc: bool) -> bool:
    """Return True if *text* contains a path separator after escape rendering.

    ESC tokens always have escapes rendered (they come from double-quoted
    or bare-word contexts where Tcl processes backslash sequences).
    STR tokens (braced strings) are checked as-is since braces suppress
    escape processing.

    After rendering, ``\\n`` becomes a newline (not a backslash) and
    ``\\\\`` becomes a single backslash (a genuine path separator on
    Windows).
    """
    rendered = backslash_subst(text) if (is_esc and "\\" in text) else text
    return "/" in rendered or "\\" in rendered


def _cmd_returns_path(cmd_text: str) -> bool:
    """Return True if the command substitution is known to return a path.

    Checks for ``file`` subcommands that produce path values and other
    common path-returning commands (``pwd``, ``info script``, etc.).
    """
    parsed = parse_command_substitution(f"[{cmd_text}]")
    if parsed is None:
        return False
    cmd, args = parsed
    if cmd == "pwd":
        return True
    if cmd == "file" and args:
        return args[0] in (
            "dirname",
            "join",
            "normalize",
            "nativename",
            "rootname",
            "tail",
            "extension",
            "readlink",
            "tempdir",
            "tempfile",
        )
    if cmd == "info" and args and args[0] in ("script", "nameofexecutable", "library"):
        return True
    return False


def _sccp_resolves_to_path_sep(
    cmd_text: str,
    sccp_values: dict[SSAValueKey, LatticeValue] | None,
) -> bool:
    """Return True if SCCP resolves a command result to a path separator.

    Handles cases like ``[string cat /]`` that statically produce a
    path separator character.
    """
    if sccp_values is None:
        return False
    # We cannot easily map a CMD token back to its SSA def here, so
    # this is a placeholder for future SCCP integration.  Currently
    # the detection relies on the literal-token scan and CMD-returns-path
    # heuristic.
    return False


def _literals_have_path_separator(value: str) -> bool:
    """Return True if literal tokens in *value* contain path separators.

    Only ESC/STR tokens are inspected; CMD tokens are opaque and VAR
    tokens carry no literal text.  ESC tokens are rendered through
    ``backslash_subst()`` to resolve escape sequences before checking.
    """
    stripped = value.strip()
    if is_pure_var_ref(stripped):
        return False
    if parse_command_substitution(stripped) is not None:
        return False

    lexer = TclLexer(stripped)
    while True:
        tok = lexer.get_token()
        if tok is None or tok.type in (TokenType.EOL, TokenType.EOF):
            break
        if tok.type in (TokenType.CMD, TokenType.VAR, TokenType.SEP):
            continue
        # ESC / STR -- literal text
        is_esc = tok.type is TokenType.ESC
        if _rendered_text_has_path_sep(tok.text, is_esc=is_esc):
            return True
    return False


def _has_variable_or_path_cmd(value: str) -> bool:
    """Return True if *value* contains a variable ref or path-returning CMD.

    A plain variable (``$x``) indicates user-controlled path content.
    A command substitution that returns path content (e.g.
    ``[file dirname $x]``) similarly indicates a path component being
    concatenated with literal separators.
    """
    stripped = value.strip()
    if is_pure_var_ref(stripped):
        return False
    if parse_command_substitution(stripped) is not None:
        return False

    lexer = TclLexer(stripped)
    while True:
        tok = lexer.get_token()
        if tok is None or tok.type in (TokenType.EOL, TokenType.EOF):
            break
        if tok.type is TokenType.VAR:
            return True
        if tok.type is TokenType.CMD and _cmd_returns_path(tok.text):
            return True
    return False


def _is_file_normalize_of(value: str, var_name: str) -> bool:
    """Return True if *value* is ``[file normalize $var_name]``."""
    parsed = parse_command_substitution(value)
    if parsed is None:
        return False
    cmd, args = parsed
    if cmd != "file" or not args or args[0] != "normalize":
        return False
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

    Detection:
      1. The rendered literal tokens contain ``/`` or ``\\`` (after
         ``backslash_subst`` when ``value_needs_backsubst`` is set).
      2. The value contains a variable reference or a command
         substitution known to return path content.

    Suppression via taint lattice:
      - ``PATH_NORMALISED`` on the assigned SSA value suppresses.

    Suppression via forward-scan:
      - The next assignment to the same variable in the same block
        is ``[file normalize $var]``.
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

            # Step 1: do the rendered literals contain path separators?
            if not _literals_have_path_separator(value):
                continue

            # Step 2: is there a variable or path-returning CMD?
            if not _has_variable_or_path_cmd(value):
                continue

            var_name = stmt.name

            # Suppression (a): taint lattice -- PATH_NORMALISED means the
            # value has been through file normalize/join.
            if taints is not None and idx < len(ssa_block.statements):
                ssa_stmt = ssa_block.statements[idx]
                suppressed = False
                for def_name, def_ver in ssa_stmt.defs.items():
                    key: SSAValueKey = (def_name, def_ver)
                    taint_val = taints.get(key)
                    if taint_val is not None and bool(
                        taint_val.colour & TaintColour.PATH_NORMALISED
                    ):
                        suppressed = True
                        break
                if suppressed:
                    continue

            # Suppression (b): forward-scan -- next assignment to same var
            # is [file normalize $var].
            suppressed = False
            for later_idx in range(idx + 1, len(block.statements)):
                later_stmt = block.statements[later_idx]
                if isinstance(later_stmt, IRAssignValue) and later_stmt.name == var_name:
                    if _is_file_normalize_of(later_stmt.value, var_name):
                        suppressed = True
                    break
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
