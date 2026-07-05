# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Canonical dialect constants shared across the registry package."""

from __future__ import annotations

from functools import lru_cache

KNOWN_DIALECTS: frozenset[str] = frozenset(
    (
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
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


# F5 dialects whose ensemble commands are fixed — no user-added
# subcommands — so the minifier can abbreviate ensemble subcommands to
# their minimum unambiguous prefix.
FIXED_ENSEMBLE_DIALECTS: frozenset[str] = frozenset({"f5-irules", "f5-iapps", "f5-bigip"})


def has_fixed_ensembles(dialect: str | None) -> bool:
    """Return True if *dialect* guarantees fixed (non-extensible) ensembles."""
    return dialect in FIXED_ENSEMBLE_DIALECTS


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
    "tcl9.1": "tcl9.1",
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
    "tcl9.1": 4,
}


# Dialect inheritance — a dialect that is a strict superset of an earlier
# release inherits all of the earlier dialect's command, subcommand, and
# option membership.  Listing the parent here means a spec tagged only with
# the parent (e.g. ``tcl9.0``) is automatically available under the child
# (``tcl9.1``) without re-tagging hundreds of command specs.  The relation
# is one-directional: ``tcl9.0`` never inherits ``tcl9.1`` additions.
DIALECT_INHERITS: dict[str, str] = {
    "tcl9.1": "tcl9.0",
}


@lru_cache(maxsize=None)
def dialect_membership(dialect: str) -> frozenset[str]:
    """Return the dialect tags that *dialect* satisfies for spec membership.

    For a plain dialect this is just ``{dialect}``.  For an inheriting
    dialect (see :data:`DIALECT_INHERITS`) it also includes every ancestor
    it is a superset of, so a spec tagged only with an ancestor
    (``{"tcl9.0"}``) still matches the descendant (``tcl9.1``).
    """
    seen = {dialect}
    cur = dialect
    while cur in DIALECT_INHERITS:
        cur = DIALECT_INHERITS[cur]
        seen.add(cur)
    return frozenset(seen)


def dialect_supports(dialect: str | None, dialects: frozenset[str] | None) -> bool:
    """Return True when *dialect* is a member of *dialects* (with inheritance).

    ``dialect is None`` (dialect-agnostic query) and ``dialects is None``
    (command available everywhere) both match.  Otherwise the query
    dialect's inherited membership set is intersected with *dialects* so an
    inheriting dialect picks up its ancestors' tags.
    """
    if dialect is None or dialects is None:
        return True
    return bool(dialects & dialect_membership(dialect))


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
