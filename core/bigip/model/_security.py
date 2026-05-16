"""Typed projection for the security module (``security.*``).

AFM firewall (policies, rule-lists, port-lists, address-lists,
schedules, user-lists, global rules), NAT policies, log /
DoS profiles, IP-intelligence feed-lists and policies, security
zones, packet-filter, SSH/HTTP profiles, protocol-inspection
compliance, device-id attributes, and bot defense.
"""

from __future__ import annotations

from dataclasses import dataclass

from ...analysis.semantic_model import Range


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallPortList:
    """A ``security firewall port-list`` object."""

    name: str
    full_path: str
    ports: tuple[str, ...] = ()
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallRuleList:
    """A ``security firewall rule-list`` object.

    ``rules`` keeps the top-level rule names (back-compat with the
    historical projection); ``rule_objects`` is the typed per-rule
    view — :class:`core.bigip.types.FirewallRule` values carrying
    action / ip-protocol / source / destination clauses — so the
    registry's reference dispatch can enumerate the address-list /
    port-list / nested rule-list edges every rule references.
    """

    name: str
    full_path: str
    rules: tuple[str, ...] = ()
    rule_objects: tuple = ()  # tuple[FirewallRule, ...]
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallConfigEntityId:
    """A ``security firewall config-entity-id`` object."""

    name: str
    full_path: str
    entity_id: str = ""
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallPolicy:
    """A ``security firewall policy`` object — AFM policy carrying
    rule-list bindings.  Sister to ``ltm policy`` on the LTM side.

    ``rules`` surfaces the top-level keys of the ``rules { ... }``
    sub-block (one per rule binding) and ``rule_lists`` is the
    extracted PathRefs into ``security firewall rule-list``.
    """

    name: str
    full_path: str
    description: str = ""
    rules: tuple[str, ...] = ()
    rule_lists: tuple[str, ...] = ()  # PathRefs → security firewall rule-list
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallAddressList:
    """A ``security firewall address-list`` object — sister to
    ``firewall port-list`` but for IP addresses / CIDRs.

    ``addresses`` surfaces the bare address tokens from the
    ``addresses { ... }`` sub-block; ``address_lists`` carries
    PathRefs to nested child address-lists.
    """

    name: str
    full_path: str
    description: str = ""
    addresses: tuple[str, ...] = ()
    address_lists: tuple[str, ...] = ()  # PathRefs → security firewall address-list
    fqdns: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallGlobalRules:
    """The ``security firewall global-rules`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    rules: tuple[str, ...] = ()
    enforced_policy: str = ""  # PathRef → security firewall policy
    staged_policy: str = ""  # PathRef → security firewall policy
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallManagementIpRules:
    """The ``security firewall management-ip-rules`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    rules: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallSchedule:
    """A ``security firewall schedule`` object."""

    name: str
    full_path: str
    description: str = ""
    daily_hour_end: str = ""
    daily_hour_start: str = ""
    days_of_week: tuple[str, ...] = ()
    date_valid_end: str = ""
    date_valid_start: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallUserList:
    """A ``security firewall user-list`` object."""

    name: str
    full_path: str
    description: str = ""
    users: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallUserDomain:
    """A ``security firewall user-domain`` object."""

    name: str
    full_path: str
    description: str = ""
    domain: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallGlobalFqdnPolicy:
    """The ``security firewall global-fqdn-policy`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    context: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallPortMisusePolicy:
    """A ``security firewall port-misuse-policy`` object."""

    name: str
    full_path: str
    description: str = ""
    default_log: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallOnDemandCompilation:
    """The ``security firewall on-demand-compilation`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallOnDemandRuleDeploy:
    """The ``security firewall on-demand-rule-deploy`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallUuidDefaultAutogenerate:
    """The ``security firewall uuid-default-autogenerate`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    auto_generate_uuid: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallConfigChangeLog:
    """The ``security firewall config-change-log`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    log_publisher: str = ""
    range: Range | None = None


# Bundle 10a — high-value security.* kinds outside firewall.*:
# NAT policy + translations, log profile, DoS profile, IP
# intelligence feed-list / global-policy, zone / protected zone,
# packet-filter, SSH profile, HTTP profile, bot-defense profile.


@dataclass(frozen=True, slots=True)
class BigipSecurityNatPolicy:
    """A ``security nat policy`` object — NAT rule-list bindings.

    Sister to ``security firewall policy``: the ``rules`` sub-block
    is keyed by rule-binding name; each binding carries a
    ``rule-list /Common/...`` PathRef.
    """

    name: str
    full_path: str
    description: str = ""
    rules: tuple[str, ...] = ()
    rule_lists: tuple[str, ...] = ()  # PathRefs → security firewall rule-list
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityNatSourceTranslation:
    """A ``security nat source-translation`` object — NAT source pool."""

    name: str
    full_path: str
    description: str = ""
    type_: str = ""  # ``dynamic-pat`` / ``static-nat`` / ``napt`` / ``static-pat``
    addresses: tuple[str, ...] = ()
    ports: tuple[str, ...] = ()
    traffic_group: str = ""  # PathRef → cm traffic-group
    egress_interfaces_disabled: bool = False  # bare flag
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityNatDestinationTranslation:
    """A ``security nat destination-translation`` object."""

    name: str
    full_path: str
    description: str = ""
    type_: str = ""
    addresses: tuple[str, ...] = ()
    ports: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityLogProfile:
    """A ``security log profile`` object — AFM / ASM logging config."""

    name: str
    full_path: str
    description: str = ""
    application_data: str = ""
    network_data: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityDosProfile:
    """A ``security dos profile`` object — DDoS profile.

    The body is a deeply nested ``application`` / ``dos-network`` /
    ``protocol-dns`` / ``protocol-sip`` block; in v1 we surface
    only the identity scalars and let consumers reach further with
    ``--scf`` for the source view.
    """

    name: str
    full_path: str
    description: str = ""
    app_service: str = ""
    threshold_sensitivity: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityIpIntelligenceFeedList:
    """A ``security ip-intelligence feed-list`` object."""

    name: str
    full_path: str
    description: str = ""
    feeds: tuple[str, ...] = ()  # top-level keys of the ``feeds`` sub-block
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityIpIntelligenceGlobalPolicy:
    """The ``security ip-intelligence global-policy`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    log_blacklist_category: str = ""
    log_publisher: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityZone:
    """A ``security zone`` object — security-zone definition.

    Single-word kind (header ``security zone /Common/X``).  Lists
    the VLANs and tunnels in the zone.
    """

    name: str
    full_path: str
    description: str = ""
    vlans: tuple[str, ...] = ()  # PathRefs → net vlan
    tunnels: tuple[str, ...] = ()  # PathRefs → net tunnels tunnel
    interfaces: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityProtectedZone:
    """A ``security protected zone`` object."""

    name: str
    full_path: str
    description: str = ""
    enabled: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityPacketFilterPolicy:
    """A ``security packet-filter policy`` object."""

    name: str
    full_path: str
    description: str = ""
    rules: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityPacketFilterDefaultRules:
    """The ``security packet-filter default-rules`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    action: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecuritySshProfile:
    """A ``security ssh profile`` object — SSH proxy profile."""

    name: str
    full_path: str
    description: str = ""
    defaults_from: str = ""  # PathRef → security ssh profile
    timeout: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityHttpProfile:
    """A ``security http profile`` object — HTTP security profile."""

    name: str
    full_path: str
    description: str = ""
    defaults_from: str = ""  # PathRef → security http profile
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityBotDefenseProfile:
    """A ``security bot-defense profile`` object — bot defense profile."""

    name: str
    full_path: str
    description: str = ""
    app_service: str = ""
    template: str = ""
    range: Range | None = None


