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
    CertKeyChainSpec,
    DataGroupRecordSpec,
    DestinationSpec,
    FirewallRuleSpec,
    GtmRegionMemberSpec,
    ListSpec,
    LtmPolicyRuleSpec,
    MonitorExpressionSpec,
    ObjectRefSpec,
    PersistenceAttachmentSpec,
    ProfileAttachmentSpec,
    SnatModeSpec,
)

# LTM monitor kinds a pool / node monitor expression can point at.
# ``"ltm monitor"`` (the family-prefix) is intentionally first so
# ``MonitorExpressionSpec.references()`` attributes refs to the family
# and ``candidate_registry_kinds_for_display`` fans the family out to
# every specific kind at resolution time.
_LTM_MONITOR_REF_KINDS = (
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

# GTM monitor kinds a gtm pool / gtm server monitor expression can
# point at.  Distinct from the LTM family so GTM references resolve
# to ``gtm_monitor_*`` rather than fanning out to LTM kinds.  Same
# family-first convention as :data:`_LTM_MONITOR_REF_KINDS`.
_GTM_MONITOR_REF_KINDS = (
    "gtm monitor",
    "gtm monitor bigip",
    "gtm monitor bigip-link",
    "gtm monitor external",
    "gtm monitor firepass",
    "gtm monitor ftp",
    "gtm monitor gateway-icmp",
    "gtm monitor gtp",
    "gtm monitor http",
    "gtm monitor https",
    "gtm monitor imap",
    "gtm monitor ldap",
    "gtm monitor mssql",
    "gtm monitor mysql",
    "gtm monitor nntp",
    "gtm monitor none",
    "gtm monitor oracle",
    "gtm monitor pop3",
    "gtm monitor postgresql",
    "gtm monitor radius",
    "gtm monitor radius-accounting",
    "gtm monitor real-server",
    "gtm monitor scripted",
    "gtm monitor sip",
    "gtm monitor smtp",
    "gtm monitor snmp",
    "gtm monitor snmp-link",
    "gtm monitor soap",
    "gtm monitor tcp",
    "gtm monitor tcp-half-open",
    "gtm monitor udp",
    "gtm monitor wap",
    "gtm monitor wmi",
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
_LTM_MONITOR_PROPERTY = PropertySpec(
    attr="monitor",
    value=MonitorExpressionSpec(ref_kinds=_LTM_MONITOR_REF_KINDS),
    writable=True,
    # Project the typed :class:`MonitorExpression` directly.  The
    # type carries ``.full_path`` / ``.name`` properties so the
    # historical ``.monitor.full-path`` PathRef-style queries keep
    # working, plus structured fields (``.mode`` / ``.monitors[]`` /
    # ``.minimum``) the new design surfaces for richer DSL access.
)

# GTM pool / GTM server monitor — same spec, GTM target kinds so
# refs resolve into ``gtm_monitor_*`` rather than the LTM family.
_GTM_MONITOR_PROPERTY = PropertySpec(
    attr="monitor",
    value=MonitorExpressionSpec(ref_kinds=_GTM_MONITOR_REF_KINDS),
    writable=True,
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
# element.  Projection is the typed :class:`BigipList`-of-
# ``ProfileAttachment`` (or ``PersistenceAttachment``) view so DSL
# queries can ask ``.profiles[] | select(.context == "clientside")``
# or ``.persist[] | select(.default) | .name`` directly; the typed
# values expose ``.full_path`` aliases for back-compat with the
# legacy PathRef contract (``.profiles[].full-path``).
_PILOT_LTM_VIRTUAL_PROFILES = PropertySpec(
    attr="profiles",  # typed BigipList stored directly on the model
    value=ListSpec(item=ProfileAttachmentSpec(), syntax="keyed-block"),
    writable=True,
)
_PILOT_LTM_VIRTUAL_PERSIST = PropertySpec(
    attr="persist",
    value=ListSpec(item=PersistenceAttachmentSpec(), syntax="keyed-block"),
    writable=True,
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

# ``ltm data-group internal.records`` — a keyed-block list where
# each record carries a key (the lookup name) + optional ``data``
# value.  Model stores tuple[str, ...] of keys; the spec parses
# each into a typed record so the references dispatch can surface
# any record-level edges (today none — records don't reference
# other objects — but the spec is in place when future record
# types do).
_PILOT_LTM_DATA_GROUP_RECORDS = PropertySpec(
    attr="records",
    value=ListSpec(item=DataGroupRecordSpec()),
    writable=True,
    project_via_legacy=True,
)

# ``gtm region.region-members`` — a list of topology rows.
_PILOT_GTM_REGION_MEMBERS = PropertySpec(
    attr="region_members",
    value=ListSpec(item=GtmRegionMemberSpec()),
    writable=True,
    tmsh_name="region-members",
    project_via_legacy=True,
)

# Client/server-SSL ``cert-key-chain`` — keyed-block list with up
# to three sub-references per entry (cert, key, chain).  The
# references dispatch surfaces every SSL artifact the profile
# depends on, addressing the design doc's "find every profile
# that references a given cert" goal.
_PILOT_LTM_CLIENT_SSL_CERT_KEY_CHAIN = PropertySpec(
    attr="cert_key_chain",
    value=ListSpec(item=CertKeyChainSpec()),
    writable=True,
    tmsh_name="cert-key-chain",
    project_via_legacy=True,
)

# Security firewall rule-lists carry ``rules`` as a keyed-block
# list of typed :class:`dialects.f5.bigip.types.FirewallRule` bodies; the
# reference dispatch walks each rule's source / destination
# endpoints and surfaces every address-list / port-list / nested
# rule-list edge.  The model stores the typed bodies under
# ``rule_objects`` (the legacy ``rules`` tuple of names is kept for
# back-compat projection).
_PILOT_SECURITY_FW_RULE_LIST_RULES = PropertySpec(
    attr="rule_objects",
    value=ListSpec(item=FirewallRuleSpec()),
    writable=False,  # rule bodies are keyed children of the list
    tmsh_name="rules",
    project_via_legacy=True,
)
_PILOT_SECURITY_FW_POLICY_RULE_LISTS = PropertySpec(
    attr="rule_lists",
    value=ListSpec(item=ObjectRefSpec(kind="security firewall rule-list")),
    writable=True,
    tmsh_name="rule-lists",
    project_via_legacy=True,
)
_PILOT_SECURITY_FW_ADDRESS_LIST_NESTED = PropertySpec(
    attr="address_lists",
    value=ListSpec(item=ObjectRefSpec(kind="security firewall address-list")),
    writable=True,
    tmsh_name="address-lists",
    project_via_legacy=True,
)

# ``ltm policy.rules`` — each rule has its own typed body
# (BigipPolicyRule with actions[] / conditions[]).  The
# LtmPolicyRuleSpec walks the nested actions / conditions and
# yields every reference each carries — today that's pool refs
# from ``forward select pool <path>`` actions.  Projection stays
# on the legacy tuple surface because the rules are objects, not
# scalar refs.
_PILOT_LTM_POLICY_RULES = PropertySpec(
    attr="rules",
    value=ListSpec(item=LtmPolicyRuleSpec()),
    writable=True,
    project_via_legacy=True,
)


# (module, object_type, property_name) -> PropertySpec
PILOT_PROPERTY_SPECS: dict[tuple[str, str, str], PropertySpec] = {
    ("ltm", "virtual", "destination"): _PILOT_LTM_VIRTUAL_DESTINATION,
    # ``monitor`` migrations — same spec, four kinds.
    ("ltm", "pool", "monitor"): _LTM_MONITOR_PROPERTY,
    ("ltm", "node", "monitor"): _LTM_MONITOR_PROPERTY,
    ("gtm", "pool", "monitor"): _GTM_MONITOR_PROPERTY,
    ("gtm", "server", "monitor"): _GTM_MONITOR_PROPERTY,
    ("ltm", "virtual", "source-address-translation"): _PILOT_LTM_VIRTUAL_SNAT,
    # List-valued attachments / refs on ltm virtual.
    ("ltm", "virtual", "profiles"): _PILOT_LTM_VIRTUAL_PROFILES,
    ("ltm", "virtual", "persist"): _PILOT_LTM_VIRTUAL_PERSIST,
    ("ltm", "virtual", "rules"): _PILOT_LTM_VIRTUAL_RULES,
    ("ltm", "virtual", "policies"): _PILOT_LTM_VIRTUAL_POLICIES,
    ("ltm", "virtual", "vlans"): _PILOT_LTM_VIRTUAL_VLANS,
    # Data-group records, GTM region rows, SSL cert chains.
    ("ltm", "data-group internal", "records"): _PILOT_LTM_DATA_GROUP_RECORDS,
    ("gtm", "region", "region-members"): _PILOT_GTM_REGION_MEMBERS,
    ("ltm", "profile client-ssl", "cert-key-chain"): _PILOT_LTM_CLIENT_SSL_CERT_KEY_CHAIN,
    ("ltm", "profile server-ssl", "cert-key-chain"): _PILOT_LTM_CLIENT_SSL_CERT_KEY_CHAIN,
    # ltm policy.rules — each entry walks its actions/conditions
    # via LtmPolicyRuleSpec, surfacing every pool ref the policy
    # forwards to.
    ("ltm", "policy", "rules"): _PILOT_LTM_POLICY_RULES,
    # Security firewall list-of-refs migrations.  Body-level rule
    # introspection waits for the model refactor that promotes
    # tuple-of-names to tuple-of-FirewallRule.
    ("security", "firewall rule-list", "rules"): _PILOT_SECURITY_FW_RULE_LIST_RULES,
    ("security", "firewall policy", "rule-lists"): _PILOT_SECURITY_FW_POLICY_RULE_LISTS,
    ("security", "firewall address-list", "address-lists"): _PILOT_SECURITY_FW_ADDRESS_LIST_NESTED,
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
