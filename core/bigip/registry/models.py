"""Data models for BIG-IP object registry metadata."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class BigipObjectKindSpec:
    """Resolution metadata for one BIG-IP object kind."""

    kind: str
    table_name: str | None = None
    resolver_name: str | None = None
    module: str | None = None
    object_types: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class BigipObjectSpec:
    """Complete registry metadata owned by one BIG-IP object-kind module."""

    kind_spec: BigipObjectKindSpec
    header_types: tuple[tuple[str, str], ...] = ()
    properties: tuple["BigipPropertySpec", ...] = ()


@dataclass(frozen=True, slots=True)
class BigipPropertySpec:
    """Property metadata used for schema/validation aware tooling."""

    name: str
    value_type: str = "string"  # string|integer|float|boolean|enum|reference|list|unknown
    in_sections: tuple[str, ...] = ()
    required: bool = False
    repeated: bool = False
    allow_none: bool = False
    enum_values: tuple[str, ...] = ()
    min_value: float | None = None
    max_value: float | None = None
    pattern: str = ""
    references: tuple[str, ...] = ()
    description: str = ""
    # tmsh modify/create requires an explicit operator on list-valued
    # properties.  When set, ``list_operators`` enumerates the
    # operator keywords the property accepts — ``add`` / ``delete``
    # / ``modify`` / ``replace-all-with`` / ``none``.  The tmsh
    # renderer uses this to:
    #
    #  - emit ``<prop> replace-all-with { ... }`` (not bare
    #    ``<prop> { ... }``) for full-body writes;
    #  - choose granular ``add`` / ``delete`` / ``modify`` operators
    #    in the delta emitter when the property supports them.
    #
    # An empty set marks the property as scalar (no operator
    # required).  ``frozenset()`` is the safe default — overriding
    # per-property is a quiet declaration that the property is
    # list-valued.
    list_operators: frozenset[str] = frozenset()

    @property
    def is_list_valued(self) -> bool:
        """True when this property is a tmsh list (operator required)."""
        return bool(self.list_operators)

    def matches_section(self, section: str | None) -> bool:
        if not self.in_sections:
            return True
        return (section or "") in self.in_sections
