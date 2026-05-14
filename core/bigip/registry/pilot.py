"""Pilot migration table — properties expressed in the new shape.

This module hosts the *migrated* property specs.  Phase 2 onward,
each consumer (projection / edit planner / parser / graph) consults
the table here BEFORE falling back to the legacy
:class:`BigipPropertySpec` / :class:`FieldSpec` paths.  Properties
absent from this table keep the historical behaviour unchanged.

The table grows phase by phase:

- **Phase 2** seeds ``ltm virtual.destination`` so the projection
  layer exercises ``DestinationSpec.project()`` as a smoke test.
  Behaviour is identical to the legacy ``typed=True`` branch — a
  canonical-string projection — because Phase 2's deliverable is
  the dispatch, not the structured-child surface (which lands in
  Phase 6 alongside the rich compound specs).

- **Phase 3** adds writable properties so the edit planner can route
  through ``spec.value.render()`` instead of its ad-hoc encoder.

- **Phase 6** populates the rest, including the rich compound types
  (``MonitorExpressionSpec`` / ``ProfileAttachmentSpec`` / etc.).

The table key is the same tuple the registry already uses for its
``PROPERTY_SPECS_BY_TYPE`` lookup — ``(module, object_type,
property_name)`` — so the migration map cleanly aligns with the
existing data.
"""

from __future__ import annotations

from collections.abc import Iterable

from .properties import PropertySpec
from .value_specs import DestinationSpec

# Phase 2 seed: ltm virtual.destination flowing through DestinationSpec.
# Keeping the spec's parameters identical to what the doc's pilot
# example proposed — IPv4/IPv6, port required, route-domain allowed,
# no partition / folder prefix — so the existing tests pass without
# the spec adding any new constraints.
_PILOT_LTM_VIRTUAL_DESTINATION = PropertySpec(
    attr="destination",
    value=DestinationSpec(
        address_families=frozenset(("ipv4", "ipv6")),
        require_port=True,
        allow_route_domain=True,
        allow_partition=False,
        allow_folder=False,
        allow_wildcard=True,
    ),
    writable=True,
)


# (module, object_type, property_name) -> PropertySpec
PILOT_PROPERTY_SPECS: dict[tuple[str, str, str], PropertySpec] = {
    ("ltm", "virtual", "destination"): _PILOT_LTM_VIRTUAL_DESTINATION,
}


def pilot_property_spec_for(
    module: str, object_type: str, property_name: str
) -> PropertySpec | None:
    """Return the migrated :class:`PropertySpec` for one property.

    Returns ``None`` when the property hasn't been migrated yet —
    callers should fall back to the legacy registry lookup.  The
    fallback chain keeps the per-phase migration safe: a partial
    migration table still produces a correct (legacy-shaped)
    answer for every unmigrated property.
    """
    return PILOT_PROPERTY_SPECS.get((module, object_type, property_name))


def iter_pilot_property_specs() -> Iterable[tuple[tuple[str, str, str], PropertySpec]]:
    """Iterate every migrated property as ``((module, object_type,
    name), spec)`` so test parity checks and docs generators can
    walk the table without importing the dict directly."""
    return PILOT_PROPERTY_SPECS.items()


__all__ = [
    "PILOT_PROPERTY_SPECS",
    "iter_pilot_property_specs",
    "pilot_property_spec_for",
]