# Alias — same shape as the other minimal kinds.  Originally a
# dedicated dataclass for bundle-10b ``security.*`` kinds (``debug
# *``, ``dos signature``, ``datasync.*``, anti-fraud, blacklist-
# publisher, protocol-inspection learning stats, etc.); collapsed
# into the shared :class:`BigipMinimalObject` since the shape is
# identical.  ``kind`` carries the TMSH module + sub-type
# (e.g. ``"security dos virtual"``).


@dataclass(frozen=True, slots=True)
class BigipSecurityIpIntelligencePolicy:
    """A ``security ip-intelligence policy`` object.

    The body is typically empty (the policy is referenced from other
    contexts); we just surface name and full-path.
    """

    name: str
    full_path: str
    description: str = ""
    default_action: str = ""
    default_log_blacklist_hit_only: str = ""
    default_log_blacklist_category: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityProtocolInspectionComplianceMap:
    """A ``security protocol-inspection compliance-map`` object."""

    name: str
    full_path: str
    insp_id: str = ""
    key_type: str = ""
    value_type: str = ""
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityProtocolInspectionComplianceObject:
    """A ``security protocol-inspection compliance-objects`` object.

    Note: BIG-IP emits multiple stanzas with the same full-path but
    different ``insp-id`` values; the dict-keyed model surfaces only
    the last one encountered.  Use the source view for the full set.
    """

    name: str
    full_path: str
    insp_id: str = ""
    type_: str = ""
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityDeviceIdAttribute:
    """A ``security device-id attribute`` object."""

    name: str
    full_path: str
    id_: str = ""
    description: str = ""
    range: Range | None = None


# apm.* — typed projection for the Access Policy Manager module.
