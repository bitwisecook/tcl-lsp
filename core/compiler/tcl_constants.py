"""Shared Tcl language constants.

Canonical definitions for boolean literals and their prefix forms
as accepted by ``Tcl_GetBoolean``.
"""

from __future__ import annotations

# Full-word boolean literals (for exact matching in non-expr contexts).
TCL_BOOL_TRUE = frozenset({"true", "yes", "on"})
TCL_BOOL_FALSE = frozenset({"false", "no", "off"})
TCL_BOOL_LITERALS = TCL_BOOL_TRUE | TCL_BOOL_FALSE

# Tcl accepts unique prefixes of boolean words (Tcl_GetBoolean).
# See Tcl 9.0 test suite: ``string is true TrU`` → 1, ``string is true ye`` → 1.
TCL_BOOL_TRUE_PREFIXES = frozenset({"t", "tr", "tru", "true", "y", "ye", "yes", "on"})
TCL_BOOL_FALSE_PREFIXES = frozenset({"f", "fa", "fal", "fals", "false", "n", "no", "of", "off"})
TCL_BOOL_ALL_PREFIXES = TCL_BOOL_TRUE_PREFIXES | TCL_BOOL_FALSE_PREFIXES
