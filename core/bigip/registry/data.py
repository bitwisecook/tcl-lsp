"""Curated BIG-IP object registry catalogue assembled from object files."""

from __future__ import annotations

from .models import BigipObjectKindSpec, BigipPropertySpec
from .specs import OBJECT_SPECS

KIND_SPECS: tuple[BigipObjectKindSpec, ...] = tuple(spec.kind_spec for spec in OBJECT_SPECS)
OBJECT_KIND_SPECS: dict[str, BigipObjectKindSpec] = {spec.kind: spec for spec in KIND_SPECS}

HEADER_KIND_MAP: dict[tuple[str, str], str] = {}
PROPERTY_REFERENCE_SPECS: dict[
    tuple[str, str],
    dict[str, tuple[BigipPropertySpec, ...]],
] = {}

for object_spec in OBJECT_SPECS:
    kind = object_spec.kind_spec.kind
    for module, object_type in object_spec.header_types:
        HEADER_KIND_MAP[(module, object_type)] = kind
        by_name = PROPERTY_REFERENCE_SPECS.setdefault((module, object_type), {})
        for prop in object_spec.properties:
            if not prop.references:
                continue
            by_name.setdefault(prop.name, ())
            by_name[prop.name] = by_name[prop.name] + (prop,)
