"""Data model for F5 BIG-IP configuration objects.

Represents the parsed inventory of a ``bigip.conf`` (or SCF) file:
virtual servers, pools, data-groups, profiles, iRules, nodes, monitors,
SNAT pools, and persistence profiles.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field
from enum import Enum, auto

from ..analysis.semantic_model import Range

# Enums


class DataGroupType(Enum):
    """Whether a data-group is stored inline or in an external file."""

    INTERNAL = auto()
    EXTERNAL = auto()


class ProfileType(Enum):
    """Broad classification of BIG-IP profile types."""

    HTTP = auto()
    TCP = auto()
    UDP = auto()
    CLIENT_SSL = auto()
    SERVER_SSL = auto()
    FTP = auto()
    DNS = auto()
    SIP = auto()
    DIAMETER = auto()
    FIX = auto()
    RADIUS = auto()
    MQTT = auto()
    WEBSOCKET = auto()
    STREAM = auto()
    HTML = auto()
    REWRITE = auto()
    FASTHTTP = auto()
    FASTL4 = auto()
    ONE_CONNECT = auto()
    PERSISTENCE = auto()
    OTHER = auto()


# Parsed objects


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
    """A single member entry inside a ``ltm pool``."""

    name: str  # e.g. "/Common/10.0.0.1:80"
    address: str = ""
    port: int = 0
    monitor: str = ""
    description: str = ""
    state: str = ""  # ``enabled`` / ``disabled`` flag from the member body
    ratio: str = ""
    priority_group: str = ""
    connection_limit: str = ""
    rate_limit: str = ""


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
    """A ``ltm node`` object."""

    name: str
    full_path: str
    address: str = ""
    description: str = ""
    monitor: str = ""
    state: str = ""  # ``enabled`` / ``disabled`` flag from the node body
    connection_limit: str = ""
    rate_limit: str = ""
    ratio: str = ""
    fqdn: str = ""  # FQDN sub-block ``name`` value, when present
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
    destination: str = ""
    pool: str = ""  # default pool path
    rules: tuple[str, ...] = ()  # attached iRule paths
    profiles: tuple[str, ...] = ()  # attached profile paths
    persist: tuple[str, ...] = ()  # persistence profile paths
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


@dataclass(frozen=True, slots=True)
class BigipMinimalObject:
    """Shared shape for every "minimal" projection.

    The DSL has hundreds of kinds — most carry only the identity
    tuple (``name``, ``full_path``) plus a ``description`` and the
    TMSH kind label (``kind``).  Rather than mint a separate
    dataclass per module, every minimal kind shares this one.  The
    discriminator is ``kind`` (e.g. ``"net routing as-path"``,
    ``"sys icall script"``), which the projection already exposes
    via ``.kind``.
    """

    name: str
    full_path: str
    kind: str = ""
    description: str = ""
    range: Range | None = None


# Per-module aliases — kept so existing imports and isinstance
# checks across the parser / projection / tests continue to work
# without an attribute renaming sweep.  All resolve to the same
# class, so ``isinstance(x, BigipNetMinimalObject)`` and
# ``isinstance(x, BigipSecurityMinimalObject)`` are the same
# runtime check.
BigipLtmMinimalObject = BigipMinimalObject
BigipNetMinimalObject = BigipMinimalObject
BigipApmMinimalObject = BigipMinimalObject
BigipPemMinimalObject = BigipMinimalObject
BigipSysMinimalObject = BigipMinimalObject
BigipVcmpMinimalObject = BigipMinimalObject
BigipCmMinimalObject = BigipMinimalObject
BigipCliMinimalObject = BigipMinimalObject
BigipApiProtectionMinimalObject = BigipMinimalObject
BigipAsmMinimalObject = BigipMinimalObject
BigipIlxMinimalObject = BigipMinimalObject
BigipWomMinimalObject = BigipMinimalObject
BigipAnalyticsMinimalObject = BigipMinimalObject


@dataclass(frozen=True, slots=True)
class BigipVirtualAddress:
    """A ``ltm virtual-address`` object.

    Distinct from ``ltm virtual``: this is the listener IP itself
    (ARP / ICMP-echo / route-advertisement settings, traffic-group
    binding).  Every ``ltm virtual.destination`` references one.
    """

    name: str
    full_path: str
    address: str = ""
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


@dataclass(frozen=True, slots=True)
class BigipGenericObject:
    """A generic BIG-IP stanza retained when no specialised model exists."""

    module: str  # e.g. "net", "auth", "sys"
    object_type: str  # e.g. "route-domain", "partition", "user"
    identifier: str  # e.g. "/Common/0", "admin", or "" for singleton stanzas
    header: str
    range: Range | None = None


# ── net.* — typed projection for the network module ─────────────────


@dataclass(frozen=True, slots=True)
class BigipNetRoute:
    """A ``net route`` object — a routing-table entry."""

    name: str
    full_path: str
    network: str = ""  # e.g. "default", "10.0.0.0/8"
    gw: str = ""  # gateway address; empty when the route uses ``pool`` instead
    pool: str = ""  # gateway pool reference; empty when ``gw`` is set
    description: str = ""
    mtu: str = ""
    blackhole: bool = False  # ``blackhole`` flag present
    interface: str = ""  # PathRef → net vlan when set
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetVlan:
    """A ``net vlan`` object."""

    name: str
    full_path: str
    tag: int = 0
    interfaces: tuple[str, ...] = ()  # untagged interface names ("1.1", "1.2")
    description: str = ""
    mtu: str = ""
    cmp_hash: str = ""
    failsafe: str = ""
    failsafe_action: str = ""
    failsafe_timeout: str = ""
    fwd_mode: str = ""
    hardware_syncookie: str = ""
    learning: str = ""
    tag_mode: str = ""
    virtual_wire: str = ""
    auto_lasthop: str = ""
    source_check: str = ""
    source_checking: str = ""
    syn_flood_rate_limit: str = ""
    syncache_threshold: str = ""
    service_policy: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetSelf:
    """A ``net self`` object — a self IP bound to a VLAN."""

    name: str
    full_path: str
    address: str = ""  # ``10.0.0.1/24``
    vlan: str = ""  # full-path of the bound VLAN
    traffic_group: str = ""
    allow_service: tuple[str, ...] = ()  # ``default`` / ``all`` / per-service tokens
    description: str = ""
    floating: str = ""  # ``enabled`` / ``disabled``
    unit: str = ""
    service_policy: str = ""
    fw_enforced_policy: str = ""
    fw_staged_policy: str = ""
    inherited_traffic_group: str = ""
    address_source: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetRouteDomain:
    """A ``net route-domain`` object."""

    name: str
    full_path: str
    id: int = 0
    vlans: tuple[str, ...] = ()  # member VLAN full-paths
    description: str = ""
    parent: str = ""  # PathRef → net route-domain
    strict: str = ""
    fw_enforced_policy: str = ""
    fw_staged_policy: str = ""
    bwc_policy: str = ""
    connection_limit: str = ""
    flow_eviction_policy: str = ""
    routing_protocol: tuple[str, ...] = ()
    security_nat_policy: str = ""
    service_policy: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetPortList:
    """A ``net port-list`` object — used by self-allow and policy rules."""

    name: str
    full_path: str
    ports: tuple[str, ...] = ()  # raw port specs (e.g. ``80``, ``1029-1043``)
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetInterface:
    """A ``net interface`` object — a physical / logical NIC.

    Interface ``name`` is the bare slot/port (``1.1``, ``mgmt``) — no
    partition prefix and no full-path slash.  ``full_path`` mirrors
    ``name`` for consistency with the other typed kinds.
    """

    name: str
    full_path: str
    media_fixed: str = ""  # e.g. "10000T-FD"
    description: str = ""
    enabled: bool = False  # ``enabled`` flag present
    disabled: bool = False  # ``disabled`` flag present
    bundle: str = ""
    bundle_speed: str = ""
    lldp_admin: str = ""
    mtu: str = ""
    flow_control: str = ""
    mac_address: str = ""
    media_active: str = ""
    media_max: str = ""
    media_sfp: str = ""
    port_fwd_mode: str = ""
    qinq_ethertype: str = ""
    stp: str = ""
    stp_edge_port: str = ""
    stp_link_type: str = ""
    stp_auto_edge_port: str = ""
    stp_reset: str = ""
    sflow_poll_interval: str = ""
    sflow_poll_interval_global: str = ""
    vendor: str = ""
    vendor_oui: str = ""
    vendor_partnum: str = ""
    vendor_revision: str = ""
    virtual_wire: str = ""
    transmitter_technology: str = ""
    lacp_port_priority: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetDnsResolver:
    """A ``net dns-resolver`` object."""

    name: str
    full_path: str
    route_domain: str = ""  # full-path of bound route-domain
    forward_zones: tuple[str, ...] = ()  # top-level zone names only
    description: str = ""
    cache_size: str = ""
    randomize_query_name_case: str = ""
    use_ipv4: str = ""
    use_ipv6: str = ""
    use_tcp: str = ""
    use_udp: str = ""
    nameservers: tuple[str, ...] = ()  # surface bare nameserver entry keys
    answer_default_zones: str = ""
    prefetch: str = ""
    nameserver_min_rtt: str = ""
    nameserver_ttl: str = ""
    outbound_msg_retry: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetTunnel:
    """A ``net tunnels tunnel`` object."""

    name: str
    full_path: str
    profile: str = ""  # tunnel profile path
    local_address: str = ""
    remote_address: str = ""
    description: str = ""
    mtu: str = ""
    mode: str = ""
    idle_timeout: str = ""
    auto_lasthop: str = ""
    secondary_address: str = ""
    traffic_group: str = ""  # PathRef → cm traffic-group
    transparent: str = ""
    key: str = ""
    use_pmtu: str = ""
    tos: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetStp:
    """A ``net stp`` object — spanning-tree instance.

    ``interfaces`` is the list of bare interface names this STP
    instance attaches to; nested per-interface costs are not modelled
    in v1.
    """

    name: str
    full_path: str
    interfaces: tuple[str, ...] = ()
    description: str = ""
    mode: str = ""
    priority: str = ""
    external_path_cost: str = ""
    internal_path_cost: str = ""
    vlans: tuple[str, ...] = ()  # PathRefs → net vlan
    range: Range | None = None


# sys.* — typed projection for the system module.
#
# Singletons (``sys dns``, ``sys ntp``, ``sys snmp``, ``sys
# global-settings``) have no full-path; they're stored with the empty
# string as the dict key so ``.sys.<kind>[]`` streams the one entry.


@dataclass(frozen=True, slots=True)
class BigipSysDns:
    """The ``sys dns`` singleton — DNS resolver settings."""

    name: str = ""
    full_path: str = ""
    name_servers: tuple[str, ...] = ()
    search: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysNtp:
    """The ``sys ntp`` singleton — NTP server settings."""

    name: str = ""
    full_path: str = ""
    servers: tuple[str, ...] = ()
    timezone: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysSnmp:
    """The ``sys snmp`` singleton — SNMP agent settings."""

    name: str = ""
    full_path: str = ""
    agent_addresses: tuple[str, ...] = ()
    communities: tuple[str, ...] = ()  # full-paths of community sub-objects
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysGlobalSettings:
    """The ``sys global-settings`` singleton."""

    name: str = ""
    full_path: str = ""
    hostname: str = ""
    gui_setup: str = ""
    mgmt_dhcp: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysProvision:
    """A ``sys provision <module>`` object — module provisioning level.

    ``name`` is the bare module token (``ltm``, ``sslo``, ``urldb``);
    ``full_path`` mirrors it.
    """

    name: str
    full_path: str
    level: str = ""
    cpu_ratio: str = ""
    memory_ratio: str = ""
    disk_ratio: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysFolder:
    """A ``sys folder`` object — partition / folder metadata."""

    name: str
    full_path: str
    device_group: str = ""
    traffic_group: str = ""
    hidden: str = ""
    description: str = ""
    inherited_device_group: str = ""
    inherited_traffic_group: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysFileSslCert:
    """A ``sys file ssl-cert`` object."""

    name: str
    full_path: str
    source_path: str = ""
    cache_path: str = ""
    revision: str = ""
    description: str = ""
    issuer: str = ""
    subject: str = ""
    expiration_string: str = ""
    expiration_date: str = ""
    fingerprint: str = ""
    key_size: str = ""
    key_type: str = ""
    is_bundle: str = ""
    certificate_key_size: str = ""
    issuer_cert: str = ""  # PathRef → sys file ssl-cert
    serial_number: str = ""
    version: str = ""
    subject_alternative_name: str = ""
    bundle_certificates: tuple[str, ...] = ()
    cert_validation_options: tuple[str, ...] = ()
    cert_validators: tuple[str, ...] = ()
    checksum: str = ""
    mode: str = ""
    size: str = ""
    create_time: str = ""
    created_by: str = ""
    last_update_time: str = ""
    updated_by: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysFileSslKey:
    """A ``sys file ssl-key`` object."""

    name: str
    full_path: str
    source_path: str = ""
    cache_path: str = ""
    revision: str = ""
    passphrase: str = ""
    description: str = ""
    key_size: str = ""
    key_type: str = ""
    security_type: str = ""
    checksum: str = ""
    mode: str = ""
    size: str = ""
    create_time: str = ""
    created_by: str = ""
    last_update_time: str = ""
    updated_by: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysManagementRoute:
    """A ``sys management-route`` object."""

    name: str
    full_path: str
    gateway: str = ""
    network: str = ""
    mtu: str = ""
    description: str = ""
    range: Range | None = None


# security.* — typed projection for AFM / DDoS / device-id / inspection.


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

    Only top-level rule names are surfaced in v1; the nested
    per-rule ``action`` / ``ip-protocol`` / source / destination
    blocks are reachable through the unmodelled-stanza source view.
    """

    name: str
    full_path: str
    rules: tuple[str, ...] = ()
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
BigipSecurityMinimalObject = BigipMinimalObject


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


