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
# Every declared property name for a given header type, indexed by
# ``(module, object_type)``.  The parser uses this catalogue as the
# "known sibling keys" set when re-splitting compact one-line stanza
# bodies: ``net route { network 10.0.0.0/8 gw 192.168.1.1 }`` needs to
# know that ``gw`` is a sibling key before it can pull it apart from
# the ``network`` value.  One canonical source of truth replaces
# previously-hardcoded per-parser key lists.
PROPERTY_NAMES_BY_TYPE: dict[tuple[str, str], tuple[str, ...]] = {}
PROPERTY_SPECS_BY_TYPE: dict[
    tuple[str, str],
    dict[str, BigipPropertySpec],
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
        # Aggregate every property name (referenced or not) so the
        # parser can disambiguate inline siblings against the
        # registry rather than against a hand-maintained per-parser
        # list.
        type_key = (module, object_type)
        if type_key not in PROPERTY_NAMES_BY_TYPE:
            PROPERTY_NAMES_BY_TYPE[type_key] = ()
        PROPERTY_SPECS_BY_TYPE.setdefault(type_key, {})
        existing = list(PROPERTY_NAMES_BY_TYPE[type_key])
        for prop in object_spec.properties:
            if prop.name not in existing:
                existing.append(prop.name)
            PROPERTY_SPECS_BY_TYPE[type_key].setdefault(prop.name, prop)
        PROPERTY_NAMES_BY_TYPE[type_key] = tuple(existing)


def property_names_for(module: str, object_type: str) -> tuple[str, ...]:
    """Return every declared property name for ``<module> <object_type>``.

    Returns an empty tuple for unknown header types.  Used by the
    block-body parser to disambiguate compact one-line stanzas — it
    treats anything in the returned set as a sibling-key boundary
    rather than a continuation of the previous value.
    """
    return PROPERTY_NAMES_BY_TYPE.get((module, object_type), ())


def property_spec_for(
    module: str, object_type: str, property_name: str
) -> BigipPropertySpec | None:
    """Look up one property's spec, or ``None`` when unknown.

    Centralised so callers don't have to know the indexing key shape
    (``(module, object_type) -> dict[name, spec]``).  Used by the
    tmsh emitter to decide whether a property is list-valued and
    which operator keywords it accepts.
    """
    return PROPERTY_SPECS_BY_TYPE.get((module, object_type), {}).get(property_name)


def list_operator_for(
    module: str,
    object_type: str,
    property_name: str,
    *,
    prefer: tuple[str, ...] = ("replace-all-with",),
) -> str:
    """Return the best tmsh operator to use for a full-body write.

    Looks up the property's ``list_operators`` set and picks the
    first operator from *prefer* that the property supports.  Returns
    an empty string when the property is scalar (no operator needed)
    or when none of the preferred operators is supported (caller
    should fall back to a bare ``<prop> { ... }``).

    The default *prefer* picks ``replace-all-with`` first because
    that's the right operator for "write the whole list from
    scratch" — every list property except ``metadata`` (which uses
    ``add``/``delete``/``modify`` only) supports it.  ``metadata``
    callers should pass ``prefer=("modify", "add")`` so they get a
    usable operator anyway.
    """
    spec = property_spec_for(module, object_type, property_name)
    if spec is None or not spec.list_operators:
        return ""
    for op in prefer:
        if op in spec.list_operators:
            return op
    return ""
