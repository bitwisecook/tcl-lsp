"""Generic iRules ``static::`` variable-name detection.

Pattern compilation/checking for "generic" static variable names (e.g.
``static::debug``, ``static::timeout``).  Lives in ``compiler/`` so both
the compiler iRules-flow pass (``compiler/irules_flow.py``) and the
analyser iRules checks (``analyser/irules_checks.py``) can consume it
without the analyser-depends-from-compiler layering inversion.
"""

from __future__ import annotations

import re

DEFAULT_GENERIC_VARIABLE_PATTERNS: tuple[str, ...] = (
    # Debug / logging
    r"^debug(_level|_enabled)?$",
    r"^dbg$",
    r"^log_(level|server|enabled)$",
    r"^logging$",
    r"^verbose$",
    r"^trace$",
    # Configuration
    r"^(response_)?timeout$",
    r"^(max_)?retr(y|ies)$",
    r"^config$",
    r"^(enabled|disabled|active)$",
    r"^mode$",
    r"^(port|host|server|pool)$",
    # Counters / limits
    r"^count(er)?$",
    r"^(limit|max_connections|threshold|rate|interval)$",
    # Generic state
    r"^(flag|level|status|state|version|name|value|data|result|test|init|default)$",
)

# Compiled patterns (lazily rebuilt when the user changes configuration).
_compiled_generic_patterns: list[re.Pattern[str]] | None = None
_raw_generic_patterns: tuple[str, ...] | None = None


def get_generic_patterns(
    patterns: list[str] | None = None,
) -> list[re.Pattern[str]]:
    """Return compiled regex patterns for generic variable name detection.

    Uses a module-level cache; recompiles when *patterns* changes.
    """
    global _compiled_generic_patterns, _raw_generic_patterns
    if patterns is None:
        patterns = list(DEFAULT_GENERIC_VARIABLE_PATTERNS)
    key = tuple(patterns)
    if _compiled_generic_patterns is not None and _raw_generic_patterns == key:
        return _compiled_generic_patterns
    _raw_generic_patterns = key
    _compiled_generic_patterns = []
    for pat in patterns:
        try:
            _compiled_generic_patterns.append(re.compile(pat, re.IGNORECASE))
        except re.error:
            pass  # skip invalid patterns from user config
    return _compiled_generic_patterns


def is_generic_static_name(
    var_name: str,
    patterns: list[str] | None = None,
) -> bool:
    """Return True if *var_name* (including ``static::`` prefix) is generic."""
    bare = var_name.removeprefix("static::").lower()
    for pat in get_generic_patterns(patterns):
        if pat.search(bare):
            return True
    return False
