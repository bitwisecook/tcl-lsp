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

    def matches_section(self, section: str | None) -> bool:
        if not self.in_sections:
            return True
        return (section or "") in self.in_sections
