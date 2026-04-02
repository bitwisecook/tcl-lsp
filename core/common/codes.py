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
    ai_category: str | None = None
    conversion_label: str | None = None
    opt_category: str | None = None


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

AI_CATEGORIES: frozenset[str] = frozenset(
    {
        "error",
        "security",
        "taint",
        "thread_safety",
        "control_flow",
        "performance",
        "style",
        "optimiser",
        "irules",
        "tk",
    }
)

OPT_CATEGORIES: frozenset[str] = frozenset(
    {
        "readability",
        "constant_folding",
        "pattern",
        "dce",
        "code_motion",
        "recursion",
    }
)


def diag(
    code: str,
    description: str,
    *,
    section: str,
    default: bool = True,
    internal: bool = False,
    ai_category: str | None = None,
    conversion_label: str | None = None,
) -> Callable[[_F], _F]:
    """Register a diagnostic code.  Use as ``@diag(...)`` decorator or bare call.

    Raises :class:`ValueError` if the code is already registered or the
    section is not in :data:`SECTIONS`.
    """
    if code in _registry:
        existing = _registry[code]
        if (
            existing.description == description
            and existing.default == default
            and existing.internal == internal
            and existing.ai_category == ai_category
            and existing.conversion_label == conversion_label
        ):

            def _identity_dup(fn: _F) -> _F:
                return fn

            return _identity_dup
        raise ValueError(f"Duplicate diagnostic code: {code}")
    if section not in SECTION_KEYS:
        raise ValueError(
            f"Unknown section {section!r} for code {code}. "
            f"Add it to SECTIONS in core/common/codes.py."
        )
    if ai_category is not None and ai_category not in AI_CATEGORIES:
        raise ValueError(
            f"Unknown ai_category {ai_category!r} for code {code}. "
            f"Valid values: {', '.join(sorted(AI_CATEGORIES))}."
        )
    if conversion_label is not None and not conversion_label.strip():
        raise ValueError(f"conversion_label must be non-empty for code {code}.")
    _registry[code] = CodeInfo(
        code=code,
        description=description,
        kind=CodeKind.DIAGNOSTIC,
        section=section,
        default=default,
        internal=internal,
        ai_category=ai_category,
        conversion_label=conversion_label,
    )

    def _identity(fn: _F) -> _F:
        return fn

    return _identity


def opt(
    code: str,
    description: str,
    *,
    default: bool = True,
    opt_category: str,
) -> Callable[[_F], _F]:
    """Register an optimisation code.  Use as ``@opt(...)`` decorator or bare call."""
    if code in _registry:
        existing = _registry[code]
        if (
            existing.description == description
            and existing.default == default
            and existing.opt_category == opt_category
        ):

            def _identity_dup(fn: _F) -> _F:
                return fn

            return _identity_dup
        raise ValueError(f"Duplicate optimisation code: {code}")
    if opt_category not in OPT_CATEGORIES:
        raise ValueError(
            f"Unknown opt_category {opt_category!r} for code {code}. "
            f"Valid values: {', '.join(sorted(OPT_CATEGORIES))}."
        )
    _registry[code] = CodeInfo(
        code=code,
        description=description,
        kind=CodeKind.OPTIMISATION,
        default=default,
        opt_category=opt_category,
    )

    def _identity(fn: _F) -> _F:
        return fn

    return _identity


# Query functions


def all_codes() -> dict[str, CodeInfo]:
    """Return the full registry as a dict (read-only snapshot)."""
    return dict(_registry)


def diagnostic_codes() -> frozenset[str]:
    """All registered user-configurable diagnostic codes."""
    return frozenset(
        c for c, info in _registry.items() if info.kind is CodeKind.DIAGNOSTIC and not info.internal
    )


def default_disabled_diagnostics() -> frozenset[str]:
    """Diagnostic codes that are opt-in (``default=False``)."""
    return frozenset(
        c
        for c, info in _registry.items()
        if info.kind is CodeKind.DIAGNOSTIC and not info.internal and not info.default
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


def optimisation_codes_by_category() -> dict[str, frozenset[str]]:
    """Return optimisation codes grouped by their ``opt_category`` metadata."""
    groups: dict[str, set[str]] = {key: set() for key in OPT_CATEGORIES}
    for info in _registry.values():
        if info.kind is not CodeKind.OPTIMISATION or info.opt_category is None:
            continue
        groups.setdefault(info.opt_category, set()).add(info.code)
    return {key: frozenset(sorted(groups.get(key, set()))) for key in sorted(OPT_CATEGORIES)}


def ai_category_overrides() -> dict[str, str]:
    """Return per-code AI category overrides declared on diagnostics."""
    return {
        info.code: info.ai_category
        for info in _registry.values()
        if info.kind is CodeKind.DIAGNOSTIC and info.ai_category is not None
    }


def conversion_map() -> dict[str, str]:
    """Return ``{code: conversion_label}`` for diagnostics with conversion metadata."""
    ordered = sorted(
        (
            info
            for info in _registry.values()
            if info.kind is CodeKind.DIAGNOSTIC and info.conversion_label is not None
        ),
        key=lambda i: (SECTION_KEYS.index(i.section) if i.section in SECTION_KEYS else 999, i.code),
    )
    return {info.code: info.conversion_label or "" for info in ordered}
