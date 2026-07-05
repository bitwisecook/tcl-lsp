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

"""Shared compiler analysis types — lifted from :mod:`compiler.core_analyses`.

Extracted to break the ``compilation_unit`` ↔ ``core_analyses`` ↔
``interprocedural`` import cycle.  Only data-class and enum definitions
that have *no* internal compiler dependencies live here; the analysis
functions stay in ``core_analyses``.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto


class LatticeKind(Enum):
    UNKNOWN = auto()
    CONST = auto()
    CONSTSET = auto()
    OVERDEFINED = auto()


# Maximum number of elements in a CONSTSET before we widen to OVERDEFINED.
_MAX_CONSTSET_SIZE = 32


@dataclass(frozen=True, slots=True)
class LatticeValue:
    kind: LatticeKind
    value: int | float | bool | str | None = None
    # For CONSTSET: the finite set of possible constant values.
    values: frozenset[int | float | bool | str] | None = None

    @staticmethod
    def unknown() -> "LatticeValue":
        return LatticeValue(LatticeKind.UNKNOWN, None)

    @staticmethod
    def overdefined() -> "LatticeValue":
        return LatticeValue(LatticeKind.OVERDEFINED, None)

    @staticmethod
    def const(value: int | float | bool | str) -> "LatticeValue":
        return LatticeValue(LatticeKind.CONST, value)

    @staticmethod
    def constset(vals: frozenset[int | float | bool | str]) -> "LatticeValue":
        """Create a CONSTSET lattice value from a finite set of constants.

        If the set has exactly one element, returns a CONST instead.
        If the set exceeds ``_MAX_CONSTSET_SIZE``, returns OVERDEFINED.
        """
        if len(vals) == 0:
            return OVERDEFINED
        if len(vals) == 1:
            return LatticeValue.const(next(iter(vals)))
        if len(vals) > _MAX_CONSTSET_SIZE:
            return OVERDEFINED
        return LatticeValue(LatticeKind.CONSTSET, None, vals)


UNKNOWN = LatticeValue.unknown()
OVERDEFINED = LatticeValue.overdefined()


def _to_set(lv: LatticeValue) -> frozenset[int | float | bool | str] | None:
    """Extract the set of possible values from a CONST or CONSTSET."""
    if lv.kind is LatticeKind.CONST and lv.value is not None:
        return frozenset((lv.value,))
    if lv.kind is LatticeKind.CONSTSET and lv.values is not None:
        return lv.values
    return None


def _join(old: LatticeValue, new: LatticeValue) -> LatticeValue:
    if new.kind is LatticeKind.UNKNOWN:
        return old
    if old.kind is LatticeKind.UNKNOWN:
        return new
    if old.kind is LatticeKind.OVERDEFINED or new.kind is LatticeKind.OVERDEFINED:
        return OVERDEFINED
    # Both are CONST or CONSTSET — merge the value sets.
    old_set = _to_set(old)
    new_set = _to_set(new)
    if old_set is not None and new_set is not None:
        merged = old_set | new_set
        if merged == old_set:
            return old
        return LatticeValue.constset(merged)
    # Fallback (should not happen): widen.
    return OVERDEFINED
