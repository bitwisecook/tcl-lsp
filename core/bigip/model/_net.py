"""Typed projection for the network module (``net.*``).

Routes, VLANs, self-IPs, route-domains, port-lists, interfaces,
DNS resolvers, tunnels, STP.  Long-tail ``net.*`` kinds share
:class:`BigipMinimalObject` via :data:`BigipNetMinimalObject`.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from ...analysis.semantic_model import Range

if TYPE_CHECKING:
    from ..types import IPAddress, Network

# ── net.* — typed projection for the network module ─────────────────


@dataclass(frozen=True, slots=True)
class BigipNetRoute:
    """A ``net route`` object — a routing-table entry.

    ``network`` / ``gw`` are typed values; the parser populates them
    by calling :meth:`Network.try_parse` / :meth:`IPAddress.try_parse`
    against the raw TMSH property, so consumers don't re-parse the
    string.  The special ``"default"`` route surfaces as
    ``is_default_route=True`` with ``network=None``.
    """

    name: str
    full_path: str
    network: "Network | None" = None
    is_default_route: bool = False  # ``network default`` route, no CIDR
    gw: "IPAddress | None" = None
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
    """A ``net self`` object — a self IP bound to a VLAN.

    ``address`` is a typed :class:`Network` populated by the parser
    (self-IPs carry both host + prefix in one field).
    """

    name: str
    full_path: str
    address: "Network | None" = None  # ``10.0.0.1/24`` typed
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
