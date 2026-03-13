"""Consistent naming helpers for Tcl identifiers."""

from __future__ import annotations


def normalise_var_name(name: str) -> str:
    """Normalise Tcl variable forms to their base name."""
    base = name
    if base.startswith("${") and base.endswith("}"):
        base = base[2:-1]
    elif base.startswith("$"):
        base = base[1:]
    if "(" in base:
        base = base.split("(", 1)[0]
    return base


def normalise_qualified_name(name: str) -> str:
    """Normalise a possibly-qualified Tcl command/proc name."""
    if not name:
        return name
    parts = [part for part in name.split("::") if part]
    if not parts:
        return "::"
    return "::" + "::".join(parts)
