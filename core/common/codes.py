"""Self-registering diagnostic and optimisation code registry.

Each diagnostic or optimisation code is registered at import time via
:func:`diag` or :func:`opt`.  The registry is the single source of truth
for all code metadata — descriptions, sections, defaults — and is queried
by the build script, tests, and the LSP server.

Use as a decorator on the function or class that implements the check::

    from core.common.codes import diag

    @diag("W100", "Unbraced expr body", section="warning")
    def check_unbraced_expr(cmd_name, args, ...):
        ...

Multiple codes can be stacked on a single function::

    @diag("E002", "Too few arguments for command.", section="error")
    @diag("E003", "Too many arguments for command.", section="error")
    def _check_arg_count(...):
        ...

For codes without a single home function, a bare call registers the code::

    diag("S100", "Single shimmer outside a loop.", section="shimmer")
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from enum import Enum
from typing import TypeVar

_F = TypeVar("_F", bound=Callable)


class CodeKind(Enum):
    DIAGNOSTIC = "diagnostic"
    OPTIMISATION = "optimisation"


@dataclass(frozen=True, slots=True)
class CodeInfo:
    """Metadata for a registered diagnostic or optimisation code."""

    code: str
    description: str
    kind: CodeKind
    section: str = ""
    default: bool = True
    internal: bool = False


# Ordered list of valid diagnostic sections with display titles.  Controls
# output ordering in all generated files and validates that every registered
# code belongs to a known section.  Add new sections here when a new category
# is needed.  Multiple sections may share a title (they merge into one group
# in editor UIs).
SECTIONS: list[tuple[str, str]] = [
    ("error", "Diagnostics — Errors"),
    ("warning", "Diagnostics — Style & Best Practice"),
    ("variable", "Diagnostics — Variables"),
    ("security", "Diagnostics — Security"),
    ("hint", "Diagnostics — Hints"),
    ("shimmer", "Diagnostics — Shimmer"),
    ("taint", "Diagnostics — Taint"),
    ("irules", "Diagnostics — iRules"),
    ("irules_security", "Diagnostics — iRules"),
    ("irules_variable", "Diagnostics — iRules"),
]

# Derived helpers for fast lookup.
SECTION_KEYS: list[str] = [key for key, _ in SECTIONS]
SECTION_TITLES: dict[str, str] = dict(SECTIONS)

_registry: dict[str, CodeInfo] = {}


def diag(
    code: str,
    description: str,
    *,
    section: str,
    default: bool = True,
    internal: bool = False,
) -> Callable[[_F], _F]:
    """Register a diagnostic code.  Use as ``@diag(...)`` decorator or bare call.

    Raises :class:`ValueError` if the code is already registered or the
    section is not in :data:`SECTIONS`.
    """
    if code in _registry:
        existing = _registry[code]
        if (
            existing.description == description
            and existing.section == section
            and existing.default == default
            and existing.internal == internal
        ):
            # Idempotent re-registration (e.g. decorator on implementation
            # function when the code was already registered in codes_*.py).
            def _identity_dup(fn: _F) -> _F:
                return fn

            return _identity_dup
        raise ValueError(f"Duplicate diagnostic code: {code}")
    if section not in SECTION_KEYS:
        raise ValueError(
            f"Unknown section {section!r} for code {code}. "
            f"Add it to SECTIONS in core/common/codes.py."
        )
    _registry[code] = CodeInfo(
        code=code,
        description=description,
        kind=CodeKind.DIAGNOSTIC,
        section=section,
        default=default,
        internal=internal,
    )

    def _identity(fn: _F) -> _F:
        return fn

    return _identity


def opt(
    code: str,
    description: str,
    *,
    default: bool = True,
) -> Callable[[_F], _F]:
    """Register an optimisation code.  Use as ``@opt(...)`` decorator or bare call."""
    if code in _registry:
        existing = _registry[code]
        if existing.description == description and existing.default == default:
            def _identity_dup(fn: _F) -> _F:
                return fn

            return _identity_dup
        raise ValueError(f"Duplicate optimisation code: {code}")
    _registry[code] = CodeInfo(
        code=code,
        description=description,
        kind=CodeKind.OPTIMISATION,
        default=default,
    )

    def _identity(fn: _F) -> _F:
        return fn

    return _identity


# ---------------------------------------------------------------------------
# Query functions
# ---------------------------------------------------------------------------


def all_codes() -> dict[str, CodeInfo]:
    """Return the full registry as a dict (read-only snapshot)."""
    return dict(_registry)


def diagnostic_codes() -> frozenset[str]:
    """All registered user-configurable diagnostic codes."""
    return frozenset(
        c for c, info in _registry.items() if info.kind is CodeKind.DIAGNOSTIC and not info.internal
    )


def optimisation_codes() -> frozenset[str]:
    """All registered optimisation codes."""
    return frozenset(c for c, info in _registry.items() if info.kind is CodeKind.OPTIMISATION)


def internal_codes() -> frozenset[str]:
    """All registered internal (non-user-configurable) codes."""
    return frozenset(c for c, info in _registry.items() if info.internal)


def diagnostics_sorted() -> list[CodeInfo]:
    """User-configurable diagnostics sorted by code, grouped by section order."""
    section_idx = {s: i for i, s in enumerate(SECTION_KEYS)}
    return sorted(
        (
            info
            for info in _registry.values()
            if info.kind is CodeKind.DIAGNOSTIC and not info.internal
        ),
        key=lambda i: (section_idx.get(i.section, 999), i.code),
    )


def optimisations_sorted() -> list[CodeInfo]:
    """All optimisation codes sorted by code."""
    return sorted(
        (info for info in _registry.values() if info.kind is CodeKind.OPTIMISATION),
        key=lambda i: i.code,
    )


def codes_by_section() -> dict[str, list[CodeInfo]]:
    """User-configurable diagnostics grouped by section, in section order."""
    groups: dict[str, list[CodeInfo]] = {s: [] for s in SECTION_KEYS}
    for info in _registry.values():
        if info.kind is CodeKind.DIAGNOSTIC and not info.internal:
            groups.setdefault(info.section, []).append(info)
    # Sort codes within each section
    for codes in groups.values():
        codes.sort(key=lambda i: i.code)
    return groups
