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

"""Data models for BIG-IP object registry metadata."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class ValueKind(str, Enum):
    """The canonical property value kind vocabulary.

    Consumers can dispatch on members directly or compare
    against the string wire form — every member is also
    equal to its string value (string-typed Enum).
    """

    STRING = "string"
    INTEGER = "integer"
    FLOAT = "float"
    BOOLEAN = "boolean"
    ENUM = "enum"
    REFERENCE = "reference"
    LIST = "list"
    BLOCK = "block"
    UNKNOWN = "unknown"
    IP_ADDRESS = "ip-address"
    ENDPOINT = "endpoint"
    OBJECT = "object"


@dataclass(frozen=True, slots=True)
class BigipObjectKindSpec:
    """BigipObjectKindSpec."""

    # The registry's canonical kind name.
    kind: str
    # Optional :class:`BigipConfig` attribute name that stores this kind.
    table_name: str | None = None
    # Optional :class:`BigipConfig` method name used to resolve references.
    resolver_name: str | None = None
    # The tmsh module word (``ltm``, ``gtm``, …).
    module: str | None = None
    # tmsh object-type word(s) after the module — e.g. ``('profile tcp',)``.
    object_types: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class BigipPropertySpec:
    """BigipPropertySpec."""

    # Property identifier as it appears in tmsh.
    name: str
    # Collapsed value tag — one of :data:`ValueKind` members.
    value_type: str = "string"
    # Parent sections this property may appear in.
    in_sections: tuple[str, ...] = ()
    # tmsh requires this field at create time.
    required: bool = False
    # Property may appear multiple times.
    repeated: bool = False
    # The literal ``none`` clears the value.
    allow_none: bool = False
    # Permitted value-space members.
    enum_values: tuple[str, ...] = ()
    # Inclusive lower bound.
    min_value: float | None = None
    # Inclusive upper bound.
    max_value: float | None = None
    # Optional regex constraint.
    pattern: str = ""
    # Object kinds this property may name — outbound graph edges.
    references: tuple[str, ...] = ()
    # Human-readable description.
    description: str = ""
    # Richer shape kind when ``value_type`` collapses the scalar.
    shape_kind: str = ""
    # Documented default value.
    default: str | None = None
    # Lifecycle flags — ``deprecated``, ``read_only``, ``not_synced``, ….
    usage_flags: frozenset[str] = frozenset()
    # tmsh list operators (``add`` / ``delete`` / ``replace-all-with``).
    list_operators: frozenset[str] = frozenset()
    # Nested sub-properties for object-shaped blocks.
    block: tuple["BigipPropertySpec", ...] = ()

    @property
    def value_kind(self) -> ValueKind:
        """Typed accessor for :attr:`value_type`.

        Returns the matching :class:`ValueKind` member, or
        ``ValueKind.UNKNOWN`` when the stored string is outside the
        enum.  Consumers that need to dispatch on the property type
        should prefer this over the raw ``value_type`` string.
        """
        try:
            return ValueKind(self.value_type)
        except ValueError:
            return ValueKind.UNKNOWN

    @property
    def shape(self) -> ValueKind:
        """Typed accessor for :attr:`shape_kind` with fallback to
        :attr:`value_kind` when the JSON normaliser did not record a
        richer shape."""
        if self.shape_kind:
            try:
                return ValueKind(self.shape_kind)
            except ValueError:
                return self.value_kind
        return self.value_kind

    @property
    def is_list_valued(self) -> bool:
        """True when this property is a tmsh list (operator required)."""
        return bool(self.list_operators)

    @property
    def is_block(self) -> bool:
        """True when this property nests sub-properties as a block."""
        return bool(self.block)

    def matches_section(self, section: str | None) -> bool:
        if not self.in_sections:
            return True
        return (section or "") in self.in_sections


@dataclass(frozen=True, slots=True)
class BigipObjectSpec:
    """BigipObjectSpec."""

    # Identity record.
    kind_spec: BigipObjectKindSpec
    # (module, object-type) tuples the parser keys this kind by.
    header_types: tuple[tuple[str, str], ...] = ()
    # Property specs declared on this object kind.
    properties: tuple[BigipPropertySpec, ...] = ()
