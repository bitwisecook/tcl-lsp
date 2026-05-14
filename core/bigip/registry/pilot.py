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
from .value_specs import (
    DestinationSpec,
    ListSpec,
    MonitorExpressionSpec,
    ObjectRefSpec,
    PersistenceAttachmentSpec,
    ProfileAttachmentSpec,
    SnatModeSpec,
)

# All "ltm monitor *" kinds a pool / node / GTM-pool / GTM-server
# monitor can point at.  Used by MonitorExpressionSpec so the graph
# layer surfaces the right target kinds when walking pool monitor
# references.  Kept in one place so adding a new monitor type only
# touches this list — every property that references a monitor
# inherits the broader set.
_MONITOR_REF_KINDS = (
    "ltm monitor",
    "ltm monitor diameter",
    "ltm monitor dns",
    "ltm monitor external",
    "ltm monitor firepass",
    "ltm monitor ftp",
    "ltm monitor gateway-icmp",
    "ltm monitor http",
    "ltm monitor http2",
    "ltm monitor https",
    "ltm monitor icmp",
    "ltm monitor imap",
    "ltm monitor inband",
    "ltm monitor ldap",
    "ltm monitor module-score",
    "ltm monitor mqtt",
    "ltm monitor mssql",
    "ltm monitor mysql",
    "ltm monitor nntp",
    "ltm monitor oracle",
    "ltm monitor pop3",
    "ltm monitor postgresql",
    "ltm monitor radius",
    "ltm monitor radius-accounting",
    "ltm monitor real-server",
    "ltm monitor rpc",
    "ltm monitor sasp",
    "ltm monitor scripted",
    "ltm monitor sip",
    "ltm monitor smb",
    "ltm monitor smtp",
    "ltm monitor snmp-dca",
    "ltm monitor snmp-dca-base",
    "ltm monitor soap",
    "ltm monitor tcp",
    "ltm monitor tcp-echo",
    "ltm monitor tcp-half-open",
    "ltm monitor udp",
    "ltm monitor virtual-location",
    "ltm monitor wap",
    "ltm monitor wmi",
)


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

# Phase 6 migrations: monitor expressions across every kind whose
# ``monitor`` property accepts the expression grammar.  Pool / node /
# GTM server / GTM pool all route through the same spec.  The
# projection still returns the canonical string (back-compat), but
# the reference layer now enumerates every monitor reference via
# ``MonitorExpression.references`` so ``references_to /Common/http``
# finds every pool / node / GTM object that uses the monitor —
# exact-path matching that the legacy grep substring seeding could
# only approximate.
_MONITOR_PROPERTY = PropertySpec(
    attr="monitor",
    value=MonitorExpressionSpec(ref_kinds=_MONITOR_REF_KINDS),
    writable=True,
    # The legacy projection wraps ``monitor`` as a ``PathRef`` for
    # ref_kind="ltm monitor" so ``.monitor.full-path`` works.  Stay
    # on that surface until the projection layer can route
    # MonitorExpression through a structured container that still
    # quacks like a PathRef for single-monitor expressions.  The
    # spec owns parse / edit / references which is what the
    # migration was for.
    project_via_legacy=True,
)

# Phase 6 migration: source-address-translation as the SNAT mode sum
# type.  The legacy representation kept the body as a string; the
# new spec parses it into a structured value so queries can filter
# by mode (``select(.snat_mode.is_automap)``) and ``references_to
# /Common/snatpool_x`` finds every virtual that uses it.
_PILOT_LTM_VIRTUAL_SNAT = PropertySpec(
    attr="source_address_translation",
    value=SnatModeSpec(),
    writable=True,
    tmsh_name="source-address-translation",
)


# Phase 6 migrations: list-valued attachment properties on
# ``ltm virtual``.  Each list element is a keyed-block attachment
# (``profiles { /Common/clientssl { context clientside } }`` /
# ``persist { /Common/cookie { default yes } }``) and the
# references dispatch unwinds the list to yield one Reference per
# element.  Projection stays on the legacy ``list_ref`` PathRef
# tuple surface so existing queries that walk ``.profiles[]`` keep
# working unchanged.
_PILOT_LTM_VIRTUAL_PROFILES = PropertySpec(
    attr="profiles",
    value=ListSpec(item=ProfileAttachmentSpec()),
    writable=True,
    project_via_legacy=True,
)
_PILOT_LTM_VIRTUAL_PERSIST = PropertySpec(
    attr="persist",
    value=ListSpec(item=PersistenceAttachmentSpec()),
    writable=True,
    project_via_legacy=True,
)
# ``rules`` is a plain ref-list (no per-attachment metadata) so the
# inner spec is the simpler ObjectRefSpec.  Migrating it here lets
# the reference dispatch hand the graph the same edges the legacy
# ``ref_kind`` + ``list_ref=True`` projection produces, but via the
# value-spec surface that downstream LSP / docs can also consume.
_PILOT_LTM_VIRTUAL_RULES = PropertySpec(
    attr="rules",
    value=ListSpec(item=ObjectRefSpec(kind="ltm rule")),
    writable=True,
    project_via_legacy=True,
)
_PILOT_LTM_VIRTUAL_POLICIES = PropertySpec(
    attr="policies",
    value=ListSpec(item=ObjectRefSpec(kind="ltm policy")),
    writable=True,
    project_via_legacy=True,
)
_PILOT_LTM_VIRTUAL_VLANS = PropertySpec(
    attr="vlans",
    value=ListSpec(item=ObjectRefSpec(kind="net vlan")),
    writable=True,
    project_via_legacy=True,
)


# (module, object_type, property_name) -> PropertySpec
PILOT_PROPERTY_SPECS: dict[tuple[str, str, str], PropertySpec] = {
    ("ltm", "virtual", "destination"): _PILOT_LTM_VIRTUAL_DESTINATION,
    # ``monitor`` migrations — same spec, four kinds.
    ("ltm", "pool", "monitor"): _MONITOR_PROPERTY,
    ("ltm", "node", "monitor"): _MONITOR_PROPERTY,
    ("gtm", "pool", "monitor"): _MONITOR_PROPERTY,
    ("gtm", "server", "monitor"): _MONITOR_PROPERTY,
    ("ltm", "virtual", "source-address-translation"): _PILOT_LTM_VIRTUAL_SNAT,
    # List-valued attachments / refs on ltm virtual.
    ("ltm", "virtual", "profiles"): _PILOT_LTM_VIRTUAL_PROFILES,
    ("ltm", "virtual", "persist"): _PILOT_LTM_VIRTUAL_PERSIST,
    ("ltm", "virtual", "rules"): _PILOT_LTM_VIRTUAL_RULES,
    ("ltm", "virtual", "policies"): _PILOT_LTM_VIRTUAL_POLICIES,
    ("ltm", "virtual", "vlans"): _PILOT_LTM_VIRTUAL_VLANS,
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
