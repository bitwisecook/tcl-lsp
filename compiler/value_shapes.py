"""Shared Tcl value-shape helpers used across compiler passes."""

from __future__ import annotations


def is_pure_var_ref(text: str) -> bool:
    """Return True if *text* is ``$x`` or ``${x}`` with no extra syntax."""
    if text.startswith("${") and text.endswith("}"):
        inner = text[2:-1]
        return "}" not in inner
    return text.startswith("$") and not any(c in text for c in ' "{}[]')


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
