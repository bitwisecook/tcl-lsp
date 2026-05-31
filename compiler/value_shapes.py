"""Shared Tcl value-shape helpers used across compiler passes."""

from __future__ import annotations

import re

# A single ``$name`` / ``$ns::name`` / ``$arr(idx)`` reference and nothing else.
# Crucially it must NOT match a *concatenation* of references (``$x$y``,
# ``$x_$y``, ``$x.foo``): that double-substitutes a composed value, so e.g.
# ``uplevel 1 $x$y`` is not the safe single-var idiom and must still warn (W301).
_SINGLE_VAR_REF = re.compile(r"\$[\w:]+(\([^)]*\))?")


def is_pure_var_ref(text: str) -> bool:
    """Return True if *text* is exactly one variable reference (``$x`` /
    ``${x}`` / ``$ns::x`` / ``$arr(idx)``) with no surrounding or concatenated
    syntax."""
    if text.startswith("${") and text.endswith("}"):
        inner = text[2:-1]
        return "}" not in inner
    return bool(_SINGLE_VAR_REF.fullmatch(text))


def parse_command_substitution(text: str) -> tuple[str, tuple[str, ...]] | None:
    """Extract command name and args from ``[cmd ...]`` using the Tcl
    lexer.

    Properly handles braced words (``[foo {a b} c]``), quoted words
    (``[foo "a b" c]``), nested substitutions (``[foo [bar] baz]``),
    namespaced command names (``[::ns::foo a]``), and any other Tcl
    word-splitting rule -- delegates to the shared
    ``compiler.parsing.command_segmenter.segment_commands`` rather
    than re-implementing word splitting.

    Returns ``None`` when *text* isn't surrounded by ``[`` / ``]``,
    when the segmenter can't parse the contents into at least one
    command, or when the cmd-sub contains multiple semicolon-
    separated commands (we don't know which one's return value is
    bound, so caller treats it as opaque).
    """
    from compiler.parsing.command_segmenter import segment_commands

    stripped = text.strip()
    if not (stripped.startswith("[") and stripped.endswith("]")):
        return None
    inner = stripped[1:-1]
    try:
        cmds = segment_commands(inner)
    except Exception:
        return None
    # A cmd-sub that contains multiple commands (``[cmd1; cmd2]``)
    # returns the LAST command's value; we don't model that here
    # because callers want to identify a single called proc.  Stay
    # conservative and bail.
    if len(cmds) != 1:
        return None
    cmd = cmds[0]
    if not cmd.texts:
        return None
    return cmd.texts[0], tuple(cmd.texts[1:])
