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
    """Extract command name and args from ``[cmd ...]``."""
    stripped = text.strip()
    if not (stripped.startswith("[") and stripped.endswith("]")):
        return None
    cmd_text = stripped[1:-1].strip()
    parts = cmd_text.split()
    if not parts:
        return None
    return parts[0], tuple(parts[1:])