@dataclass(frozen=True, slots=True)
class BigipApmEphemeralAuthSshSecurityConfig:
    """A ``apm ephemeral-auth ssh-security-config`` object.

    The body lists ciphers / hmacs / kex-methods / compressions as
    numerically-keyed sub-blocks; we surface flattened name lists in
    v1 and leave the per-entry detail to the source view.
    """

    name: str
    full_path: str
    ciphers: tuple[str, ...] = ()
    hmacs: tuple[str, ...] = ()
    kex_methods: tuple[str, ...] = ()
    compressions: tuple[str, ...] = ()
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmOauthDbInstance:
    """A ``apm oauth db-instance`` object."""

    name: str
    full_path: str
    description: str = ""
    db_name: str = ""
    purge_frequency: str = ""
    purge_time: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmPolicyAccessPolicy:
    """A ``apm policy access-policy`` object — a per-flow access policy."""

    name: str
    full_path: str
    start_item: str = ""  # PathRef → apm policy policy-item
    default_ending: str = ""  # PathRef → apm policy policy-item
    items: tuple[str, ...] = ()  # PathRefs → apm policy policy-item
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmPolicyCustomizationSource:
    """A ``apm policy customization-source`` object."""

    name: str
    full_path: str
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmPolicyItem:
    """A ``apm policy policy-item`` object — a node in an access policy.

    ``agents`` holds the full-paths of the agent sub-block keys; the
    nested ``rules { { … } { … } }`` anonymous sequence is not
    modelled in v1 (use the source view).
    """

    name: str
    full_path: str
    caption: str = ""
    color: str = ""
    item_type: str = ""  # action | ending
    agents: tuple[str, ...] = ()
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmPolicyAgent:
    """An ``apm policy agent <type>`` object.

    The agent type (``ending-allow``, ``ending-deny``, ``kerberos``,
    …) is captured in ``agent_type`` so callers can filter without
    reaching into the kind string.
    """

    name: str
    full_path: str
    agent_type: str = ""
    customization_group: str = ""
    auth: str = ""
    max_logon_attempt: str = ""
    auth_max_logon_attempt: str = ""
    fetch_nested_groups: str = ""
    fetch_primary_groups: str = ""
    password_source: str = ""
    query: str = ""
    query_attrname: str = ""
    query_filter: str = ""
    server: str = ""
    show_extended_error: str = ""
    upn: str = ""
    username_source: str = ""
    attribute_consuming_service: str = ""
    attr_consuming_service_session_var: str = ""
    hints: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmReportDefaultReport:
    """The ``apm report default-report`` singleton."""

    name: str = ""
    full_path: str = ""
    report_name: str = ""
    user: str = ""
    range: Range | None = None


# cm.* — typed projection for the Cluster Manager module.


