"""Typed projection for the LTM module (``ltm.*``).

Covers the cross-cutting LTM kinds: cipher groups/rules, NAT /
SNAT, traffic classes, DNS Express, message routing, LTM auth
profiles, and the shared minimal fallback for the long-tail
kinds (CGNAT / LSN, global-settings singletons, classification,
URL-DB, tacdb).
"""

from __future__ import annotations

from dataclasses import dataclass

from ...analysis.semantic_model import Range

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
