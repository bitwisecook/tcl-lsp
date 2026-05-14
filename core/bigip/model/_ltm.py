"""Typed projection for the LTM module (``ltm.*``).

Covers every typed LTM kind:

- v1 core: data-groups, pools (+ members), nodes, profiles, monitors,
  SNAT pools, persistence profiles, iRules, virtual servers, virtual
  addresses, and the policy AST (policy / policy-rule / policy-action /
  policy-condition).
- bundles 13-16: cipher groups / rules, NAT, SNAT, traffic classes,
  iFiles, eviction policies, DNS Express, message-routing, LTM auth
  profiles.
- bundles 17-20 long-tail kinds (CGNAT / LSN, global-settings
  singletons, classification, URL-DB, tacdb) share
  :class:`BigipMinimalObject` via :data:`BigipLtmMinimalObject`.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field as dc_field
from typing import TYPE_CHECKING

from ...analysis.semantic_model import Range
from ._enums import DataGroupType, ProfileType

if TYPE_CHECKING:
    from ..types import FQDN, Address, Destination, IPAddress


# v1 LTM core types (data-groups, pools, virtuals, monitors, profiles,
# persistence, iRules, policies, virtual-addresses, snat-pools).
# Originally in ``_common.py``; consolidated here in followup A so all
# LTM dataclasses live in one place — the original "common" file was
# misnamed since every type in it was actually ``ltm.*``.
@dataclass(frozen=True, slots=True)
class BigipDataGroup:
    """A ``ltm data-group internal|external`` object."""

    name: str
    full_path: str  # e.g. "/Common/my_dg"
    kind: DataGroupType = DataGroupType.INTERNAL
    value_type: str = ""  # "string", "ip", "integer"
    records: tuple[str, ...] = ()  # record names/keys
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPoolMember:
    """A single member entry inside a ``ltm pool``.

    ``address`` is a typed :class:`Address` (``IPAddress`` for IPv4 /
    IPv6 hosts, :class:`FQDN` for FQDN-based pool members) populated
    by the parser; ``None`` when no ``address`` property was given.

    ``field_offsets`` carries the absolute (start, end) byte offsets
    of each field's value in the source so the projection layer can
    surface per-member :class:`FieldSlot` entries and the edit
    planner can rewrite a single member's property in place
    (``.pool.members[].address |= ip("10.50.0.0/16", .)``).
    Members written on a single line (no per-member body braces) do
    not contribute to this map.
    """

    name: str  # e.g. "/Common/10.0.0.1:80"
    address: "Address | None" = None
    port: int = 0
    monitor: str = ""
    description: str = ""
    state: str = ""  # ``enabled`` / ``disabled`` flag from the member body
    ratio: str = ""
    priority_group: str = ""
    connection_limit: str = ""
    rate_limit: str = ""
    # ``{tmsh-spelt key: (start, end)}`` in the original source.  The
    # range covers just the value half — ``address [10.0.0.1]`` →
    # range for ``10.0.0.1``.
    field_offsets: dict[str, tuple[int, int]] = dc_field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class BigipPool:
    """A ``ltm pool`` object."""

    name: str
    full_path: str
    module: str = "ltm"
    members: tuple[BigipPoolMember, ...] = ()
    monitor: str = ""
    load_balancing_mode: str = ""
    description: str = ""
    min_active_members: str = ""
    min_up_members: str = ""
    service_down_action: str = ""
    slow_ramp_time: str = ""
    allow_snat: str = ""
    allow_nat: str = ""
    reselect_tries: str = ""
    queue_depth_limit: str = ""
    queue_time_limit: str = ""
    connection_limit: str = ""
    rate_limit: str = ""
    ratio: str = ""
    down_interval: str = ""
    interval: str = ""
    min_up_members_action: str = ""
    min_up_members_checking: str = ""
    ip_tos_to_client: str = ""
    ip_tos_to_server: str = ""
    link_qos_to_client: str = ""
    link_qos_to_server: str = ""
    gateway_failsafe_device: str = ""
    ignore_persisted_weight: str = ""
    inherit_profile: str = ""
    queue_on_connection_limit: str = ""
    address_family: str = ""
    autopopulate: str = ""
    profiles: tuple[str, ...] = ()  # PathRefs → ltm profile
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNode:
    """A ``ltm node`` object.

    ``address`` is a typed :class:`Address` (``IPAddress`` for IPv4 /
    IPv6 hosts, :class:`FQDN` for FQDN nodes); ``fqdn`` is the typed
    :class:`FQDN` from a ``fqdn { name ... }`` sub-block when the
    node uses dynamic resolution.
    """

    name: str
    full_path: str
    address: "Address | None" = None
    description: str = ""
    monitor: str = ""
    state: str = ""  # ``enabled`` / ``disabled`` flag from the node body
    connection_limit: str = ""
    rate_limit: str = ""
    ratio: str = ""
    fqdn: "FQDN | None" = None
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipProfile:
    """A ``ltm profile <type>`` object."""

    name: str
    full_path: str
    profile_type: ProfileType = ProfileType.OTHER
    defaults_from: str = ""  # PathRef → ltm profile
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipMonitor:
    """A ``ltm monitor <type>`` object."""

    name: str
    full_path: str
    monitor_type: str = ""  # "http", "tcp", "https", etc.
    defaults_from: str = ""  # PathRef → ltm monitor
    description: str = ""
    interval: str = ""
    timeout: str = ""
    destination: str = ""
    send: str = ""
    recv: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSnatPool:
    """A ``ltm snatpool`` object."""

    name: str
    full_path: str
    members: tuple[str, ...] = ()
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPersistence:
    """A ``ltm persistence <type>`` object."""

    name: str
    full_path: str
    persistence_type: str = ""  # "cookie", "source-addr", "ssl", etc.
    defaults_from: str = ""  # PathRef → ltm persistence
    description: str = ""
    timeout: str = ""
    match_across_pools: str = ""
    match_across_services: str = ""
    match_across_virtuals: str = ""
    mirror: str = ""
    override_connection_limit: str = ""
    always_send: str = ""
    cookie_name: str = ""
    cookie_encryption: str = ""
    cookie_encryption_passphrase: str = ""
    httponly: str = ""
    secure: str = ""
    expiration: str = ""
    method: str = ""
    hash_length: str = ""
    hash_offset: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipRule:
    """A ``ltm rule`` object — an iRule embedded in the config."""

    name: str
    full_path: str
    source: str = ""  # the raw Tcl body
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipVirtualServer:
    """A ``ltm virtual`` object."""

    name: str
    full_path: str
    destination: "Destination | None" = None
    pool: str = ""  # default pool path
    rules: tuple[str, ...] = ()  # attached iRule paths
    profiles: tuple[str, ...] = ()  # attached profile paths
    # Typed per-attachment view of ``profiles``: each ListItem
    # carries a ``ProfileAttachment`` value (``.path``, ``.context``)
    # so DSL queries can ask ``.profiles[] | select(.context ==
    # "clientside")``.  Populated alongside ``profiles`` by the
    # parser; ``None`` when the legacy back-compat path is enough.
    profile_attachments: object = None  # BigipList | None
    persist: tuple[str, ...] = ()  # persistence profile paths
    # Sister field to ``profile_attachments`` for persistence
    # attachments — surfaces ``.default`` to the DSL.
    persist_attachments: object = None  # BigipList | None
    policies: tuple[str, ...] = ()  # ltm policy paths attached to this VS
    snatpool: str = ""
    source_address_translation: str = ""
    description: str = ""
    mask: str = ""
    source: str = ""
    ip_protocol: str = ""
    connection_limit: str = ""
    rate_limit: str = ""
    rate_limit_mode: str = ""
    rate_limit_dst_mask: str = ""
    rate_limit_src_mask: str = ""
    auto_lasthop: str = ""
    translate_address: str = ""
    translate_port: str = ""
    state: str = ""  # ``enabled`` / ``disabled`` flag from the virtual body
    address_status: str = ""
    auto_discovery: str = ""
    cmp_enabled: str = ""
    eviction_protected: str = ""
    dhcp_relay: bool = False  # bare ``dhcp-relay`` flag present
    internal: bool = False  # bare ``internal`` flag present
    ip_forward: bool = False  # bare ``ip-forward`` flag present
    l2_forward: bool = False  # bare ``l2-forward`` flag present
    reject: bool = False  # bare ``reject`` flag present
    nat64: str = ""
    gtm_score: str = ""
    mirror: str = ""
    service_down_immediate_action: str = ""
    source_port: str = ""
    serverssl_use_sni: str = ""
    rate_class: str = ""
    per_flow_request_access_policy: str = ""  # PathRef → apm policy access-policy
    transparent_nexthop: str = ""  # PathRef → net vlan
    vlans: tuple[str, ...] = ()  # PathRefs → net vlan
    vlans_disabled: bool = False  # ``vlans-disabled`` flag was present
    vlans_enabled: bool = False  # ``vlans-enabled`` flag was present
    fallback_persistence: str = ""  # PathRef → ltm persistence
    last_hop_pool: str = ""  # PathRef → ltm pool
    fw_enforced_policy: str = ""
    fw_staged_policy: str = ""
    flow_eviction_policy: str = ""
    service_policy: str = ""
    auth_profiles: tuple[str, ...] = ()
    traffic_classes: tuple[str, ...] = ()
    clone_pools: tuple[str, ...] = ()  # cloned PathRefs → ltm pool
    pool_range: Range | None = None
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipVirtualAddress:
    """A ``ltm virtual-address`` object.

    Distinct from ``ltm virtual``: this is the listener IP itself
    (ARP / ICMP-echo / route-advertisement settings, traffic-group
    binding).  Every ``ltm virtual.destination`` references one.
    """

    name: str
    full_path: str
    address: "IPAddress | None" = None
    mask: str = ""
    arp: str = ""
    icmp_echo: str = ""
    auto_delete: str = ""
    connection_limit: str = ""
    traffic_group: str = ""  # PathRef → cm traffic-group
    inherited_traffic_group: str = ""
    route_advertisement: str = ""
    server_scope: str = ""
    spanning: str = ""
    unit: str = ""
    description: str = ""
    state: str = ""  # ``enabled`` / ``disabled`` bare flag
    floating: str = ""
    traffic_group_restored: str = ""
    range: Range | None = None

    @property
    def network_typed(self):
        """The :attr:`address` + :attr:`mask` combined as a typed
        :class:`Network` (CIDR).

        Some virtual-addresses are wildcard-mask listeners
        (``0.0.0.0/0``); :meth:`Network.try_parse` handles both
        host-mask and CIDR notation.
        """
        from ..types import Network

        if self.address is None:
            return None
        if self.mask:
            return Network.try_parse(f"{self.address}/{self.mask}")
        # No mask — treat the address as a /32 (IPv4) or /128 (IPv6) host.
        return Network.try_parse(str(self.address))


@dataclass(frozen=True, slots=True)
class BigipPolicyCondition:
    """A single ``conditions { N { … } }`` entry inside a policy rule.

    Operand / selector / operator are bare positional flag tokens in
    the source (``http-host host equals``); we classify them by
    membership in a finite vocabulary at parse time.  ``name`` is the
    targeted header / extension for ``http-header`` / ``ssl-extension``
    operands, where the source carries it as ``name <value>``.
    """

    index: int
    operand: str = ""  # http-host, http-uri, http-method, http-header, ssl-extension, tcp
    selector: str = ""  # operand-specific (host, path, query, all, address, …)
    operator: str = "equals"
    values: tuple[str, ...] = ()
    name: str = ""  # http-header / ssl-extension target name
    negate: bool = False
    case_insensitive: bool = False
    event: str = ""  # request, response, ssl-client-hello, …


@dataclass(frozen=True, slots=True)
class BigipPolicyAction:
    """A single ``actions { N { … } }`` entry inside a policy rule."""

    index: int
    target: str = ""  # forward, http-reply, http-uri, http-header, http-cookie, tcp, log
    verb: str = ""  # select, redirect, replace, insert, remove, reset, drop
    pool: str = ""
    location: str = ""
    name: str = ""  # http-header / cookie target name
    value: str = ""
    path: str = ""  # http-uri replace path / query / host component
    query: str = ""
    host: str = ""
    event: str = ""


@dataclass(frozen=True, slots=True)
class BigipPolicyRule:
    """A named rule inside a policy: ordered conditions + actions."""

    name: str
    ordinal: int = 0
    conditions: tuple[BigipPolicyCondition, ...] = ()
    actions: tuple[BigipPolicyAction, ...] = ()


@dataclass(frozen=True, slots=True)
class BigipPolicy:
    """A ``ltm policy`` object."""

    name: str
    full_path: str
    strategy: str = "first-match"  # first-match | all-match | best-match
    requires: tuple[str, ...] = ()
    controls: tuple[str, ...] = ()
    rules: tuple[BigipPolicyRule, ...] = ()
    description: str = ""
    status: str = ""  # ``published`` / ``draft`` / ``legacy``
    last_modified: str = ""
    range: Range | None = None


# Bundle 13 — ltm.* cross-cutting infra (cipher group/rule, nat,
# snat, snat-translation, policy-strategy, traffic-class,
# traffic-matching-criteria, ifile, eviction-policy).


@dataclass(frozen=True, slots=True)
class BigipLtmCipherGroup:
    """A ``ltm cipher group`` object — collection of cipher rules
    with allow / require / exclude semantics."""

    name: str
    full_path: str
    description: str = ""
    allow: tuple[str, ...] = ()  # PathRefs → ltm cipher rule
    require: tuple[str, ...] = ()  # PathRefs → ltm cipher rule
    exclude: tuple[str, ...] = ()  # PathRefs → ltm cipher rule
    ordering: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmCipherRule:
    """A ``ltm cipher rule`` object — a named cipher list."""

    name: str
    full_path: str
    description: str = ""
    cipher: str = ""
    dh_groups: str = ""
    signature_algorithms: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmNat:
    """A ``ltm nat`` object — 1:1 NAT mapping."""

    name: str
    full_path: str
    description: str = ""
    translation_address: str = ""
    originating_address: str = ""
    traffic_group: str = ""  # PathRef → cm traffic-group
    vlans: tuple[str, ...] = ()  # PathRefs → net vlan
    vlans_disabled: bool = False
    vlans_enabled: bool = False
    mirror: str = ""
    arp: str = ""
    state: str = ""  # enabled/disabled
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmSnat:
    """A ``ltm snat`` object — Source NAT.

    ``origins`` surfaces the top-level keys of the ``origins { … }``
    sub-block (each key is an originating subnet); the SNAT can also
    declare ``automap``, a translation IP, or a snatpool.
    """

    name: str
    full_path: str
    description: str = ""
    origins: tuple[str, ...] = ()
    translation: str = ""
    snatpool: str = ""  # PathRef → ltm snatpool
    vlans: tuple[str, ...] = ()  # PathRefs → net vlan
    vlans_disabled: bool = False
    vlans_enabled: bool = False
    automap: bool = False  # bare flag
    mirror: str = ""
    state: str = ""  # enabled/disabled
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmSnatTranslation:
    """A ``ltm snat-translation`` object — a SNAT address pool entry."""

    name: str
    full_path: str
    description: str = ""
    address: str = ""
    inherited_traffic_group: str = ""
    traffic_group: str = ""  # PathRef → cm traffic-group
    connection_limit: str = ""
    ip_idle_timeout: str = ""
    tcp_idle_timeout: str = ""
    udp_idle_timeout: str = ""
    state: str = ""  # enabled/disabled
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmPolicyStrategy:
    """A ``ltm policy-strategy`` object — operand-matching strategy
    used by ``ltm policy``.  ``operands`` surfaces the top-level
    indexed sub-block keys (the per-operand bodies are flattened in
    v1; reach for ``--scf`` for the source view).
    """

    name: str
    full_path: str
    description: str = ""
    strategy: str = ""
    operands: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmTrafficClass:
    """A ``ltm traffic-class`` object."""

    name: str
    full_path: str
    description: str = ""
    classification: str = ""
    match_method: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmTrafficMatchingCriteria:
    """A ``ltm traffic-matching-criteria`` object.

    Carries destination / source address / port matching used by VS
    with traffic-matching-criteria (TMC-based virtuals).  Address +
    port fields come in two forms: ``-list`` PathRefs into
    ``security firewall address-list`` / ``security firewall
    port-list``, or ``-inline`` literal values.
    """

    name: str
    full_path: str
    description: str = ""
    destination_address_list: str = ""  # PathRef → security firewall address-list
    destination_address_inline: str = ""
    destination_port_list: str = ""  # PathRef → security firewall port-list
    destination_port_inline: str = ""
    source_address_list: str = ""  # PathRef → security firewall address-list
    source_address_inline: str = ""
    source_port_list: str = ""  # PathRef → security firewall port-list
    source_port_inline: str = ""
    protocol: str = ""
    route_domain: str = ""  # PathRef → net route-domain
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmIfile:
    """A ``ltm ifile`` object — iRule-accessible inline file."""

    name: str
    full_path: str
    description: str = ""
    file_name: str = ""  # PathRef → sys file ifile
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmEvictionPolicy:
    """A ``ltm eviction-policy`` object."""

    name: str
    full_path: str
    description: str = ""
    high_water_mark: str = ""
    low_water_mark: str = ""
    slow_flow_throttle: str = ""
    slow_flow_monitoring: str = ""
    range: Range | None = None


# Bundle 14 — ltm dns.* (DNS Express).


@dataclass(frozen=True, slots=True)
class BigipLtmDnsNameserver:
    """A ``ltm dns nameserver`` object — external DNS nameserver."""

    name: str
    full_path: str
    description: str = ""
    address: str = ""
    port: str = ""
    tsig_key: str = ""  # PathRef → ltm dns tsig-key
    route_domain: str = ""  # PathRef → net route-domain
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsTsigKey:
    """A ``ltm dns tsig-key`` object."""

    name: str
    full_path: str
    description: str = ""
    algorithm: str = ""
    secret: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsZone:
    """A ``ltm dns zone`` object — DNS Express zone."""

    name: str
    full_path: str
    description: str = ""
    dns_express_server: str = ""  # PathRef → ltm dns nameserver
    dns_express_allow_notify: tuple[str, ...] = ()  # PathRefs → ltm dns nameserver
    dns_express_enabled: str = ""
    response_policy: str = ""
    transfer_clients: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsDnssecKey:
    """A ``ltm dns dnssec key`` object."""

    name: str
    full_path: str
    description: str = ""
    type_: str = ""  # zone-signing-key / key-signing-key
    algorithm: str = ""
    bit_width: str = ""
    rollover_period: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsDnssecZone:
    """A ``ltm dns dnssec zone`` object."""

    name: str
    full_path: str
    description: str = ""
    keys: tuple[str, ...] = ()  # PathRefs → ltm dns dnssec key
    enable: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsCacheResolver:
    """A ``ltm dns cache resolver`` object."""

    name: str
    full_path: str
    description: str = ""
    message_cache_size: str = ""
    resolver_cache_size: str = ""
    answer_default_zones: str = ""
    forward_zones: tuple[str, ...] = ()
    route_domain: str = ""  # PathRef → net route-domain
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsCacheTransparent:
    """A ``ltm dns cache transparent`` object."""

    name: str
    full_path: str
    description: str = ""
    message_cache_size: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsCacheValidatingResolver:
    """A ``ltm dns cache validating-resolver`` object."""

    name: str
    full_path: str
    description: str = ""
    message_cache_size: str = ""
    resolver_cache_size: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsCacheGlobalSettings:
    """The ``ltm dns cache global-settings`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    expiry_time: str = ""
    nameserver_ttl: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsCacheRecord:
    """A ``ltm dns cache records <kind>`` object.

    ``record_kind`` distinguishes the five sub-kinds (``all``,
    ``key``, ``msg``, ``nameserver``, ``rrset``); all share the
    same shape (cache snapshot data).  These are normally
    runtime / read-only kinds — we surface them so they're
    queryable but don't model the per-kind cache record details.
    """

    name: str
    full_path: str
    record_kind: str = ""  # ``all`` / ``key`` / ``msg`` / ``nameserver`` / ``rrset``
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsHpkeKey:
    """A ``ltm dns hpke key`` object."""

    name: str
    full_path: str
    description: str = ""
    algorithm: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsHpkeProfile:
    """A ``ltm dns hpke profile`` object."""

    name: str
    full_path: str
    description: str = ""
    defaults_from: str = ""  # PathRef → ltm dns hpke profile
    keys: tuple[str, ...] = ()  # PathRefs → ltm dns hpke key
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipLtmDnsAnalyticsGlobalSettings:
    """The ``ltm dns analytics global-settings`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    range: Range | None = None