@dataclass(frozen=True, slots=True)
class BigipCmCert:
    """A ``cm cert`` object — a device-trust certificate."""

    name: str
    full_path: str
    cache_path: str = ""
    checksum: str = ""
    revision: str = ""
    issuer: str = ""
    subject: str = ""
    subject_alternative_name: str = ""
    expiration_date: str = ""
    expiration_string: str = ""
    fingerprint: str = ""
    serial_number: str = ""
    version: str = ""
    key_type: str = ""
    certificate_key_size: str = ""
    is_bundle: str = ""
    email: str = ""
    source_path: str = ""
    system_path: str = ""
    size: str = ""
    mode: str = ""
    create_time: str = ""
    created_by: str = ""
    last_update_time: str = ""
    updated_by: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmKey:
    """A ``cm key`` object — the private key paired with a ``cm cert``."""

    name: str
    full_path: str
    cache_path: str = ""
    checksum: str = ""
    revision: str = ""
    key_size: str = ""
    key_type: str = ""
    security_type: str = ""
    source_path: str = ""
    system_path: str = ""
    size: str = ""
    mode: str = ""
    create_time: str = ""
    created_by: str = ""
    last_update_time: str = ""
    updated_by: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmDevice:
    """A ``cm device`` object.

    Only the core scalar identity / placement fields are surfaced; the
    bulky ``active-modules`` / ``optional-modules`` / ``time-limited-modules``
    lists (quoted multi-line bundles) are left to the source view.
    """

    name: str
    full_path: str
    hostname: str = ""
    management_ip: str = ""
    base_mac: str = ""
    build: str = ""
    edition: str = ""
    version: str = ""
    product: str = ""
    platform_id: str = ""
    chassis_id: str = ""
    marketing_name: str = ""
    self_device: str = ""  # "true" / "false" (text, not coerced)
    time_zone: str = ""
    cert: str = ""  # PathRef → cm cert
    key: str = ""  # PathRef → cm key
    description: str = ""
    comment: str = ""
    contact: str = ""
    location: str = ""
    mirror_ip: str = ""
    mirror_secondary_ip: str = ""
    multicast_interface: str = ""
    multicast_ip: str = ""
    multicast_port: str = ""
    unicast_address: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmDeviceGroup:
    """A ``cm device-group`` object."""

    name: str
    full_path: str
    auto_sync: str = ""
    network_failover: str = ""
    hidden: str = ""
    devices: tuple[str, ...] = ()  # PathRefs → cm device
    description: str = ""
    type_: str = ""  # ``sync-failover`` / ``sync-only``
    save_on_auto_sync: str = ""
    full_load_on_sync: str = ""
    asm_sync: str = ""
    incremental_config_sync_size_max: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmTrafficGroup:
    """A ``cm traffic-group`` object."""

    name: str
    full_path: str
    unit_id: str = ""
    description: str = ""
    default_device: str = ""  # PathRef → cm device
    ha_load_factor: str = ""
    ha_order: tuple[str, ...] = ()  # PathRefs → cm device
    auto_failback_enabled: str = ""
    auto_failback_time: str = ""
    mac: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmTrustDomain:
    """A ``cm trust-domain`` object."""

    name: str
    full_path: str
    ca_cert: str = ""  # PathRef → cm cert
    ca_cert_bundle: str = ""  # PathRef → cm cert
    ca_key: str = ""  # PathRef → cm key
    ca_devices: tuple[str, ...] = ()  # PathRefs → cm device
    guid: str = ""
    status: str = ""
    trust_group: str = ""  # PathRef → cm device-group
    range: Range | None = None


# gtm.* — typed projection for the Global Traffic Manager module.


@dataclass(frozen=True, slots=True)
class BigipGtmDatacenter:
    """A ``gtm datacenter`` object."""

    name: str
    full_path: str
    contact: str = ""
    location: str = ""
    description: str = ""
    prober_pool: str = ""  # PathRef → gtm prober-pool
    prober_preference: str = ""
    prober_fallback: str = ""
    state: str = ""  # ``enabled`` / ``disabled``
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmServer:
    """A ``gtm server`` object — a logical server attached to a DC.

    ``addresses`` flattens the addresses across every nested
    ``devices { N { addresses { ... } } }`` sub-block.
    ``virtual_servers`` surfaces the ``destination`` value of every
    numerically-keyed ``virtual-servers { N { ... } }`` entry.
    """

    name: str
    full_path: str
    datacenter: str = ""  # PathRef → gtm datacenter
    monitor: str = ""
    product: str = ""
    addresses: tuple[str, ...] = ()
    virtual_servers: tuple[str, ...] = ()  # destinations of each VS sub-block
    description: str = ""
    state: str = ""  # ``enabled`` / ``disabled``
    prober_pool: str = ""  # PathRef → gtm prober-pool
    prober_preference: str = ""
    prober_fallback: str = ""
    virtual_server_discovery: str = ""
    expose_route_domains: str = ""
    iq_allow_path: str = ""
    iq_allow_service_check: str = ""
    iq_allow_snmp: str = ""
    limit_max_bps: str = ""
    limit_max_connections: str = ""
    limit_max_pps: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmPool:
    """A ``gtm pool <record-type>`` object — a record-type-tagged pool.

    ``record_type`` is the DNS record type (``a``, ``aaaa``, ``cname``,
    ``mx``, ``srv``, ``naptr``) — surface as one ``BigipGtmPool`` per
    kind rather than 6 near-identical dataclasses.
    """

    name: str
    full_path: str
    record_type: str = ""
    members: tuple[str, ...] = ()
    monitor: str = ""
    alternate_mode: str = ""
    fallback_mode: str = ""
    load_balancing_mode: str = ""
    ttl: str = ""
    description: str = ""
    state: str = ""  # ``enabled`` / ``disabled``
    verify_member_availability: str = ""
    fallback_ip: str = ""
    max_answers_returned: str = ""
    qos_hit_ratio: str = ""
    qos_hops: str = ""
    qos_kbps: str = ""
    qos_lcs: str = ""
    qos_packet_rate: str = ""
    qos_rtt: str = ""
    qos_topology: str = ""
    qos_vs_capacity: str = ""
    qos_vs_score: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmWideip:
    """A ``gtm wideip <record-type>`` object.

    ``pools`` is a list of PathRefs into ``gtm pool <record-type>``;
    record_type carries the DNS record type that disambiguates which
    pool kind to dereference.
    """

    name: str
    full_path: str
    record_type: str = ""
    pools: tuple[str, ...] = ()  # PathRefs → gtm pool <record-type>
    aliases: tuple[str, ...] = ()
    pool_lb_mode: str = ""
    last_resort_pool: str = ""
    description: str = ""
    state: str = ""  # ``enabled`` / ``disabled``
    failure_rcode: str = ""
    failure_rcode_response: str = ""
    failure_rcode_ttl: str = ""
    minimal_response: str = ""
    persistence: str = ""
    persist_cidr_ipv4: str = ""
    persist_cidr_ipv6: str = ""
    topology_prefer_edns0_client_subnet: str = ""
    ttl_persistence: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmProberPool:
    """A ``gtm prober-pool`` object."""

    name: str
    full_path: str
    description: str = ""
    load_balancing_mode: str = ""
    members: tuple[str, ...] = ()  # PathRefs → gtm server
    state: str = ""  # ``enabled`` / ``disabled``
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmRegion:
    """A ``gtm region`` object — a named topology region.

    ``region_members`` surfaces the top-level keys of the
    ``region-members { ... }`` block.  The individual member shapes
    (``continent SA``, ``not country DE``, …) are token sequences
    rather than full-paths; ``region-members`` is intentionally a
    plain string tuple, not a PathRef list.
    """

    name: str
    full_path: str
    description: str = ""
    region_members: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmRule:
    """A ``gtm rule`` object — a GTM iRule (DNS_REQUEST etc.)."""

    name: str
    full_path: str
    source: str = ""
    description: str = ""
    range: Range | None = None


# Bundle 12 — gtm listeners / link / topology / distributed-app /
# global-settings singletons.


@dataclass(frozen=True, slots=True)
class BigipGtmListener:
    """A ``gtm listener`` object — DNS listener, GTM equivalent of
    ``ltm virtual``."""

    name: str
    full_path: str
    description: str = ""
    address: str = ""
    port: str = ""
    ip_protocol: str = ""
    mask: str = ""
    pool: str = ""  # PathRef → gtm pool / ltm pool (DNS-Express)
    profiles: tuple[str, ...] = ()  # PathRefs → ltm profile
    rules: tuple[str, ...] = ()  # PathRefs → ltm rule / gtm rule
    source_address_translation: str = ""
    state: str = ""  # enabled/disabled
    vlans: tuple[str, ...] = ()  # PathRefs → net vlan
    vlans_disabled: bool = False
    vlans_enabled: bool = False
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmListenerDohProxy:
    """A ``gtm listener-doh-proxy`` object — DNS-over-HTTPS proxy listener."""

    name: str
    full_path: str
    description: str = ""
    address: str = ""
    port: str = ""
    pool: str = ""  # PathRef → gtm pool / ltm pool
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmListenerDohServer:
    """A ``gtm listener-doh-server`` object — DOH server listener."""

    name: str
    full_path: str
    description: str = ""
    address: str = ""
    port: str = ""
    pool: str = ""  # PathRef → gtm pool / ltm pool
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmLink:
    """A ``gtm link`` object — uplink-bandwidth tracker."""

    name: str
    full_path: str
    description: str = ""
    datacenter: str = ""  # PathRef → gtm datacenter
    monitor: str = ""
    prober_pool: str = ""  # PathRef → gtm prober-pool
    state: str = ""  # enabled/disabled
    weight: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmDistributedApp:
    """A ``gtm distributed-app`` object — multi-wideip app grouping."""

    name: str
    full_path: str
    description: str = ""
    wide_ips: tuple[str, ...] = ()  # PathRefs → gtm wideip
    persist_cidr: str = ""
    dependency_level: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmTopology:
    """A ``gtm topology`` object — a topology record.

    The header takes a multi-token condition rather than a normal
    full-path: ``gtm topology ldns: subnet 10.15.1.1/32 server:
    subnet 10.16.1.1/32 { ... }``.  We treat the entire condition
    string as the identifier; ``name`` and ``full_path`` both carry
    it verbatim.
    """

    name: str
    full_path: str
    description: str = ""
    order: str = ""
    score: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmGlobalSettingsGeneral:
    """The ``gtm global-settings general`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    auto_discovery: str = ""
    synchronization: str = ""
    synchronization_group_name: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmGlobalSettingsLoadBalancing:
    """The ``gtm global-settings load-balancing`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    topology_longest_match: str = ""
    ignore_path_ttl: str = ""
    respect_dependent_objects: str = ""
    verify_vs_availability: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmGlobalSettingsMetrics:
    """The ``gtm global-settings metrics`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    metrics_collection_protocols: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGtmGlobalSettingsMetricsExclusions:
    """The ``gtm global-settings metrics-exclusions`` singleton."""

    name: str = ""
    full_path: str = ""
    description: str = ""
    addresses: str = ""
    range: Range | None = None


