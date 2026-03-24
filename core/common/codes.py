"""Self-registering diagnostic and optimisation code registry.

Each diagnostic or optimisation code is registered at import time via
:func:`diag` or :func:`opt`.  The registry is the single source of truth
for all code metadata — descriptions, sections, defaults — and is queried
by the build script, tests, and the LSP server.

Usage in check modules::

    from core.common.codes import diag
    W100 = diag("W100", "Unbraced expr body", section="warning")

``diag()`` returns the code string (``"W100"``), so call sites are
unchanged: ``Diagnostic(..., code=W100, ...)``.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


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
) -> str:
    """Register a diagnostic code and return its string value.

    Raises :class:`ValueError` if the code is already registered or the
    section is not in :data:`SECTIONS`.
    """
    if code in _registry:
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
    return code


def opt(
    code: str,
    description: str,
    *,
    default: bool = True,
) -> str:
    """Register an optimisation code and return its string value."""
    if code in _registry:
        raise ValueError(f"Duplicate optimisation code: {code}")
    _registry[code] = CodeInfo(
        code=code,
        description=description,
        kind=CodeKind.OPTIMISATION,
        default=default,
    )
    return code


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