# Bundle 15 — ltm message-routing.* (20 kinds across diameter / sip /
# mqtt / generic protocols).  Each kind has its own ``BigipConfig``
# dict, but all share this minimal shape; ``kind`` carries the
# protocol + row label so a query that hits
# ``.ltm.message-routing-diameter-peer[].kind`` returns ``"ltm
# message-routing diameter peer"``.


@dataclass(frozen=True, slots=True)
class BigipLtmMessageRoutingObject:
    """A ``ltm message-routing *`` object — shared minimal shape."""

    name: str
    full_path: str
    kind: str = ""
    description: str = ""
    range: Range | None = None


# Bundle 16 — ltm auth.* profiles (11 kinds).  These are the
# LTM-side auth profile entities (distinct from the
# administrative ``auth.*`` namespace in bundle 8).  All share
# this minimal shape with the kind label preserved on each
# instance.


@dataclass(frozen=True, slots=True)
class BigipLtmAuthObject:
    """A ``ltm auth *`` object — shared minimal shape."""

    name: str
    full_path: str
    kind: str = ""
    description: str = ""
    defaults_from: str = ""
    range: Range | None = None


# Bundles 17-20 — shared minimal shape for the long-tail ltm.*
# kinds (CGNAT / LSN, global-settings singletons, classification /
# URL-DB, tacdb).  Each kind keeps its own ``BigipConfig``
# attribute; the ``kind`` field preserves the full TMSH label.