# pem.* — typed projection for the Policy Enforcement Manager module.


@dataclass(frozen=True, slots=True)
class BigipPemPolicy:
    """A ``pem policy`` object — a subscriber-traffic policy.

    Rule bodies are kept as raw stanza names in ``rules`` so callers
    can interrogate which rules exist without modelling the full
    condition / action grammar.
    """

    name: str
    full_path: str
    description: str = ""
    rules: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemRule:
    """A ``pem irule`` object — a PEM-context iRule."""

    name: str
    full_path: str
    source: str = ""
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemListener:
    """A ``pem listener`` object — applies PEM to a set of virtual servers."""

    name: str
    full_path: str
    description: str = ""
    profile_spm: str = ""  # PathRef → pem profile spm
    profile_subscriber_mgmt: str = ""  # PathRef → pem profile subscriber-mgmt
    virtual_servers: tuple[str, ...] = ()  # PathRefs → ltm virtual
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemForwardingEndpoint:
    """A ``pem forwarding-endpoint`` object — a downstream forwarding target."""

    name: str
    full_path: str
    description: str = ""
    pool: str = ""  # PathRef → ltm pool
    snat_pool: str = ""  # PathRef → ltm snatpool
    source_ip: str = ""
    destination_ip: str = ""
    type_: str = ""
    persistence: str = ""
    translate_address: str = ""
    translate_service: str = ""
    fallback: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemInterceptionEndpoint:
    """A ``pem interception-endpoint`` object — an upstream tap destination."""

    name: str
    full_path: str
    description: str = ""
    pool: str = ""  # PathRef → ltm pool
    persistence: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemServiceChainEndpoint:
    """A ``pem service-chain-endpoint`` object — an ordered chain of endpoints."""

    name: str
    full_path: str
    description: str = ""
    service_endpoints: tuple[str, ...] = ()
    steering_policy: str = ""  # PathRef → pem policy
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemProfile:
    """A ``pem profile <type>`` object — bundles every PEM profile sub-type.

    The ``profile_type`` field carries the sub-type token
    (``diameter-endpoint``, ``radius-aaa``, ``spm``, ``subscriber-mgmt``)
    so callers can filter without reaching back into the kind string.
    """

    name: str
    full_path: str
    profile_type: str = ""
    defaults_from: str = ""  # PathRef → pem profile <same type>
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemRatingGroup:
    """A ``pem quota-mgmt rating-group`` object."""

    name: str
    full_path: str
    description: str = ""
    rating_group_id: str = ""
    default_quota: str = ""
    default_quota_holding_time: str = ""
    default_validity_time: str = ""
    default_threshold: str = ""
    total_octets: str = ""
    input_octets: str = ""
    output_octets: str = ""
    time: str = ""
    consumption_time: str = ""
    usage_time: str = ""
    volume: str = ""
    range: Range | None = None


# auth.* — typed projection for the authentication module.


@dataclass(frozen=True, slots=True)
class BigipAuthPartition:
    """An ``auth partition <name>`` object — administrative partition."""

    name: str
    full_path: str
    description: str = ""
    default_route_domain: str = ""
    inherited_traffic_group: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthUser:
    """An ``auth user <name>`` object — local user account.

    ``partition_access`` surfaces the keys of the ``partition-access
    { ... }`` sub-block (each key is a partition name or
    ``all-partitions``); the per-key ``role`` value is not modelled
    in v1.
    """

    name: str
    full_path: str
    description: str = ""
    partition: str = ""
    shell: str = ""
    encrypted_password: str = ""
    partition_access: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthPassword:
    """The ``auth password`` singleton."""

    name: str = ""
    full_path: str = ""
    expiration_warning: str = ""
    minimum_length: str = ""
    policy: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthPasswordPolicy:
    """The ``auth password-policy`` singleton."""

    name: str = ""
    full_path: str = ""
    expiration_warning: str = ""
    max_duration: str = ""
    max_login_failures: str = ""
    min_duration: str = ""
    minimum_length: str = ""
    minimum_regular_characters: str = ""
    password_memory: str = ""
    policy_enforcement: str = ""
    required_lowercase: str = ""
    required_numeric: str = ""
    required_special: str = ""
    required_uppercase: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthSource:
    """The ``auth source`` singleton — selects the authentication backend."""

    name: str = ""
    full_path: str = ""
    fallback: str = ""
    type_: str = ""  # ``local`` / ``radius`` / ``ldap`` / ``tacacs`` / …
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthRemoteRole:
    """The ``auth remote-role`` singleton.

    ``role_info`` surfaces the top-level keys of the ``role-info {
    ... }`` sub-block (one per remote-role entry); per-key
    attributes (``role``, ``console``, ``deny``, …) are not modelled
    in v1.
    """

    name: str = ""
    full_path: str = ""
    role_info: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthRemoteUser:
    """The ``auth remote-user`` singleton."""

    name: str = ""
    full_path: str = ""
    default_partition: str = ""
    default_role: str = ""
    remote_console_access: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthLoginFailures:
    """The ``auth login-failures`` singleton."""

    name: str = ""
    full_path: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthLdap:
    """An ``auth ldap <name>`` object — LDAP authentication profile."""

    name: str = ""
    full_path: str = ""
    bind_dn: str = ""
    bind_pw: str = ""
    bind_timeout: str = ""
    check_host_attr: str = ""
    check_roles_group: str = ""
    filter_: str = ""
    group_dn: str = ""
    group_member_attribute: str = ""
    idle_timeout: str = ""
    ignore_auth_info_unavail: str = ""
    ignore_unknown_user: str = ""
    login_attribute: str = ""
    port: str = ""
    scope: str = ""
    search_base_dn: str = ""
    search_timeout: str = ""
    servers: tuple[str, ...] = ()
    ssl: str = ""
    ssl_ca_cert: str = ""
    ssl_check_peer: str = ""
    ssl_client_cert: str = ""
    ssl_client_key: str = ""
    user_template: str = ""
    version: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthRadius:
    """An ``auth radius <name>`` object — RADIUS authentication profile."""

    name: str = ""
    full_path: str = ""
    service_type: str = ""
    servers: tuple[str, ...] = ()  # PathRefs → auth radius-server
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthRadiusServer:
    """An ``auth radius-server <name>`` object."""

    name: str = ""
    full_path: str = ""
    server: str = ""
    port: str = ""
    secret: str = ""
    timeout: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthTacacs:
    """An ``auth tacacs <name>`` object — TACACS+ authentication profile."""

    name: str = ""
    full_path: str = ""
    protocol: str = ""
    secret: str = ""
    service: str = ""
    servers: tuple[str, ...] = ()  # bare hostnames / IPs, not full-paths
    accounting: str = ""
    authentication: str = ""
    debug: str = ""
    encryption: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthCertLdap:
    """An ``auth cert-ldap <name>`` object — certificate-LDAP profile."""

    name: str = ""
    full_path: str = ""
    bind_dn: str = ""
    bind_pw: str = ""
    bind_timeout: str = ""
    idle_timeout: str = ""
    login_attribute: str = ""
    port: str = ""
    scope: str = ""
    search_base_dn: str = ""
    search_timeout: str = ""
    servers: tuple[str, ...] = ()
    ssl: str = ""
    user_template: str = ""
    version: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthApmAuth:
    """An ``auth apm-auth <name>`` object — APM auth profile binding."""

    name: str = ""
    full_path: str = ""
    profile: str = ""  # PathRef → apm policy access-policy
    range: Range | None = None


# Aggregate config inventory


