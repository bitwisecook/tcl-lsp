"""Canonical dialect constants shared across the registry package."""

from __future__ import annotations

KNOWN_DIALECTS: frozenset[str] = frozenset(
    (
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
        "f5-bigip",
        "synopsys-eda-tcl",
        "cadence-eda-tcl",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "expect",
    )
)

# Positive dialect set for commands available everywhere except iRules.
DIALECTS_EXCEPT_IRULES: frozenset[str] = KNOWN_DIALECTS - frozenset({"f5-irules"})

# Canonical spelling plus the legacy alias for the F5 iRules dialect.
# ``f5-irules`` is the value carried by the active-dialect context var and
# every ``KNOWN_DIALECTS`` entry; ``irules`` is an older alias still passed
# by some call sites (e.g. side-effect lookups).
IRULES_DIALECT_NAMES: frozenset[str] = frozenset({"f5-irules", "irules"})


def is_irules_dialect(dialect: str | None) -> bool:
    """Return True if *dialect* names the F5 iRules dialect.

    Single source of truth for the iRules-dialect check: accepts the
    canonical ``f5-irules`` and the legacy ``irules`` alias.  Prefer this
    over inline ``dialect == "f5-irules"`` comparisons so the alias and
    any future spelling change live in one place.
    """
    return dialect in IRULES_DIALECT_NAMES


# Runtime Tcl version that each dialect is based on.  Used by
# ``dialects_since()`` to resolve version-dependent behaviour such as
# ``incr`` safely initialising an uninitialised variable (8.5+).
#
# Dialects not listed here (e.g. ``f5-bigip``) are excluded from
# version-based trait resolution — ``dialects_since()`` will never
# include them.
DIALECT_BASE_VERSION: dict[str, str] = {
    "tcl8.4": "tcl8.4",
    "tcl8.5": "tcl8.5",
    "tcl8.6": "tcl8.6",
    "tcl9.0": "tcl9.0",
    # iRules: TMOS embedded Tcl 8.4.6.
    "f5-irules": "tcl8.4",
    # iApps/tmsh: CentOS 7 system Tcl 8.5.13.
    "f5-iapps": "tcl8.5",
    "f5-tmsh": "tcl8.5",
    # f5-bigip: custom parser, not Tcl — intentionally omitted.
    # EDA vendor tools embed various Tcl versions.
    "synopsys-eda-tcl": "tcl8.6",
    "cadence-eda-tcl": "tcl8.6",
    "xilinx-eda-tcl": "tcl8.5",
    "intel-quartus-eda-tcl": "tcl8.5",
    "mentor-eda-tcl": "tcl8.5",
    "expect": "tcl8.6",
}

_TCL_VERSION_RANK: dict[str, int] = {
    "tcl8.4": 0,
    "tcl8.5": 1,
    "tcl8.6": 2,
    "tcl9.0": 3,
}


def dialects_since(min_version: str) -> frozenset[str]:
    """Return all dialects whose base Tcl version is >= *min_version*.

    Raises ``KeyError`` if *min_version* or any base version in
    ``DIALECT_BASE_VERSION`` is not in ``_TCL_VERSION_RANK``.

    >>> "f5-irules" in dialects_since("tcl8.5")
    False
    >>> "tcl8.6" in dialects_since("tcl8.5")
    True
    """
    min_rank = _TCL_VERSION_RANK[min_version]
    return frozenset(
        d for d, base in DIALECT_BASE_VERSION.items() if _TCL_VERSION_RANK[base] >= min_rank
    )
