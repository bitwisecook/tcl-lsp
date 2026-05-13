"""Typed projection for the GTM / DNS module (``gtm.*``).

Datacenters, servers, pools, wide-IPs, prober pools, regions,
rules, listeners (incl. DoH proxy/server), links, distributed
apps, topology rules, global-settings singletons.
"""

from __future__ import annotations

from dataclasses import dataclass

from ...analysis.semantic_model import Range


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