@dataclass
class BigipConfig:
    """Complete parsed inventory of a BIG-IP configuration file."""

    data_groups: dict[str, BigipDataGroup] = field(default_factory=dict)
    pools: dict[str, BigipPool] = field(default_factory=dict)
    virtual_servers: dict[str, BigipVirtualServer] = field(default_factory=dict)
    virtual_addresses: dict[str, BigipVirtualAddress] = field(default_factory=dict)
    # Bundle 13 — ltm.* cross-cutting infra.
    ltm_cipher_groups: dict[str, BigipLtmCipherGroup] = field(default_factory=dict)
    ltm_cipher_rules: dict[str, BigipLtmCipherRule] = field(default_factory=dict)
    ltm_nats: dict[str, BigipLtmNat] = field(default_factory=dict)
    ltm_snats: dict[str, BigipLtmSnat] = field(default_factory=dict)
    ltm_snat_translations: dict[str, BigipLtmSnatTranslation] = field(default_factory=dict)
    ltm_policy_strategies: dict[str, BigipLtmPolicyStrategy] = field(default_factory=dict)
    ltm_traffic_classes: dict[str, BigipLtmTrafficClass] = field(default_factory=dict)
    ltm_traffic_matching_criteria: dict[str, BigipLtmTrafficMatchingCriteria] = field(
        default_factory=dict
    )
    ltm_ifiles: dict[str, BigipLtmIfile] = field(default_factory=dict)
    ltm_eviction_policies: dict[str, BigipLtmEvictionPolicy] = field(default_factory=dict)
    # Bundle 14 — ltm dns.* (DNS Express).
    ltm_dns_nameservers: dict[str, BigipLtmDnsNameserver] = field(default_factory=dict)
    ltm_dns_tsig_keys: dict[str, BigipLtmDnsTsigKey] = field(default_factory=dict)
    ltm_dns_zones: dict[str, BigipLtmDnsZone] = field(default_factory=dict)
    ltm_dns_dnssec_keys: dict[str, BigipLtmDnsDnssecKey] = field(default_factory=dict)
    ltm_dns_dnssec_zones: dict[str, BigipLtmDnsDnssecZone] = field(default_factory=dict)
    ltm_dns_cache_resolvers: dict[str, BigipLtmDnsCacheResolver] = field(default_factory=dict)
    ltm_dns_cache_transparent: dict[str, BigipLtmDnsCacheTransparent] = field(default_factory=dict)
    ltm_dns_cache_validating_resolvers: dict[str, BigipLtmDnsCacheValidatingResolver] = field(
        default_factory=dict
    )
    ltm_dns_cache_global_settings: dict[str, BigipLtmDnsCacheGlobalSettings] = field(
        default_factory=dict
    )
    # All five ``ltm dns cache records X`` sub-kinds merge into one
    # container keyed by full-path; ``record_kind`` disambiguates.
    ltm_dns_cache_records: dict[str, BigipLtmDnsCacheRecord] = field(default_factory=dict)
    ltm_dns_hpke_keys: dict[str, BigipLtmDnsHpkeKey] = field(default_factory=dict)
    ltm_dns_hpke_profiles: dict[str, BigipLtmDnsHpkeProfile] = field(default_factory=dict)
    ltm_dns_analytics_global_settings: dict[str, BigipLtmDnsAnalyticsGlobalSettings] = field(
        default_factory=dict
    )
    # Bundle 15 — ltm message-routing.* (20 kinds, four protocols).
    ltm_mr_diameter_peers: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_diameter_routes: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_diameter_profile_router: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    ltm_mr_diameter_profile_session: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    ltm_mr_diameter_transport_config: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    ltm_mr_sip_peers: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_sip_routes: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_sip_profile_router: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_sip_profile_session: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    ltm_mr_sip_transport_config: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    ltm_mr_mqtt_peers: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_mqtt_routes: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_mqtt_profile_router: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    ltm_mr_mqtt_profile_session: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    ltm_mr_mqtt_transport_config: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    ltm_mr_generic_peers: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_generic_protocols: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_generic_routes: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_generic_routers: dict[str, BigipLtmMessageRoutingObject] = field(default_factory=dict)
    ltm_mr_generic_transport_config: dict[str, BigipLtmMessageRoutingObject] = field(
        default_factory=dict
    )
    # Bundle 16 — ltm auth.* profiles (11 kinds).
    ltm_auth_profiles: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_ldap: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_radius: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_radius_servers: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_tacacs: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_crldp_servers: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_ocsp_responders: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_kerberos_delegations: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_ssl_cc_ldap: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_ssl_crldp: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    ltm_auth_ssl_ocsp: dict[str, BigipLtmAuthObject] = field(default_factory=dict)
    # Bundle 17 — ltm CGNAT / LSN (3 kinds).
    ltm_lsn_pools: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_lsn_log_profiles: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_alg_log_profiles: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    # Bundle 18 — ltm global-settings + misc singletons.
    ltm_default_node_monitor: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_global_settings_connection: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_global_settings_general: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_global_settings_rule: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_global_settings_traffic_control: dict[str, BigipLtmMinimalObject] = field(
        default_factory=dict
    )
    ltm_rule_profiler: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    # Bundle 19 — ltm classification + clientssl (11 kinds).
    ltm_classification_application: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_classification_auto_update_settings: dict[str, BigipLtmMinimalObject] = field(
        default_factory=dict
    )
    ltm_classification_category: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_classification_ce: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_classification_signature_update_schedule: dict[str, BigipLtmMinimalObject] = field(
        default_factory=dict
    )
    ltm_classification_url_cat_policy: dict[str, BigipLtmMinimalObject] = field(
        default_factory=dict
    )
    ltm_classification_url_category: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_classification_urldb_feed_list: dict[str, BigipLtmMinimalObject] = field(
        default_factory=dict
    )
    ltm_classification_urldb_file: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_clientssl_ocsp_stapling_responses: dict[str, BigipLtmMinimalObject] = field(
        default_factory=dict
    )
    ltm_clientssl_proxy_cached_certs: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    # Bundle 20 — ltm tacdb (3 kinds).
    ltm_tacdb_customdb: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_tacdb_customdb_file: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_tacdb_licenseddb: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    # Bundle 21 — net routing (10 kinds).
    net_routing_access_lists: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_routing_bfd: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_routing_bgp: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_routing_community_lists: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_routing_extcommunity_lists: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_routing_prefix_lists: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_routing_profile_bgp: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_routing_route_maps: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_routing_debug: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_router_advertisements: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    # Bundle 22 — net tunnels family (14 kinds).
    net_tunnels_endpoints: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_etherip: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_fec: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_geneve: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_gre: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_ipip: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_ipsec: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_lw4o6: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_map: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_ppp: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_tcp_forward: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_v6rd: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_vxlan: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_tunnels_wccp: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    # Bundle 23 — net ipsec (5 kinds).
    net_ipsec_ike_daemon: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_ipsec_ike_peers: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_ipsec_ipsec_policies: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_ipsec_manual_security_associations: dict[str, BigipNetMinimalObject] = field(
        default_factory=dict
    )
    net_ipsec_traffic_selectors: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    # Bundle 24 — net BWC / cos / rate-shaping (12 kinds).
    net_bwc_policies: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_bwc_priority_groups: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_bwc_traffic_groups: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_cos_global_settings: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_cos_map_8021p: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_cos_map_dscp: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_cos_traffic_priority: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_rate_shaping_class: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_rate_shaping_color_policer: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_rate_shaping_drop_policy: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_rate_shaping_queue: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_rate_shaping_shaping_policy: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    # Bundle 25 — net L2 / misc (22 kinds).
    net_address_lists: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_arp: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_dag_globals: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_fdb_tunnel: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_fdb_vlan: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_interface_cos: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_ipv6_subscriber_prefix_length: dict[str, BigipNetMinimalObject] = field(
        default_factory=dict
    )
    net_lacp_globals: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_lldp_globals: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_multicast_globals: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_ndp: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_packet_filter: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_packet_filter_trusted: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_port_mirror: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_rst_cause: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_self_allow: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_service_policy: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_stp_globals: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_timer_policy: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_trunk: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_vlan_group: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_wccp: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    # Bundle 26 — net sfc (2 kinds).
    net_sfc_chain: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    net_sfc_sf: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    # Bundles 27-31 — apm.* minimal kinds.
    apm_aaa_active_directory: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_active_directory_trusted_domains: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_aaa_crldp: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_endpoint_management_system: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_aaa_f5_mfa_configuration: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_f5_service_connector: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_http: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_http_connector_request: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_http_connector_transport: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_kerberos: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_kerberos_keytab_file: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_ldap: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_oam: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_oauth_provider: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_oauth_request: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_oauth_server: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_ocsp: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_okta_connector: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_radius: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_saml: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_saml_idp_automation: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_saml_idp_connector: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_securid: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_aaa_tacacsplus: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_profile_access: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_profile_connectivity: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_profile_exchange: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_profile_oauth: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_profile_vdi: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_basic: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_form_based: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_form_basedv2: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_kerberos: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_ntlmv1: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_ntlmv2: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_oauth_bearer: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_saml: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_saml_resource: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_saml_sp_automation: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_sso_saml_sp_connector: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_address_space: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_app_tunnel: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_client_rate_class: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_client_traffic_classifier: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_resource_ipv6_leasepool: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_leasepool: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_network_access: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_portal_access: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_remote_desktop_citrix: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_resource_remote_desktop_citrix_client_bundle: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_resource_remote_desktop_citrix_client_package_file: dict[str, BigipApmMinimalObject] = (
        field(default_factory=dict)
    )
    apm_resource_remote_desktop_quest: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_resource_remote_desktop_rdp: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_remote_desktop_vmware_view: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_resource_sandbox: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_webtop: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_resource_webtop_link: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_oauth_jwk_config: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_oauth_jwt_config: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_oauth_jwt_provider_list: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_oauth_oauth_claim: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_oauth_oauth_client_app: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_oauth_oauth_resource_server: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_oauth_oauth_scope: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_saml_artifact_resolution_service: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_saml_attribute_consuming_service: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_saml_auth_context_class_list: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_ntlm_machine_account: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_ntlm_ntlm_auth: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_acl: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_log_setting: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_url_filter: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_swg_scheme: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_client_image: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_configuration_captcha: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_epsec_epsec_package: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_apm_avr_config: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_report_custom_report_field: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_policy_customization_group: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_policy_customization_languages: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    apm_policy_image_file: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    apm_policy_windows_group_policy_file: dict[str, BigipApmMinimalObject] = field(
        default_factory=dict
    )
    nodes: dict[str, BigipNode] = field(default_factory=dict)
    profiles: dict[str, BigipProfile] = field(default_factory=dict)
    monitors: dict[str, BigipMonitor] = field(default_factory=dict)
    snat_pools: dict[str, BigipSnatPool] = field(default_factory=dict)
    persistence: dict[str, BigipPersistence] = field(default_factory=dict)
    # LTM iRules only.  GTM iRules live in ``gtm_rules`` so a tenant
    # with the same partition path in both modules does not collide.
    rules: dict[str, BigipRule] = field(default_factory=dict)
    policies: dict[str, BigipPolicy] = field(default_factory=dict)
    # net.* — typed projection for the network module.
    net_routes: dict[str, BigipNetRoute] = field(default_factory=dict)
    net_vlans: dict[str, BigipNetVlan] = field(default_factory=dict)
    net_selves: dict[str, BigipNetSelf] = field(default_factory=dict)
    net_route_domains: dict[str, BigipNetRouteDomain] = field(default_factory=dict)
    net_port_lists: dict[str, BigipNetPortList] = field(default_factory=dict)
    net_interfaces: dict[str, BigipNetInterface] = field(default_factory=dict)
    net_dns_resolvers: dict[str, BigipNetDnsResolver] = field(default_factory=dict)
    net_tunnels: dict[str, BigipNetTunnel] = field(default_factory=dict)
    net_stps: dict[str, BigipNetStp] = field(default_factory=dict)
    # sys.* — singletons live under the empty-string key.
    sys_dns: dict[str, BigipSysDns] = field(default_factory=dict)
    sys_ntp: dict[str, BigipSysNtp] = field(default_factory=dict)
    sys_snmp: dict[str, BigipSysSnmp] = field(default_factory=dict)
    sys_global_settings: dict[str, BigipSysGlobalSettings] = field(default_factory=dict)
    sys_provisions: dict[str, BigipSysProvision] = field(default_factory=dict)
    sys_folders: dict[str, BigipSysFolder] = field(default_factory=dict)
    sys_file_ssl_certs: dict[str, BigipSysFileSslCert] = field(default_factory=dict)
    sys_file_ssl_keys: dict[str, BigipSysFileSslKey] = field(default_factory=dict)
    sys_management_routes: dict[str, BigipSysManagementRoute] = field(default_factory=dict)
    # Bundles 33-41 — sys.* minimal kinds.
    sys_ha_group: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_application_service: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_application_template: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_application_apl_script: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_application_custom_stat: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_autoscale_group: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_db: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_httpd: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_sshd: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_syslog: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_outbound_smtp: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_smtp_server: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_feature_module: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_console: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_log_rotate: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_ucs: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_url_db_download_schedule: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_url_db_url_category: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_data_group: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_external_monitor: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_ifile: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_rewrite_rule: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_apache_ssl_cert: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_ssl_crl: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_lwtunneltbl: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_browser_capabilities_db: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_file_device_capabilities_db: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_log_config_destination_alertd: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_log_config_destination_arcsight: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_log_config_destination_ipfix: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_log_config_destination_local_database: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_log_config_destination_local_syslog: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_log_config_destination_management_port: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_log_config_destination_remote_high_speed_log: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_log_config_destination_remote_syslog: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_log_config_destination_splunk: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_log_config_filter: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_log_config_publisher: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_daemon_log_settings_clusterd: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_daemon_log_settings_csyncd: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_daemon_log_settings_icr_eventd: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_daemon_log_settings_icrd: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_daemon_log_settings_lind: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_daemon_log_settings_mcpd: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_daemon_log_settings_tmm: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_cert: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_key: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_crl: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_csr: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_master_key: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_cert_order_manager: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_ca_bundle_manager: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_cert_validator_crl: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_cert_validator_ocsp: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_cert_validation_response_ocsp: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_crypto_client: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_server: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_acceleration_strategy: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_fips_key: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_crypto_fips_external_hsm: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_ipfix_destination: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_ipfix_element: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_ipfix_irules: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_icall_handler_periodic: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_icall_handler_perpetual: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_icall_handler_triggered: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_icall_script: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_icall_istats_trigger: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_management_dhcp: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_management_ip: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_management_ovsdb: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_management_proxy_config: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_state_mirroring: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_datastor: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_sflow_receiver: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_sflow_global_settings_http: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_sflow_global_settings_interface: dict[str, BigipSysMinimalObject] = field(
        default_factory=dict
    )
    sys_sflow_global_settings_system: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_sflow_global_settings_vlan: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_software_hotfix: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_software_image: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_software_signature: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_software_volume: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_alert_lcd: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_aom: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_appiq_config: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_cluster: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_config: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_default_config: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_failover: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_internal_proxy: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_traffic: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_tmm_traffic: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_turboflex_profile_config: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_fpga_firmware_config: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    # security.* — AFM / DDoS / inspection / device-id.
    security_firewall_port_lists: dict[str, BigipSecurityFirewallPortList] = field(
        default_factory=dict
    )
    security_firewall_rule_lists: dict[str, BigipSecurityFirewallRuleList] = field(
        default_factory=dict
    )
    security_firewall_config_entity_ids: dict[str, BigipSecurityFirewallConfigEntityId] = field(
        default_factory=dict
    )
    security_firewall_policies: dict[str, BigipSecurityFirewallPolicy] = field(default_factory=dict)
    security_firewall_address_lists: dict[str, BigipSecurityFirewallAddressList] = field(
        default_factory=dict
    )
    security_firewall_global_rules: dict[str, BigipSecurityFirewallGlobalRules] = field(
        default_factory=dict
    )
    security_firewall_management_ip_rules: dict[str, BigipSecurityFirewallManagementIpRules] = (
        field(default_factory=dict)
    )
    security_firewall_schedules: dict[str, BigipSecurityFirewallSchedule] = field(
        default_factory=dict
    )
    security_firewall_user_lists: dict[str, BigipSecurityFirewallUserList] = field(
        default_factory=dict
    )
    security_firewall_user_domains: dict[str, BigipSecurityFirewallUserDomain] = field(
        default_factory=dict
    )
    security_firewall_global_fqdn_policy: dict[str, BigipSecurityFirewallGlobalFqdnPolicy] = field(
        default_factory=dict
    )
    security_firewall_port_misuse_policies: dict[str, BigipSecurityFirewallPortMisusePolicy] = (
        field(default_factory=dict)
    )
    security_firewall_on_demand_compilation: dict[str, BigipSecurityFirewallOnDemandCompilation] = (
        field(default_factory=dict)
    )
    security_firewall_on_demand_rule_deploy: dict[str, BigipSecurityFirewallOnDemandRuleDeploy] = (
        field(default_factory=dict)
    )
    security_firewall_uuid_default_autogenerate: dict[
        str, BigipSecurityFirewallUuidDefaultAutogenerate
    ] = field(default_factory=dict)
    security_firewall_config_change_log: dict[str, BigipSecurityFirewallConfigChangeLog] = field(
        default_factory=dict
    )
    # Bundle 10a — high-value security.* outside firewall.*.
    security_nat_policies: dict[str, BigipSecurityNatPolicy] = field(default_factory=dict)
    security_nat_source_translations: dict[str, BigipSecurityNatSourceTranslation] = field(
        default_factory=dict
    )
    security_nat_destination_translations: dict[str, BigipSecurityNatDestinationTranslation] = (
        field(default_factory=dict)
    )
    security_log_profiles: dict[str, BigipSecurityLogProfile] = field(default_factory=dict)
    security_dos_profiles: dict[str, BigipSecurityDosProfile] = field(default_factory=dict)
    security_ip_intelligence_feed_lists: dict[str, BigipSecurityIpIntelligenceFeedList] = field(
        default_factory=dict
    )
    security_ip_intelligence_global_policy: dict[str, BigipSecurityIpIntelligenceGlobalPolicy] = (
        field(default_factory=dict)
    )
    security_zones: dict[str, BigipSecurityZone] = field(default_factory=dict)
    security_protected_zones: dict[str, BigipSecurityProtectedZone] = field(default_factory=dict)
    security_packet_filter_policies: dict[str, BigipSecurityPacketFilterPolicy] = field(
        default_factory=dict
    )
    security_packet_filter_default_rules: dict[str, BigipSecurityPacketFilterDefaultRules] = field(
        default_factory=dict
    )
    security_ssh_profiles: dict[str, BigipSecuritySshProfile] = field(default_factory=dict)
    security_http_profiles: dict[str, BigipSecurityHttpProfile] = field(default_factory=dict)
    security_bot_defense_profiles: dict[str, BigipSecurityBotDefenseProfile] = field(
        default_factory=dict
    )
    # Bundle 10b — minimal security.* projections sharing
    # ``BigipSecurityMinimalObject`` for shape uniformity.
    security_analytics_settings: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_anti_fraud_profiles: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_anti_fraud_signatures_update: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_blacklist_publisher_categories: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_blacklist_publisher_profiles: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_bot_defense_signatures: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_bot_defense_signature_categories: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_cloud_services_connectors: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_datasync_background_tasks: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_datasync_global_profiles: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_datasync_local_profiles: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_debug_drop_redirect_stats: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_debug_matcher: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_debug_register: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_device_device_context: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_autodos_file_objects: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_behavioral_signatures: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_bot_signatures: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_dos_bot_signature_categories: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_device_config: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_dos_dns_nxdomain_stat: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_dos_signatures: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_dos_dynamic_signatures: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_ip_uncommon_protolists: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_l4bdos_file_objects: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_network_whitelists: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_dos_stress_stats: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_dos_udp_portlists: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_dos_virtuals: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_flowspec_route_injector_profiles: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_ip_intelligence_blacklist_categories: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_protocol_inspection_common_config: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_protocol_inspection_learning_stats: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_protocol_inspection_profiles: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_protocol_inspection_signatures: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_scrubber_profiles: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_ssh_ciphers: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    security_ip_intelligence_policies: dict[str, BigipSecurityIpIntelligencePolicy] = field(
        default_factory=dict
    )
    security_pi_compliance_maps: dict[str, BigipSecurityProtocolInspectionComplianceMap] = field(
        default_factory=dict
    )
    security_pi_compliance_objects: dict[str, BigipSecurityProtocolInspectionComplianceObject] = (
        field(default_factory=dict)
    )
    security_device_id_attributes: dict[str, BigipSecurityDeviceIdAttribute] = field(
        default_factory=dict
    )
    # apm.* — Access Policy Manager.
    apm_ephemeral_auth_ssh_security_configs: dict[str, BigipApmEphemeralAuthSshSecurityConfig] = (
        field(default_factory=dict)
    )
    apm_oauth_db_instances: dict[str, BigipApmOauthDbInstance] = field(default_factory=dict)
    apm_policy_access_policies: dict[str, BigipApmPolicyAccessPolicy] = field(default_factory=dict)
    apm_policy_customization_sources: dict[str, BigipApmPolicyCustomizationSource] = field(
        default_factory=dict
    )
    apm_policy_items: dict[str, BigipApmPolicyItem] = field(default_factory=dict)
    # All three ``apm policy agent <type>`` sub-kinds (``ending-allow``,
    # ``ending-deny``, ``kerberos``) merge into this single container.
    # TMSH enforces full-path uniqueness across the sub-kinds, so the
    # dict key is unambiguous; the ``agent_type`` field on each value
    # distinguishes which sub-kind a row came from.
    apm_policy_agents: dict[str, BigipApmPolicyAgent] = field(default_factory=dict)
    apm_report_default_report: dict[str, BigipApmReportDefaultReport] = field(default_factory=dict)
    # cm.* — cluster / trust / traffic-group state.
    cm_certs: dict[str, BigipCmCert] = field(default_factory=dict)
    cm_keys: dict[str, BigipCmKey] = field(default_factory=dict)
    cm_devices: dict[str, BigipCmDevice] = field(default_factory=dict)
    cm_device_groups: dict[str, BigipCmDeviceGroup] = field(default_factory=dict)
    cm_traffic_groups: dict[str, BigipCmTrafficGroup] = field(default_factory=dict)
    cm_trust_domains: dict[str, BigipCmTrustDomain] = field(default_factory=dict)
    # gtm.* — Global Traffic Manager / DNS load-balancing state.
    gtm_datacenters: dict[str, BigipGtmDatacenter] = field(default_factory=dict)
    gtm_servers: dict[str, BigipGtmServer] = field(default_factory=dict)
    # All six ``gtm pool <record-type>`` (a, aaaa, cname, mx, srv,
    # naptr) variants merge into this single container; same for
    # ``gtm_wideips`` below.  TMSH enforces full-path uniqueness
    # across the variants (a config can't carry both ``gtm pool a /X``
    # and ``gtm pool aaaa /X``), so the dict key is unambiguous; the
    # ``record_type`` field disambiguates within each row.
    gtm_pools: dict[str, BigipGtmPool] = field(default_factory=dict)
    gtm_wideips: dict[str, BigipGtmWideip] = field(default_factory=dict)
    gtm_prober_pools: dict[str, BigipGtmProberPool] = field(default_factory=dict)
    gtm_regions: dict[str, BigipGtmRegion] = field(default_factory=dict)
    gtm_rules: dict[str, BigipGtmRule] = field(default_factory=dict)
    # ``gtm monitor <type>`` lands here — kept separate from
    # ``monitors`` (which holds ltm) so a config with the same path
    # under both modules doesn't collide.  All 31 protocol variants
    # share the ``BigipMonitor`` dataclass; ``monitor_type``
    # disambiguates.
    gtm_monitors: dict[str, BigipMonitor] = field(default_factory=dict)
    # Bundle 12 — gtm listeners / link / topology / distributed-app /
    # global-settings singletons.
    gtm_listeners: dict[str, BigipGtmListener] = field(default_factory=dict)
    gtm_listener_doh_proxies: dict[str, BigipGtmListenerDohProxy] = field(default_factory=dict)
    gtm_listener_doh_servers: dict[str, BigipGtmListenerDohServer] = field(default_factory=dict)
    gtm_links: dict[str, BigipGtmLink] = field(default_factory=dict)
    gtm_topologies: dict[str, BigipGtmTopology] = field(default_factory=dict)
    gtm_distributed_apps: dict[str, BigipGtmDistributedApp] = field(default_factory=dict)
    gtm_global_settings_general: dict[str, BigipGtmGlobalSettingsGeneral] = field(
        default_factory=dict
    )
    gtm_global_settings_load_balancing: dict[str, BigipGtmGlobalSettingsLoadBalancing] = field(
        default_factory=dict
    )
    gtm_global_settings_metrics: dict[str, BigipGtmGlobalSettingsMetrics] = field(
        default_factory=dict
    )
    gtm_global_settings_metrics_exclusions: dict[str, BigipGtmGlobalSettingsMetricsExclusions] = (
        field(default_factory=dict)
    )
    # pem.* — Policy Enforcement Manager (subscriber policy).
    pem_policies: dict[str, BigipPemPolicy] = field(default_factory=dict)
    pem_rules: dict[str, BigipPemRule] = field(default_factory=dict)
    pem_listeners: dict[str, BigipPemListener] = field(default_factory=dict)
    pem_forwarding_endpoints: dict[str, BigipPemForwardingEndpoint] = field(default_factory=dict)
    pem_interception_endpoints: dict[str, BigipPemInterceptionEndpoint] = field(
        default_factory=dict
    )
    pem_service_chain_endpoints: dict[str, BigipPemServiceChainEndpoint] = field(
        default_factory=dict
    )
    pem_profiles: dict[str, BigipPemProfile] = field(default_factory=dict)
    pem_rating_groups: dict[str, BigipPemRatingGroup] = field(default_factory=dict)
    # Bundle 32 — pem.* minimal kinds.
    pem_gs_analytics: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_gs_gx: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_gs_hsl_flow: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_gs_hsl_report: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_gs_insert_content: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_gs_policy: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_gs_quota_mgmt: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_gs_session_mgmt_attributes: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_gs_subscriber_activity_log: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_protocol_diameter_avp: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_protocol_radius_avp: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_protocol_profile_gx: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_protocol_profile_radius: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_reporting_format_script: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_subscriber: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    pem_subscriber_attribute: dict[str, BigipPemMinimalObject] = field(default_factory=dict)
    # auth.* — administrative partitions, users, and authentication
    # back-ends.  Singletons (``auth password``, ``auth password-policy``,
    # ``auth source``, ``auth remote-role``, ``auth remote-user``,
    # ``auth login-failures``) live under the empty-string key.
    auth_partitions: dict[str, BigipAuthPartition] = field(default_factory=dict)
    auth_users: dict[str, BigipAuthUser] = field(default_factory=dict)
    auth_password: dict[str, BigipAuthPassword] = field(default_factory=dict)
    auth_password_policy: dict[str, BigipAuthPasswordPolicy] = field(default_factory=dict)
    auth_source: dict[str, BigipAuthSource] = field(default_factory=dict)
    auth_remote_role: dict[str, BigipAuthRemoteRole] = field(default_factory=dict)
    auth_remote_user: dict[str, BigipAuthRemoteUser] = field(default_factory=dict)
    auth_login_failures: dict[str, BigipAuthLoginFailures] = field(default_factory=dict)
    auth_ldaps: dict[str, BigipAuthLdap] = field(default_factory=dict)
    auth_radius: dict[str, BigipAuthRadius] = field(default_factory=dict)
    auth_radius_servers: dict[str, BigipAuthRadiusServer] = field(default_factory=dict)
    auth_tacacs: dict[str, BigipAuthTacacs] = field(default_factory=dict)
    auth_cert_ldaps: dict[str, BigipAuthCertLdap] = field(default_factory=dict)
    auth_apm_auths: dict[str, BigipAuthApmAuth] = field(default_factory=dict)
    # Bundle 42 — vcmp.* minimal kinds.
    vcmp_guests: dict[str, BigipVcmpMinimalObject] = field(default_factory=dict)
    vcmp_traffic_profiles: dict[str, BigipVcmpMinimalObject] = field(default_factory=dict)
    vcmp_virtual_disks: dict[str, BigipVcmpMinimalObject] = field(default_factory=dict)
    vcmp_virtual_disk_templates: dict[str, BigipVcmpMinimalObject] = field(default_factory=dict)
    # Bundle 43 — cm.* follow-ons.
    cm_ha_groups: dict[str, BigipCmMinimalObject] = field(default_factory=dict)
    cm_config_sync: dict[str, BigipCmMinimalObject] = field(default_factory=dict)
    # Bundle 44 — cli.* minimal kinds.
    cli_admin_partitions: dict[str, BigipCliMinimalObject] = field(default_factory=dict)
    cli_alias_private: dict[str, BigipCliMinimalObject] = field(default_factory=dict)
    cli_alias_shared: dict[str, BigipCliMinimalObject] = field(default_factory=dict)
    cli_global_settings: dict[str, BigipCliMinimalObject] = field(default_factory=dict)
    cli_preference: dict[str, BigipCliMinimalObject] = field(default_factory=dict)
    cli_script: dict[str, BigipCliMinimalObject] = field(default_factory=dict)
    cli_transaction: dict[str, BigipCliMinimalObject] = field(default_factory=dict)
    cli_version: dict[str, BigipCliMinimalObject] = field(default_factory=dict)
    # Bundle 45 — api-protection.* minimal kinds.
    api_protection_profile_apiprotection: dict[str, BigipApiProtectionMinimalObject] = field(
        default_factory=dict
    )
    api_protection_response: dict[str, BigipApiProtectionMinimalObject] = field(
        default_factory=dict
    )
    api_protection_server: dict[str, BigipApiProtectionMinimalObject] = field(default_factory=dict)
    # Audit follow-up — kinds found in real BIG-IP configs.
    # ltm html-rule.* (7 subtypes).
    ltm_html_rule_comment_raise_event: dict[str, BigipLtmMinimalObject] = field(
        default_factory=dict
    )
    ltm_html_rule_comment_remove: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_html_rule_tag_append_html: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_html_rule_tag_prepend_html: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_html_rule_tag_raise_event: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_html_rule_tag_remove: dict[str, BigipLtmMinimalObject] = field(default_factory=dict)
    ltm_html_rule_tag_remove_attribute: dict[str, BigipLtmMinimalObject] = field(
        default_factory=dict
    )
    # security shared-objects.* (2).
    security_shared_objects_port_lists: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    security_shared_objects_address_lists: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    # security dos ipv6-ext-hdr (separate from bundle 10b dos kinds).
    security_dos_ipv6_ext_hdr: dict[str, BigipSecurityMinimalObject] = field(default_factory=dict)
    # sys.* follow-ons.
    sys_ecm_cloud_provider: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_software_update: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_dynad_settings: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_compatibility_level: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    sys_diags_ihealth: dict[str, BigipSysMinimalObject] = field(default_factory=dict)
    # apm follow-on.
    apm_client_packaging: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    # New module — asm.*  (Application Security Manager).
    asm_policies: dict[str, BigipAsmMinimalObject] = field(default_factory=dict)
    # New module — ilx.*  (iRulesLX).
    ilx_global_settings: dict[str, BigipIlxMinimalObject] = field(default_factory=dict)
    # New module — wom.*  (WAN Optimization Manager, legacy but in
    # real configs).
    wom_endpoint_discovery: dict[str, BigipWomMinimalObject] = field(default_factory=dict)
    # Sibling-completeness follow-ups (HOL-2571 + BigIPReport + sslo
    # .scf corpus scan).  Distinct from look-alike kinds already
    # captured under different modules (e.g. ``ltm dns analytics
    # global-settings``).
    net_routing_as_paths: dict[str, BigipNetMinimalObject] = field(default_factory=dict)
    security_dos_profile_signatures: dict[str, BigipSecurityMinimalObject] = field(
        default_factory=dict
    )
    apm_aaa_localdb: dict[str, BigipApmMinimalObject] = field(default_factory=dict)
    # New module — analytics.*  (top-level analytics global-settings,
    # not the ``ltm dns analytics global-settings`` singleton).
    analytics_global_settings: dict[str, BigipAnalyticsMinimalObject] = field(default_factory=dict)
    generic_objects: dict[str, BigipGenericObject] = field(default_factory=dict)

    def resolve_name(self, name: str, objects: Mapping[str, object]) -> str | None:
        """Resolve a possibly-short name to a full path in *objects*.

        BIG-IP configs use full paths like ``/Common/my_pool`` but iRules
        may reference just ``my_pool``.  This tries exact match first, then
        falls back to a suffix match.
        """
        if name in objects:
            return name
        # Try with /Common/ prefix
        candidate = f"/Common/{name}"
        if candidate in objects:
            return candidate
        # Suffix match: look for any key ending with /<name>
        suffix = f"/{name}"
        for key in objects:
            if key.endswith(suffix):
                return key
        return None

    def resolve_pool(self, name: str) -> str | None:
        return self.resolve_name(name, self.pools)

    def resolve_data_group(self, name: str) -> str | None:
        return self.resolve_name(name, self.data_groups)

    def resolve_snat_pool(self, name: str) -> str | None:
        return self.resolve_name(name, self.snat_pools)

    def resolve_persistence(self, name: str) -> str | None:
        return self.resolve_name(name, self.persistence)

    def resolve_rule(self, name: str) -> str | None:
        return self.resolve_name(name, self.rules)

    def resolve_profile(self, name: str) -> str | None:
        return self.resolve_name(name, self.profiles)

    def resolve_generic_object(
        self,
        name: str,
        *,
        module: str | None = None,
        object_types: tuple[str, ...] | None = None,
    ) -> str | None:
        """Resolve a generic BIG-IP object key by identifier/name."""
        clean = name.strip()
        if not clean:
            return None

        def _matches(obj: BigipGenericObject) -> bool:
            if module is not None and obj.module != module:
                return False
            if object_types is not None and obj.object_type not in object_types:
                return False
            ident = obj.identifier
            if ident == clean:
                return True
            if clean.startswith("/") and ident.endswith(clean):
                return True
            if not clean.startswith("/"):
                if ident.endswith(f"/{clean}") or ident == clean:
                    return True
            return False

        for key, obj in self.generic_objects.items():
            if _matches(obj):
                return key
        return None

    def profiles_for_virtual(self, vs_name: str) -> list[BigipProfile]:
        """Return resolved profile objects attached to a virtual server."""
        vs = self.virtual_servers.get(vs_name)
        if vs is None:
            return []
        result: list[BigipProfile] = []
        for pref in vs.profiles:
            resolved = self.resolve_profile(pref)
            if resolved and resolved in self.profiles:
                result.append(self.profiles[resolved])
        return result

    def profile_types_for_virtual(self, vs_name: str) -> frozenset[ProfileType]:
        """Return the set of profile types attached to a virtual server."""
        return frozenset(p.profile_type for p in self.profiles_for_virtual(vs_name))
