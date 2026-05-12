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
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPoolMember:
    """A single member entry inside a ``ltm pool``."""

    name: str  # e.g. "/Common/10.0.0.1:80"
    address: str = ""
    port: int = 0
    monitor: str = ""


@dataclass(frozen=True, slots=True)
class BigipPool:
    """A ``ltm pool`` object."""

    name: str
    full_path: str
    module: str = "ltm"
    members: tuple[BigipPoolMember, ...] = ()
    monitor: str = ""
    load_balancing_mode: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNode:
    """A ``ltm node`` object."""

    name: str
    full_path: str
    address: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipProfile:
    """A ``ltm profile <type>`` object."""

    name: str
    full_path: str
    profile_type: ProfileType = ProfileType.OTHER
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipMonitor:
    """A ``ltm monitor <type>`` object."""

    name: str
    full_path: str
    monitor_type: str = ""  # "http", "tcp", "https", etc.
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSnatPool:
    """A ``ltm snatpool`` object."""

    name: str
    full_path: str
    members: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPersistence:
    """A ``ltm persistence <type>`` object."""

    name: str
    full_path: str
    persistence_type: str = ""  # "cookie", "source-addr", "ssl", etc.
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipRule:
    """A ``ltm rule`` object — an iRule embedded in the config."""

    name: str
    full_path: str
    source: str = ""  # the raw Tcl body
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
    pool_range: Range | None = None
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
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetVlan:
    """A ``net vlan`` object."""

    name: str
    full_path: str
    tag: int = 0
    interfaces: tuple[str, ...] = ()  # untagged interface names ("1.1", "1.2")
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
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetRouteDomain:
    """A ``net route-domain`` object."""

    name: str
    full_path: str
    id: int = 0
    vlans: tuple[str, ...] = ()  # member VLAN full-paths
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetPortList:
    """A ``net port-list`` object — used by self-allow and policy rules."""

    name: str
    full_path: str
    ports: tuple[str, ...] = ()  # raw port specs (e.g. ``80``, ``1029-1043``)
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
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNetDnsResolver:
    """A ``net dns-resolver`` object."""

    name: str
    full_path: str
    route_domain: str = ""  # full-path of bound route-domain
    forward_zones: tuple[str, ...] = ()  # top-level zone names only
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
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysFolder:
    """A ``sys folder`` object — partition / folder metadata."""

    name: str
    full_path: str
    device_group: str = ""
    traffic_group: str = ""
    hidden: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysFileSslCert:
    """A ``sys file ssl-cert`` object."""

    name: str
    full_path: str
    source_path: str = ""
    cache_path: str = ""
    revision: str = ""
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
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityFirewallConfigEntityId:
    """A ``security firewall config-entity-id`` object."""

    name: str
    full_path: str
    entity_id: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityIpIntelligencePolicy:
    """A ``security ip-intelligence policy`` object.

    The body is typically empty (the policy is referenced from other
    contexts); we just surface name and full-path.
    """

    name: str
    full_path: str
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityProtocolInspectionComplianceMap:
    """A ``security protocol-inspection compliance-map`` object."""

    name: str
    full_path: str
    insp_id: str = ""
    key_type: str = ""
    value_type: str = ""
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
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSecurityDeviceIdAttribute:
    """A ``security device-id attribute`` object."""

    name: str
    full_path: str
    id_: str = ""
    range: Range | None = None


# Aggregate config inventory


@dataclass
class BigipConfig:
    """Complete parsed inventory of a BIG-IP configuration file."""

    data_groups: dict[str, BigipDataGroup] = field(default_factory=dict)
    pools: dict[str, BigipPool] = field(default_factory=dict)
    virtual_servers: dict[str, BigipVirtualServer] = field(default_factory=dict)
    nodes: dict[str, BigipNode] = field(default_factory=dict)
    profiles: dict[str, BigipProfile] = field(default_factory=dict)
    monitors: dict[str, BigipMonitor] = field(default_factory=dict)
    snat_pools: dict[str, BigipSnatPool] = field(default_factory=dict)
    persistence: dict[str, BigipPersistence] = field(default_factory=dict)
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
