"""Project a :class:`BigipConfig` into the tree shape the query DSL navigates.

The DSL exposes the parsed config as a nested mapping::

    .ltm.virtual["/Common/web_vs"].pool   -> PathRef("/Common/web_pool")
    .ltm.pool["/Common/web_pool"].members -> list[MemberObjectRef]
    .ltm.rule["/Common/r1"].body          -> string

This module builds those projections lazily.  Walking through containers
allocates lightweight :class:`Container` objects on demand; fully
projected :class:`.values.ObjectRef` instances are cached on the root.

The TMSH-spelt keys exposed here are the *user-visible* names.  They
map back to the underlying dataclass attribute names through
:data:`_KIND_FIELD_MAPS` for every supported object kind.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any

from ..model import (
    BigipApmEphemeralAuthSshSecurityConfig,
    BigipApmMinimalObject,
    BigipApmOauthDbInstance,
    BigipApmPolicyAccessPolicy,
    BigipApmPolicyAgent,
    BigipApmPolicyCustomizationSource,
    BigipApmPolicyItem,
    BigipApmReportDefaultReport,
    BigipAuthApmAuth,
    BigipAuthCertLdap,
    BigipAuthLdap,
    BigipAuthLoginFailures,
    BigipAuthPartition,
    BigipAuthPassword,
    BigipAuthPasswordPolicy,
    BigipAuthRadius,
    BigipAuthRadiusServer,
    BigipAuthRemoteRole,
    BigipAuthRemoteUser,
    BigipAuthSource,
    BigipAuthTacacs,
    BigipAuthUser,
    BigipCmCert,
    BigipCmDevice,
    BigipCmDeviceGroup,
    BigipCmKey,
    BigipCmTrafficGroup,
    BigipCmTrustDomain,
    BigipDataGroup,
    BigipGtmDatacenter,
    BigipGtmDistributedApp,
    BigipGtmGlobalSettingsGeneral,
    BigipGtmGlobalSettingsLoadBalancing,
    BigipGtmGlobalSettingsMetrics,
    BigipGtmGlobalSettingsMetricsExclusions,
    BigipGtmLink,
    BigipGtmListener,
    BigipGtmListenerDohProxy,
    BigipGtmListenerDohServer,
    BigipGtmPool,
    BigipGtmProberPool,
    BigipGtmRegion,
    BigipGtmRule,
    BigipGtmServer,
    BigipGtmTopology,
    BigipGtmWideip,
    BigipLtmAuthObject,
    BigipLtmCipherGroup,
    BigipLtmCipherRule,
    BigipLtmDnsAnalyticsGlobalSettings,
    BigipLtmDnsCacheGlobalSettings,
    BigipLtmDnsCacheRecord,
    BigipLtmDnsCacheResolver,
    BigipLtmDnsCacheTransparent,
    BigipLtmDnsCacheValidatingResolver,
    BigipLtmDnsDnssecKey,
    BigipLtmDnsDnssecZone,
    BigipLtmDnsHpkeKey,
    BigipLtmDnsHpkeProfile,
    BigipLtmDnsNameserver,
    BigipLtmDnsTsigKey,
    BigipLtmDnsZone,
    BigipLtmEvictionPolicy,
    BigipLtmIfile,
    BigipLtmMessageRoutingObject,
    BigipLtmMinimalObject,
    BigipLtmNat,
    BigipLtmPolicyStrategy,
    BigipLtmSnat,
    BigipLtmSnatTranslation,
    BigipLtmTrafficClass,
    BigipLtmTrafficMatchingCriteria,
    BigipMonitor,
    BigipNetDnsResolver,
    BigipNetInterface,
    BigipNetMinimalObject,
    BigipNetPortList,
    BigipNetRoute,
    BigipNetRouteDomain,
    BigipNetSelf,
    BigipNetStp,
    BigipNetTunnel,
    BigipNetVlan,
    BigipNode,
    BigipPemForwardingEndpoint,
    BigipPemInterceptionEndpoint,
    BigipPemListener,
    BigipPemMinimalObject,
    BigipPemPolicy,
    BigipPemProfile,
    BigipPemRatingGroup,
    BigipPemRule,
    BigipPemServiceChainEndpoint,
    BigipPersistence,
    BigipPolicy,
    BigipPolicyAction,
    BigipPolicyCondition,
    BigipPolicyRule,
    BigipPool,
    BigipPoolMember,
    BigipProfile,
    BigipRule,
    BigipSecurityBotDefenseProfile,
    BigipSecurityDeviceIdAttribute,
    BigipSecurityDosProfile,
    BigipSecurityFirewallAddressList,
    BigipSecurityFirewallConfigChangeLog,
    BigipSecurityFirewallConfigEntityId,
    BigipSecurityFirewallGlobalFqdnPolicy,
    BigipSecurityFirewallGlobalRules,
    BigipSecurityFirewallManagementIpRules,
    BigipSecurityFirewallOnDemandCompilation,
    BigipSecurityFirewallOnDemandRuleDeploy,
    BigipSecurityFirewallPolicy,
    BigipSecurityFirewallPortList,
    BigipSecurityFirewallPortMisusePolicy,
    BigipSecurityFirewallRuleList,
    BigipSecurityFirewallSchedule,
    BigipSecurityFirewallUserDomain,
    BigipSecurityFirewallUserList,
    BigipSecurityFirewallUuidDefaultAutogenerate,
    BigipSecurityHttpProfile,
    BigipSecurityIpIntelligenceFeedList,
    BigipSecurityIpIntelligenceGlobalPolicy,
    BigipSecurityIpIntelligencePolicy,
    BigipSecurityLogProfile,
    BigipSecurityMinimalObject,
    BigipSecurityNatDestinationTranslation,
    BigipSecurityNatPolicy,
    BigipSecurityNatSourceTranslation,
    BigipSecurityPacketFilterDefaultRules,
    BigipSecurityPacketFilterPolicy,
    BigipSecurityProtectedZone,
    BigipSecurityProtocolInspectionComplianceMap,
    BigipSecurityProtocolInspectionComplianceObject,
    BigipSecuritySshProfile,
    BigipSecurityZone,
    BigipSnatPool,
    BigipSysDns,
    BigipSysFileSslCert,
    BigipSysFileSslKey,
    BigipSysFolder,
    BigipSysGlobalSettings,
    BigipSysManagementRoute,
    BigipSysNtp,
    BigipSysProvision,
    BigipSysSnmp,
    BigipVirtualAddress,
    BigipVirtualServer,
)
from .errors import EvalError
from .values import FieldSlot, ObjectRef, PathRef, Root

# ---------------------------------------------------------------------------
# Container abstraction
# ---------------------------------------------------------------------------


@dataclass
class Container:
    """A navigable mapping projected from a :class:`BigipConfig`.

    Containers are returned for the namespace nodes ``.ltm``, ``.gtm``,
    and for kind nodes like ``.ltm.virtual``.  Leaf entries inside a
    kind container are :class:`ObjectRef` instances; entries inside a
    namespace container are themselves :class:`Container` instances.

    The ``kind`` field carries either the module name (``"ltm"``) or
    the full TMSH module+type (``"ltm virtual"``).  Builtins and
    error messages use it to describe the level being navigated.
    """

    kind: str
    root: Root
    # Lazily filled.  Keys are the user-visible identifiers — full-paths
    # for object kinds, plain TMSH type names for module namespaces.
    _entries: dict[str, Any] | None = None
    _entry_source: str = ""  # "ltm.virtual", "ltm", ""

    def entries(self) -> dict[str, Any]:
        if self._entries is None:
            self._entries = _build_entries(self)
        return self._entries

    def lookup(self, key: str) -> Any:
        ents = self.entries()
        if key in ents:
            return ents[key]
        # Partition shorthand: bare name resolves to ``/Common/<name>``
        # when that key exists and is unambiguous.  Any other matching
        # full-path with a different partition makes the lookup
        # ambiguous and we raise rather than guess.
        if self._is_object_kind() and not key.startswith("/"):
            full = f"/Common/{key}"
            matches = [k for k in ents if k.endswith(f"/{key}")]
            if full in ents:
                return ents[full]
            if len(matches) == 1:
                return ents[matches[0]]
            if len(matches) > 1:
                raise EvalError(
                    f"{self.kind}: name {key!r} is ambiguous "
                    f"({len(matches)} matches; use a full path)"
                )
        raise EvalError(f"{self.kind}: no entry {key!r}")

    def regex_keys(self, pattern: str) -> list[str]:
        try:
            rx = re.compile(pattern)
        except re.error as exc:
            raise EvalError(f"invalid regex subscript {pattern!r}: {exc}") from exc
        return [k for k in self.entries() if rx.search(k)]

    def _is_object_kind(self) -> bool:
        # Object kinds are the leaf containers (``ltm virtual`` etc.);
        # the bare module containers ("ltm", "gtm") hold sub-containers.
        return " " in self.kind or self.kind in _OBJECT_KIND_ALIASES


# ---------------------------------------------------------------------------
# Field maps — TMSH-spelt user names mapping to dataclass attribute names.
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class FieldSpec:
    """How a single TMSH-spelt field projects from a dataclass."""

    attr: str  # dataclass attribute
    # ``ref_kind`` non-empty signals "this is a PathRef into <kind>".
    # ``list_ref`` flags list-of-PathRef fields.
    ref_kind: str = ""
    list_ref: bool = False


_VS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "destination": FieldSpec("destination"),
    "pool": FieldSpec("pool", ref_kind="ltm pool"),
    "rules": FieldSpec("rules", ref_kind="ltm rule", list_ref=True),
    "profiles": FieldSpec("profiles", ref_kind="ltm profile", list_ref=True),
    "persist": FieldSpec("persist", ref_kind="ltm persistence", list_ref=True),
    "policies": FieldSpec("policies", ref_kind="ltm policy", list_ref=True),
    "snatpool": FieldSpec("snatpool", ref_kind="ltm snatpool"),
    "source-address-translation": FieldSpec("source_address_translation"),
    "description": FieldSpec("description"),
    "mask": FieldSpec("mask"),
    "source": FieldSpec("source"),
    "ip-protocol": FieldSpec("ip_protocol"),
    "connection-limit": FieldSpec("connection_limit"),
    "rate-limit": FieldSpec("rate_limit"),
    "rate-limit-mode": FieldSpec("rate_limit_mode"),
    "rate-limit-dst-mask": FieldSpec("rate_limit_dst_mask"),
    "rate-limit-src-mask": FieldSpec("rate_limit_src_mask"),
    "auto-lasthop": FieldSpec("auto_lasthop"),
    "translate-address": FieldSpec("translate_address"),
    "translate-port": FieldSpec("translate_port"),
    "state": FieldSpec("state"),
    "address-status": FieldSpec("address_status"),
    "auto-discovery": FieldSpec("auto_discovery"),
    "cmp-enabled": FieldSpec("cmp_enabled"),
    "eviction-protected": FieldSpec("eviction_protected"),
    "dhcp-relay": FieldSpec("dhcp_relay"),
    "internal": FieldSpec("internal"),
    "ip-forward": FieldSpec("ip_forward"),
    "l2-forward": FieldSpec("l2_forward"),
    "reject": FieldSpec("reject"),
    "nat64": FieldSpec("nat64"),
    "gtm-score": FieldSpec("gtm_score"),
    "mirror": FieldSpec("mirror"),
    "service-down-immediate-action": FieldSpec("service_down_immediate_action"),
    "source-port": FieldSpec("source_port"),
    "serverssl-use-sni": FieldSpec("serverssl_use_sni"),
    "rate-class": FieldSpec("rate_class"),
    "per-flow-request-access-policy": FieldSpec(
        "per_flow_request_access_policy", ref_kind="apm policy access-policy"
    ),
    "transparent-nexthop": FieldSpec("transparent_nexthop", ref_kind="net vlan"),
    "vlans": FieldSpec("vlans", ref_kind="net vlan", list_ref=True),
    "vlans-disabled": FieldSpec("vlans_disabled"),
    "vlans-enabled": FieldSpec("vlans_enabled"),
    "fallback-persistence": FieldSpec("fallback_persistence", ref_kind="ltm persistence"),
    "last-hop-pool": FieldSpec("last_hop_pool", ref_kind="ltm pool"),
    "fw-enforced-policy": FieldSpec("fw_enforced_policy"),
    "fw-staged-policy": FieldSpec("fw_staged_policy"),
    "flow-eviction-policy": FieldSpec("flow_eviction_policy"),
    "service-policy": FieldSpec("service_policy"),
    "auth": FieldSpec("auth_profiles", ref_kind="ltm profile", list_ref=True),
    "traffic-classes": FieldSpec("traffic_classes"),
    "clone-pools": FieldSpec("clone_pools", ref_kind="ltm pool", list_ref=True),
}

_POOL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "module": FieldSpec("module"),
    "monitor": FieldSpec("monitor", ref_kind="ltm monitor"),
    "load-balancing-mode": FieldSpec("load_balancing_mode"),
    "members": FieldSpec("members"),  # special-cased: list of member objects
    "description": FieldSpec("description"),
    "min-active-members": FieldSpec("min_active_members"),
    "min-up-members": FieldSpec("min_up_members"),
    "service-down-action": FieldSpec("service_down_action"),
    "slow-ramp-time": FieldSpec("slow_ramp_time"),
    "allow-snat": FieldSpec("allow_snat"),
    "allow-nat": FieldSpec("allow_nat"),
    "reselect-tries": FieldSpec("reselect_tries"),
    "queue-depth-limit": FieldSpec("queue_depth_limit"),
    "queue-time-limit": FieldSpec("queue_time_limit"),
    "connection-limit": FieldSpec("connection_limit"),
    "rate-limit": FieldSpec("rate_limit"),
    "ratio": FieldSpec("ratio"),
    "down-interval": FieldSpec("down_interval"),
    "interval": FieldSpec("interval"),
    "min-up-members-action": FieldSpec("min_up_members_action"),
    "min-up-members-checking": FieldSpec("min_up_members_checking"),
    "ip-tos-to-client": FieldSpec("ip_tos_to_client"),
    "ip-tos-to-server": FieldSpec("ip_tos_to_server"),
    "link-qos-to-client": FieldSpec("link_qos_to_client"),
    "link-qos-to-server": FieldSpec("link_qos_to_server"),
    "gateway-failsafe-device": FieldSpec("gateway_failsafe_device"),
    "ignore-persisted-weight": FieldSpec("ignore_persisted_weight"),
    "inherit-profile": FieldSpec("inherit_profile"),
    "queue-on-connection-limit": FieldSpec("queue_on_connection_limit"),
    "address-family": FieldSpec("address_family"),
    "autopopulate": FieldSpec("autopopulate"),
    "profiles": FieldSpec("profiles", ref_kind="ltm profile", list_ref=True),
}

_VIRTUAL_ADDRESS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "address": FieldSpec("address"),
    "mask": FieldSpec("mask"),
    "arp": FieldSpec("arp"),
    "icmp-echo": FieldSpec("icmp_echo"),
    "auto-delete": FieldSpec("auto_delete"),
    "connection-limit": FieldSpec("connection_limit"),
    "traffic-group": FieldSpec("traffic_group", ref_kind="cm traffic-group"),
    "inherited-traffic-group": FieldSpec("inherited_traffic_group"),
    "route-advertisement": FieldSpec("route_advertisement"),
    "server-scope": FieldSpec("server_scope"),
    "spanning": FieldSpec("spanning"),
    "unit": FieldSpec("unit"),
    "description": FieldSpec("description"),
    "state": FieldSpec("state"),
    "floating": FieldSpec("floating"),
    "traffic-group-restored": FieldSpec("traffic_group_restored"),
}

# Bundle 13 — ltm.* cross-cutting infra.

_LTM_CIPHER_GROUP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "allow": FieldSpec("allow", ref_kind="ltm cipher rule", list_ref=True),
    "require": FieldSpec("require", ref_kind="ltm cipher rule", list_ref=True),
    "exclude": FieldSpec("exclude", ref_kind="ltm cipher rule", list_ref=True),
    "ordering": FieldSpec("ordering"),
}

_LTM_CIPHER_RULE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "cipher": FieldSpec("cipher"),
    "dh-groups": FieldSpec("dh_groups"),
    "signature-algorithms": FieldSpec("signature_algorithms"),
}

_LTM_NAT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "translation-address": FieldSpec("translation_address"),
    "originating-address": FieldSpec("originating_address"),
    "traffic-group": FieldSpec("traffic_group", ref_kind="cm traffic-group"),
    "vlans": FieldSpec("vlans", ref_kind="net vlan", list_ref=True),
    "vlans-disabled": FieldSpec("vlans_disabled"),
    "vlans-enabled": FieldSpec("vlans_enabled"),
    "mirror": FieldSpec("mirror"),
    "arp": FieldSpec("arp"),
    "state": FieldSpec("state"),
}

_LTM_SNAT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "origins": FieldSpec("origins"),
    "translation": FieldSpec("translation"),
    "snatpool": FieldSpec("snatpool", ref_kind="ltm snatpool"),
    "vlans": FieldSpec("vlans", ref_kind="net vlan", list_ref=True),
    "vlans-disabled": FieldSpec("vlans_disabled"),
    "vlans-enabled": FieldSpec("vlans_enabled"),
    "automap": FieldSpec("automap"),
    "mirror": FieldSpec("mirror"),
    "state": FieldSpec("state"),
}

_LTM_SNAT_TRANSLATION_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "address": FieldSpec("address"),
    "inherited-traffic-group": FieldSpec("inherited_traffic_group"),
    "traffic-group": FieldSpec("traffic_group", ref_kind="cm traffic-group"),
    "connection-limit": FieldSpec("connection_limit"),
    "ip-idle-timeout": FieldSpec("ip_idle_timeout"),
    "tcp-idle-timeout": FieldSpec("tcp_idle_timeout"),
    "udp-idle-timeout": FieldSpec("udp_idle_timeout"),
    "state": FieldSpec("state"),
}

_LTM_POLICY_STRATEGY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "strategy": FieldSpec("strategy"),
    "operands": FieldSpec("operands"),
}

_LTM_TRAFFIC_CLASS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "classification": FieldSpec("classification"),
    "match-method": FieldSpec("match_method"),
}

_LTM_TRAFFIC_MATCHING_CRITERIA_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "destination-address-list": FieldSpec(
        "destination_address_list", ref_kind="security firewall address-list"
    ),
    "destination-address-inline": FieldSpec("destination_address_inline"),
    "destination-port-list": FieldSpec(
        "destination_port_list", ref_kind="security firewall port-list"
    ),
    "destination-port-inline": FieldSpec("destination_port_inline"),
    "source-address-list": FieldSpec(
        "source_address_list", ref_kind="security firewall address-list"
    ),
    "source-address-inline": FieldSpec("source_address_inline"),
    "source-port-list": FieldSpec("source_port_list", ref_kind="security firewall port-list"),
    "source-port-inline": FieldSpec("source_port_inline"),
    "protocol": FieldSpec("protocol"),
    "route-domain": FieldSpec("route_domain", ref_kind="net route-domain"),
}

_LTM_IFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "file-name": FieldSpec("file_name"),
}

_LTM_EVICTION_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "high-water-mark": FieldSpec("high_water_mark"),
    "low-water-mark": FieldSpec("low_water_mark"),
    "slow-flow-throttle": FieldSpec("slow_flow_throttle"),
    "slow-flow-monitoring": FieldSpec("slow_flow_monitoring"),
}

# Bundle 14 — ltm dns.* (DNS Express).

_LTM_DNS_NAMESERVER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "address": FieldSpec("address"),
    "port": FieldSpec("port"),
    "tsig-key": FieldSpec("tsig_key", ref_kind="ltm dns tsig-key"),
    "route-domain": FieldSpec("route_domain", ref_kind="net route-domain"),
}

_LTM_DNS_TSIG_KEY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "algorithm": FieldSpec("algorithm"),
    "secret": FieldSpec("secret"),
}

_LTM_DNS_ZONE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "dns-express-server": FieldSpec("dns_express_server", ref_kind="ltm dns nameserver"),
    "dns-express-allow-notify": FieldSpec(
        "dns_express_allow_notify", ref_kind="ltm dns nameserver", list_ref=True
    ),
    "dns-express-enabled": FieldSpec("dns_express_enabled"),
    "response-policy": FieldSpec("response_policy"),
    "transfer-clients": FieldSpec("transfer_clients"),
}

_LTM_DNS_DNSSEC_KEY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "type": FieldSpec("type_"),
    "algorithm": FieldSpec("algorithm"),
    "bit-width": FieldSpec("bit_width"),
    "rollover-period": FieldSpec("rollover_period"),
}

_LTM_DNS_DNSSEC_ZONE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "keys": FieldSpec("keys", ref_kind="ltm dns dnssec key", list_ref=True),
    "enable": FieldSpec("enable"),
}

_LTM_DNS_CACHE_RESOLVER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "message-cache-size": FieldSpec("message_cache_size"),
    "resolver-cache-size": FieldSpec("resolver_cache_size"),
    "answer-default-zones": FieldSpec("answer_default_zones"),
    "forward-zones": FieldSpec("forward_zones"),
    "route-domain": FieldSpec("route_domain", ref_kind="net route-domain"),
}

_LTM_DNS_CACHE_TRANSPARENT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "message-cache-size": FieldSpec("message_cache_size"),
}

_LTM_DNS_CACHE_VALIDATING_RESOLVER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "message-cache-size": FieldSpec("message_cache_size"),
    "resolver-cache-size": FieldSpec("resolver_cache_size"),
}

_LTM_DNS_CACHE_GLOBAL_SETTINGS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "expiry-time": FieldSpec("expiry_time"),
    "nameserver-ttl": FieldSpec("nameserver_ttl"),
}

_LTM_DNS_CACHE_RECORD_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "record-kind": FieldSpec("record_kind"),
    "description": FieldSpec("description"),
}

_LTM_DNS_HPKE_KEY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "algorithm": FieldSpec("algorithm"),
}

_LTM_DNS_HPKE_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "defaults-from": FieldSpec("defaults_from", ref_kind="ltm dns hpke profile"),
    "keys": FieldSpec("keys", ref_kind="ltm dns hpke key", list_ref=True),
}

_LTM_DNS_ANALYTICS_GLOBAL_SETTINGS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
}

# Bundle 15 — shared field map for all 20 ltm message-routing.* kinds.
_LTM_MESSAGE_ROUTING_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}

# Bundle 16 — shared field map for all 11 ltm auth.* profile kinds.
# Surfaces ``defaults-from`` since most auth profiles inherit from a
# system-default chain.
_LTM_AUTH_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
    "defaults-from": FieldSpec("defaults_from"),
}

# Bundles 17-20 — shared field map for the generic ltm.* minimal
# kinds (CGNAT/LSN, global-settings, classification, tacdb).
_LTM_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}

# Bundles 21-26: net.*  /  27-31: apm.*  /  32: pem.*  /  33-41: sys.*
# / 42: vcmp.* / 43: cm.* / 44: cli.* / 45: api-protection.*
# All use identical shape — name / full-path / kind / description.
_NET_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}
_APM_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}
_PEM_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}
_SYS_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}
_VCMP_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}
_CM_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}
_CLI_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}
_API_PROTECTION_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}

_NODE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "address": FieldSpec("address"),
    "description": FieldSpec("description"),
    "monitor": FieldSpec("monitor", ref_kind="ltm monitor"),
    "state": FieldSpec("state"),
    "connection-limit": FieldSpec("connection_limit"),
    "rate-limit": FieldSpec("rate_limit"),
    "ratio": FieldSpec("ratio"),
    "fqdn": FieldSpec("fqdn"),
}

_RULE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "body": FieldSpec("source"),
    "description": FieldSpec("description"),
    "refs": FieldSpec("__refs__"),  # synthesised — see ``_rule_refs_value``
}

_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("profile_type"),
    "defaults-from": FieldSpec("defaults_from", ref_kind="ltm profile"),
    "description": FieldSpec("description"),
}

_MONITOR_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("monitor_type"),
    "defaults-from": FieldSpec("defaults_from", ref_kind="ltm monitor"),
    "description": FieldSpec("description"),
    "interval": FieldSpec("interval"),
    "timeout": FieldSpec("timeout"),
    "destination": FieldSpec("destination"),
    "send": FieldSpec("send"),
    "recv": FieldSpec("recv"),
}

# Same shape as ``_MONITOR_FIELDS`` but ``defaults-from`` resolves
# into ``gtm monitor`` rather than ``ltm monitor`` so a query
# like ``.gtm.monitor[]."defaults-from".name`` walks the parent
# chain within GTM, not into the (potentially same-path) LTM
# monitor.
_GTM_MONITOR_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("monitor_type"),
    "defaults-from": FieldSpec("defaults_from", ref_kind="gtm monitor"),
    "description": FieldSpec("description"),
    "interval": FieldSpec("interval"),
    "timeout": FieldSpec("timeout"),
    "destination": FieldSpec("destination"),
    "send": FieldSpec("send"),
    "recv": FieldSpec("recv"),
}

_PERSIST_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("persistence_type"),
    "defaults-from": FieldSpec("defaults_from", ref_kind="ltm persistence"),
    "description": FieldSpec("description"),
    "timeout": FieldSpec("timeout"),
    "match-across-pools": FieldSpec("match_across_pools"),
    "match-across-services": FieldSpec("match_across_services"),
    "match-across-virtuals": FieldSpec("match_across_virtuals"),
    "mirror": FieldSpec("mirror"),
    "override-connection-limit": FieldSpec("override_connection_limit"),
    "always-send": FieldSpec("always_send"),
    "cookie-name": FieldSpec("cookie_name"),
    "cookie-encryption": FieldSpec("cookie_encryption"),
    "cookie-encryption-passphrase": FieldSpec("cookie_encryption_passphrase"),
    "httponly": FieldSpec("httponly"),
    "secure": FieldSpec("secure"),
    "expiration": FieldSpec("expiration"),
    "method": FieldSpec("method"),
    "hash-length": FieldSpec("hash_length"),
    "hash-offset": FieldSpec("hash_offset"),
}

_SNATPOOL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "members": FieldSpec("members"),
    "description": FieldSpec("description"),
}

_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "strategy": FieldSpec("strategy"),
    "controls": FieldSpec("controls"),
    "requires": FieldSpec("requires"),
    # ``rules`` is special-cased in ``_project_field`` so each rule
    # surfaces as an ``ltm policy-rule`` ObjectRef with nested
    # ``conditions`` / ``actions`` sub-objects.
    "rules": FieldSpec("rules"),
    "description": FieldSpec("description"),
    "status": FieldSpec("status"),
    "last-modified": FieldSpec("last_modified"),
}

_DATAGROUP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("value_type"),
    "kind": FieldSpec("kind"),
    "records": FieldSpec("records"),
    "description": FieldSpec("description"),
}

_NET_ROUTE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "network": FieldSpec("network"),
    "gw": FieldSpec("gw"),
    "pool": FieldSpec("pool", ref_kind="ltm pool"),
    "description": FieldSpec("description"),
    "mtu": FieldSpec("mtu"),
    "blackhole": FieldSpec("blackhole"),
    "interface": FieldSpec("interface", ref_kind="net vlan"),
}

_NET_VLAN_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "tag": FieldSpec("tag"),
    "interfaces": FieldSpec("interfaces"),
    "description": FieldSpec("description"),
    "mtu": FieldSpec("mtu"),
    "cmp-hash": FieldSpec("cmp_hash"),
    "failsafe": FieldSpec("failsafe"),
    "failsafe-action": FieldSpec("failsafe_action"),
    "failsafe-timeout": FieldSpec("failsafe_timeout"),
    "fwd-mode": FieldSpec("fwd_mode"),
    "hardware-syncookie": FieldSpec("hardware_syncookie"),
    "learning": FieldSpec("learning"),
    "tag-mode": FieldSpec("tag_mode"),
    "virtual-wire": FieldSpec("virtual_wire"),
    "auto-lasthop": FieldSpec("auto_lasthop"),
    "source-check": FieldSpec("source_check"),
    "source-checking": FieldSpec("source_checking"),
    "syn-flood-rate-limit": FieldSpec("syn_flood_rate_limit"),
    "syncache-threshold": FieldSpec("syncache_threshold"),
    "service-policy": FieldSpec("service_policy"),
}

_NET_SELF_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "address": FieldSpec("address"),
    "vlan": FieldSpec("vlan", ref_kind="net vlan"),
    "traffic-group": FieldSpec("traffic_group", ref_kind="cm traffic-group"),
    "allow-service": FieldSpec("allow_service"),
    "description": FieldSpec("description"),
    "floating": FieldSpec("floating"),
    "unit": FieldSpec("unit"),
    "service-policy": FieldSpec("service_policy"),
    "fw-enforced-policy": FieldSpec("fw_enforced_policy"),
    "fw-staged-policy": FieldSpec("fw_staged_policy"),
    "inherited-traffic-group": FieldSpec("inherited_traffic_group"),
    "address-source": FieldSpec("address_source"),
}

_NET_ROUTE_DOMAIN_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "id": FieldSpec("id"),
    "vlans": FieldSpec("vlans", ref_kind="net vlan", list_ref=True),
    "description": FieldSpec("description"),
    "parent": FieldSpec("parent", ref_kind="net route-domain"),
    "strict": FieldSpec("strict"),
    "fw-enforced-policy": FieldSpec("fw_enforced_policy"),
    "fw-staged-policy": FieldSpec("fw_staged_policy"),
    "bwc-policy": FieldSpec("bwc_policy"),
    "connection-limit": FieldSpec("connection_limit"),
    "flow-eviction-policy": FieldSpec("flow_eviction_policy"),
    "routing-protocol": FieldSpec("routing_protocol"),
    "security-nat-policy": FieldSpec("security_nat_policy"),
    "service-policy": FieldSpec("service_policy"),
}

_NET_PORT_LIST_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "ports": FieldSpec("ports"),
    "description": FieldSpec("description"),
}

_NET_INTERFACE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "media-fixed": FieldSpec("media_fixed"),
    "description": FieldSpec("description"),
    "enabled": FieldSpec("enabled"),
    "disabled": FieldSpec("disabled"),
    "bundle": FieldSpec("bundle"),
    "bundle-speed": FieldSpec("bundle_speed"),
    "lldp-admin": FieldSpec("lldp_admin"),
    "mtu": FieldSpec("mtu"),
    "flow-control": FieldSpec("flow_control"),
    "mac-address": FieldSpec("mac_address"),
    "media-active": FieldSpec("media_active"),
    "media-max": FieldSpec("media_max"),
    "media-sfp": FieldSpec("media_sfp"),
    "port-fwd-mode": FieldSpec("port_fwd_mode"),
    "qinq-ethertype": FieldSpec("qinq_ethertype"),
    "stp": FieldSpec("stp"),
    "stp-edge-port": FieldSpec("stp_edge_port"),
    "stp-link-type": FieldSpec("stp_link_type"),
    "stp-auto-edge-port": FieldSpec("stp_auto_edge_port"),
    "stp-reset": FieldSpec("stp_reset"),
    "sflow-poll-interval": FieldSpec("sflow_poll_interval"),
    "sflow-poll-interval-global": FieldSpec("sflow_poll_interval_global"),
    "vendor": FieldSpec("vendor"),
    "vendor-oui": FieldSpec("vendor_oui"),
    "vendor-partnum": FieldSpec("vendor_partnum"),
    "vendor-revision": FieldSpec("vendor_revision"),
    "virtual-wire": FieldSpec("virtual_wire"),
    "transmitter-technology": FieldSpec("transmitter_technology"),
    "lacp-port-priority": FieldSpec("lacp_port_priority"),
}

_NET_DNS_RESOLVER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "route-domain": FieldSpec("route_domain", ref_kind="net route-domain"),
    "forward-zones": FieldSpec("forward_zones"),
    "description": FieldSpec("description"),
    "cache-size": FieldSpec("cache_size"),
    "randomize-query-name-case": FieldSpec("randomize_query_name_case"),
    "use-ipv4": FieldSpec("use_ipv4"),
    "use-ipv6": FieldSpec("use_ipv6"),
    "use-tcp": FieldSpec("use_tcp"),
    "use-udp": FieldSpec("use_udp"),
    "nameservers": FieldSpec("nameservers"),
    "answer-default-zones": FieldSpec("answer_default_zones"),
    "prefetch": FieldSpec("prefetch"),
    "nameserver-min-rtt": FieldSpec("nameserver_min_rtt"),
    "nameserver-ttl": FieldSpec("nameserver_ttl"),
    "outbound-msg-retry": FieldSpec("outbound_msg_retry"),
}

_NET_TUNNEL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "profile": FieldSpec("profile", ref_kind="ltm profile"),
    "local-address": FieldSpec("local_address"),
    "remote-address": FieldSpec("remote_address"),
    "description": FieldSpec("description"),
    "mtu": FieldSpec("mtu"),
    "mode": FieldSpec("mode"),
    "idle-timeout": FieldSpec("idle_timeout"),
    "auto-lasthop": FieldSpec("auto_lasthop"),
    "secondary-address": FieldSpec("secondary_address"),
    "traffic-group": FieldSpec("traffic_group", ref_kind="cm traffic-group"),
    "transparent": FieldSpec("transparent"),
    "key": FieldSpec("key"),
    "use-pmtu": FieldSpec("use_pmtu"),
    "tos": FieldSpec("tos"),
}

_NET_STP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "interfaces": FieldSpec("interfaces"),
    "description": FieldSpec("description"),
    "mode": FieldSpec("mode"),
    "priority": FieldSpec("priority"),
    "external-path-cost": FieldSpec("external_path_cost"),
    "internal-path-cost": FieldSpec("internal_path_cost"),
    "vlans": FieldSpec("vlans", ref_kind="net vlan", list_ref=True),
}

_SYS_DNS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "name-servers": FieldSpec("name_servers"),
    "search": FieldSpec("search"),
}

_SYS_NTP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "servers": FieldSpec("servers"),
    "timezone": FieldSpec("timezone"),
}

_SYS_SNMP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "agent-addresses": FieldSpec("agent_addresses"),
    "communities": FieldSpec("communities"),
}

_SYS_GLOBAL_SETTINGS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "hostname": FieldSpec("hostname"),
    "gui-setup": FieldSpec("gui_setup"),
    "mgmt-dhcp": FieldSpec("mgmt_dhcp"),
}

_SYS_PROVISION_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "level": FieldSpec("level"),
    "cpu-ratio": FieldSpec("cpu_ratio"),
    "memory-ratio": FieldSpec("memory_ratio"),
    "disk-ratio": FieldSpec("disk_ratio"),
}

_SYS_FOLDER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "device-group": FieldSpec("device_group", ref_kind="cm device-group"),
    "traffic-group": FieldSpec("traffic_group", ref_kind="cm traffic-group"),
    "hidden": FieldSpec("hidden"),
    "description": FieldSpec("description"),
    "inherited-device-group": FieldSpec("inherited_device_group"),
    "inherited-traffic-group": FieldSpec("inherited_traffic_group"),
}

_SYS_FILE_SSL_CERT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "source-path": FieldSpec("source_path"),
    "cache-path": FieldSpec("cache_path"),
    "revision": FieldSpec("revision"),
    "description": FieldSpec("description"),
    "issuer": FieldSpec("issuer"),
    "subject": FieldSpec("subject"),
    "expiration-string": FieldSpec("expiration_string"),
    "expiration-date": FieldSpec("expiration_date"),
    "fingerprint": FieldSpec("fingerprint"),
    "key-size": FieldSpec("key_size"),
    "key-type": FieldSpec("key_type"),
    "is-bundle": FieldSpec("is_bundle"),
    "certificate-key-size": FieldSpec("certificate_key_size"),
    "issuer-cert": FieldSpec("issuer_cert", ref_kind="sys file ssl-cert"),
    "serial-number": FieldSpec("serial_number"),
    "version": FieldSpec("version"),
    "subject-alternative-name": FieldSpec("subject_alternative_name"),
    "bundle-certificates": FieldSpec("bundle_certificates"),
    "cert-validation-options": FieldSpec("cert_validation_options"),
    "cert-validators": FieldSpec("cert_validators"),
    "checksum": FieldSpec("checksum"),
    "mode": FieldSpec("mode"),
    "size": FieldSpec("size"),
    "create-time": FieldSpec("create_time"),
    "created-by": FieldSpec("created_by"),
    "last-update-time": FieldSpec("last_update_time"),
    "updated-by": FieldSpec("updated_by"),
}

_SYS_FILE_SSL_KEY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "source-path": FieldSpec("source_path"),
    "cache-path": FieldSpec("cache_path"),
    "revision": FieldSpec("revision"),
    "passphrase": FieldSpec("passphrase"),
    "description": FieldSpec("description"),
    "key-size": FieldSpec("key_size"),
    "key-type": FieldSpec("key_type"),
    "security-type": FieldSpec("security_type"),
    "checksum": FieldSpec("checksum"),
    "mode": FieldSpec("mode"),
    "size": FieldSpec("size"),
    "create-time": FieldSpec("create_time"),
    "created-by": FieldSpec("created_by"),
    "last-update-time": FieldSpec("last_update_time"),
    "updated-by": FieldSpec("updated_by"),
}

_SYS_MANAGEMENT_ROUTE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "gateway": FieldSpec("gateway"),
    "network": FieldSpec("network"),
    "mtu": FieldSpec("mtu"),
    "description": FieldSpec("description"),
}

_SECURITY_FW_PORT_LIST_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "ports": FieldSpec("ports"),
    "description": FieldSpec("description"),
}

_SECURITY_FW_RULE_LIST_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "rules": FieldSpec("rules"),
    "description": FieldSpec("description"),
}

_SECURITY_FW_CONFIG_ENTITY_ID_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "entity-id": FieldSpec("entity_id"),
    "description": FieldSpec("description"),
}

_SECURITY_FW_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "rules": FieldSpec("rules"),
    "rule-lists": FieldSpec("rule_lists", ref_kind="security firewall rule-list", list_ref=True),
}

_SECURITY_FW_ADDRESS_LIST_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "addresses": FieldSpec("addresses"),
    "address-lists": FieldSpec(
        "address_lists", ref_kind="security firewall address-list", list_ref=True
    ),
    "fqdns": FieldSpec("fqdns"),
}

_SECURITY_FW_GLOBAL_RULES_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "rules": FieldSpec("rules"),
    "enforced-policy": FieldSpec("enforced_policy", ref_kind="security firewall policy"),
    "staged-policy": FieldSpec("staged_policy", ref_kind="security firewall policy"),
}

_SECURITY_FW_MANAGEMENT_IP_RULES_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "rules": FieldSpec("rules"),
}

_SECURITY_FW_SCHEDULE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "daily-hour-end": FieldSpec("daily_hour_end"),
    "daily-hour-start": FieldSpec("daily_hour_start"),
    "days-of-week": FieldSpec("days_of_week"),
    "date-valid-end": FieldSpec("date_valid_end"),
    "date-valid-start": FieldSpec("date_valid_start"),
}

_SECURITY_FW_USER_LIST_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "users": FieldSpec("users"),
}

_SECURITY_FW_USER_DOMAIN_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "domain": FieldSpec("domain"),
}

_SECURITY_FW_GLOBAL_FQDN_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "context": FieldSpec("context"),
}

_SECURITY_FW_PORT_MISUSE_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "default-log": FieldSpec("default_log"),
}

_SECURITY_FW_ON_DEMAND_COMPILATION_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
}

_SECURITY_FW_ON_DEMAND_RULE_DEPLOY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
}

_SECURITY_FW_UUID_DEFAULT_AUTOGENERATE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "auto-generate-uuid": FieldSpec("auto_generate_uuid"),
}

_SECURITY_FW_CONFIG_CHANGE_LOG_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "log-publisher": FieldSpec("log_publisher"),
}

_SECURITY_NAT_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "rules": FieldSpec("rules"),
    "rule-lists": FieldSpec("rule_lists", ref_kind="security firewall rule-list", list_ref=True),
}

_SECURITY_NAT_SOURCE_TRANSLATION_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "type": FieldSpec("type_"),
    "addresses": FieldSpec("addresses"),
    "ports": FieldSpec("ports"),
    "traffic-group": FieldSpec("traffic_group", ref_kind="cm traffic-group"),
    "egress-interfaces-disabled": FieldSpec("egress_interfaces_disabled"),
}

_SECURITY_NAT_DESTINATION_TRANSLATION_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "type": FieldSpec("type_"),
    "addresses": FieldSpec("addresses"),
    "ports": FieldSpec("ports"),
}

_SECURITY_LOG_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "application-data": FieldSpec("application_data"),
    "network-data": FieldSpec("network_data"),
}

_SECURITY_DOS_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "app-service": FieldSpec("app_service"),
    "threshold-sensitivity": FieldSpec("threshold_sensitivity"),
}

_SECURITY_IP_INTELLIGENCE_FEED_LIST_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "feeds": FieldSpec("feeds"),
}

_SECURITY_IP_INTELLIGENCE_GLOBAL_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "log-blacklist-category": FieldSpec("log_blacklist_category"),
    "log-publisher": FieldSpec("log_publisher"),
}

_SECURITY_ZONE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "vlans": FieldSpec("vlans", ref_kind="net vlan", list_ref=True),
    "tunnels": FieldSpec("tunnels", ref_kind="net tunnels tunnel", list_ref=True),
    "interfaces": FieldSpec("interfaces"),
}

_SECURITY_PROTECTED_ZONE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "enabled": FieldSpec("enabled"),
}

_SECURITY_PACKET_FILTER_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "rules": FieldSpec("rules"),
}

_SECURITY_PACKET_FILTER_DEFAULT_RULES_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "action": FieldSpec("action"),
}

_SECURITY_SSH_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "defaults-from": FieldSpec("defaults_from", ref_kind="security ssh profile"),
    "timeout": FieldSpec("timeout"),
}

_SECURITY_HTTP_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "defaults-from": FieldSpec("defaults_from", ref_kind="security http profile"),
}

_SECURITY_BOT_DEFENSE_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "app-service": FieldSpec("app_service"),
    "template": FieldSpec("template"),
}

# Bundle 10b — shared field map for the 37 minimal security.* kinds.
_SECURITY_MINIMAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "kind": FieldSpec("kind"),
    "description": FieldSpec("description"),
}


_SECURITY_IP_INTELLIGENCE_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "default-action": FieldSpec("default_action"),
    "default-log-blacklist-hit-only": FieldSpec("default_log_blacklist_hit_only"),
    "default-log-blacklist-category": FieldSpec("default_log_blacklist_category"),
}

_SECURITY_PI_COMPLIANCE_MAP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "insp-id": FieldSpec("insp_id"),
    "key-type": FieldSpec("key_type"),
    "value-type": FieldSpec("value_type"),
    "description": FieldSpec("description"),
}

_SECURITY_PI_COMPLIANCE_OBJECT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "insp-id": FieldSpec("insp_id"),
    "type": FieldSpec("type_"),
    "description": FieldSpec("description"),
}

_SECURITY_DEVICE_ID_ATTRIBUTE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "id": FieldSpec("id_"),
    "description": FieldSpec("description"),
}

_APM_SSH_SECURITY_CONFIG_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "ciphers": FieldSpec("ciphers"),
    "hmacs": FieldSpec("hmacs"),
    "kex-methods": FieldSpec("kex_methods"),
    "compressions": FieldSpec("compressions"),
    "description": FieldSpec("description"),
}

_APM_OAUTH_DB_INSTANCE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "db-name": FieldSpec("db_name"),
    "purge-frequency": FieldSpec("purge_frequency"),
    "purge-time": FieldSpec("purge_time"),
}

_APM_POLICY_ACCESS_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "start-item": FieldSpec("start_item", ref_kind="apm policy policy-item"),
    "default-ending": FieldSpec("default_ending", ref_kind="apm policy policy-item"),
    "items": FieldSpec("items", ref_kind="apm policy policy-item", list_ref=True),
    "description": FieldSpec("description"),
}

_APM_POLICY_CUSTOMIZATION_SOURCE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
}

_APM_POLICY_ITEM_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "caption": FieldSpec("caption"),
    "color": FieldSpec("color"),
    "item-type": FieldSpec("item_type"),
    "agents": FieldSpec("agents", ref_kind="apm policy agent", list_ref=True),
    "description": FieldSpec("description"),
}

_APM_POLICY_AGENT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "agent-type": FieldSpec("agent_type"),
    "customization-group": FieldSpec("customization_group"),
    "auth": FieldSpec("auth"),
    "max-logon-attempt": FieldSpec("max_logon_attempt"),
    "auth-max-logon-attempt": FieldSpec("auth_max_logon_attempt"),
    "fetch-nested-groups": FieldSpec("fetch_nested_groups"),
    "fetch-primary-groups": FieldSpec("fetch_primary_groups"),
    "password-source": FieldSpec("password_source"),
    "query": FieldSpec("query"),
    "query-attrname": FieldSpec("query_attrname"),
    "query-filter": FieldSpec("query_filter"),
    "server": FieldSpec("server"),
    "show-extended-error": FieldSpec("show_extended_error"),
    "upn": FieldSpec("upn"),
    "username-source": FieldSpec("username_source"),
    "attribute-consuming-service": FieldSpec("attribute_consuming_service"),
    "attr-consuming-service-session-var": FieldSpec("attr_consuming_service_session_var"),
    "hints": FieldSpec("hints"),
}

_APM_REPORT_DEFAULT_REPORT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "report-name": FieldSpec("report_name"),
    "user": FieldSpec("user"),
}

_CM_CERT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "cache-path": FieldSpec("cache_path"),
    "checksum": FieldSpec("checksum"),
    "revision": FieldSpec("revision"),
    "issuer": FieldSpec("issuer"),
    "subject": FieldSpec("subject"),
    "subject-alternative-name": FieldSpec("subject_alternative_name"),
    "expiration-date": FieldSpec("expiration_date"),
    "expiration-string": FieldSpec("expiration_string"),
    "fingerprint": FieldSpec("fingerprint"),
    "serial-number": FieldSpec("serial_number"),
    "version": FieldSpec("version"),
    "key-type": FieldSpec("key_type"),
    "certificate-key-size": FieldSpec("certificate_key_size"),
    "is-bundle": FieldSpec("is_bundle"),
    "email": FieldSpec("email"),
    "source-path": FieldSpec("source_path"),
    "system-path": FieldSpec("system_path"),
    "size": FieldSpec("size"),
    "mode": FieldSpec("mode"),
    "create-time": FieldSpec("create_time"),
    "created-by": FieldSpec("created_by"),
    "last-update-time": FieldSpec("last_update_time"),
    "updated-by": FieldSpec("updated_by"),
}

_CM_KEY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "cache-path": FieldSpec("cache_path"),
    "checksum": FieldSpec("checksum"),
    "revision": FieldSpec("revision"),
    "key-size": FieldSpec("key_size"),
    "key-type": FieldSpec("key_type"),
    "security-type": FieldSpec("security_type"),
    "source-path": FieldSpec("source_path"),
    "system-path": FieldSpec("system_path"),
    "size": FieldSpec("size"),
    "mode": FieldSpec("mode"),
    "create-time": FieldSpec("create_time"),
    "created-by": FieldSpec("created_by"),
    "last-update-time": FieldSpec("last_update_time"),
    "updated-by": FieldSpec("updated_by"),
}

_CM_DEVICE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "hostname": FieldSpec("hostname"),
    "management-ip": FieldSpec("management_ip"),
    "base-mac": FieldSpec("base_mac"),
    "build": FieldSpec("build"),
    "edition": FieldSpec("edition"),
    "version": FieldSpec("version"),
    "product": FieldSpec("product"),
    "platform-id": FieldSpec("platform_id"),
    "chassis-id": FieldSpec("chassis_id"),
    "marketing-name": FieldSpec("marketing_name"),
    "self-device": FieldSpec("self_device"),
    "time-zone": FieldSpec("time_zone"),
    "cert": FieldSpec("cert", ref_kind="cm cert"),
    "key": FieldSpec("key", ref_kind="cm key"),
    "description": FieldSpec("description"),
    "comment": FieldSpec("comment"),
    "contact": FieldSpec("contact"),
    "location": FieldSpec("location"),
    "mirror-ip": FieldSpec("mirror_ip"),
    "mirror-secondary-ip": FieldSpec("mirror_secondary_ip"),
    "multicast-interface": FieldSpec("multicast_interface"),
    "multicast-ip": FieldSpec("multicast_ip"),
    "multicast-port": FieldSpec("multicast_port"),
    "unicast-address": FieldSpec("unicast_address"),
}

_CM_DEVICE_GROUP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "auto-sync": FieldSpec("auto_sync"),
    "network-failover": FieldSpec("network_failover"),
    "hidden": FieldSpec("hidden"),
    "devices": FieldSpec("devices", ref_kind="cm device", list_ref=True),
    "description": FieldSpec("description"),
    "type": FieldSpec("type_"),
    "save-on-auto-sync": FieldSpec("save_on_auto_sync"),
    "full-load-on-sync": FieldSpec("full_load_on_sync"),
    "asm-sync": FieldSpec("asm_sync"),
    "incremental-config-sync-size-max": FieldSpec("incremental_config_sync_size_max"),
}

_CM_TRAFFIC_GROUP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "unit-id": FieldSpec("unit_id"),
    "description": FieldSpec("description"),
    "default-device": FieldSpec("default_device", ref_kind="cm device"),
    "ha-load-factor": FieldSpec("ha_load_factor"),
    "ha-order": FieldSpec("ha_order", ref_kind="cm device", list_ref=True),
    "auto-failback-enabled": FieldSpec("auto_failback_enabled"),
    "auto-failback-time": FieldSpec("auto_failback_time"),
    "mac": FieldSpec("mac"),
}

_CM_TRUST_DOMAIN_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "ca-cert": FieldSpec("ca_cert", ref_kind="cm cert"),
    "ca-cert-bundle": FieldSpec("ca_cert_bundle", ref_kind="cm cert"),
    "ca-key": FieldSpec("ca_key", ref_kind="cm key"),
    "ca-devices": FieldSpec("ca_devices", ref_kind="cm device", list_ref=True),
    "guid": FieldSpec("guid"),
    "status": FieldSpec("status"),
    "trust-group": FieldSpec("trust_group", ref_kind="cm device-group"),
}

_GTM_DATACENTER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "contact": FieldSpec("contact"),
    "location": FieldSpec("location"),
    "description": FieldSpec("description"),
    "prober-pool": FieldSpec("prober_pool", ref_kind="gtm prober-pool"),
    "prober-preference": FieldSpec("prober_preference"),
    "prober-fallback": FieldSpec("prober_fallback"),
    "state": FieldSpec("state"),
}

_GTM_SERVER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "datacenter": FieldSpec("datacenter", ref_kind="gtm datacenter"),
    "monitor": FieldSpec("monitor"),
    "product": FieldSpec("product"),
    "addresses": FieldSpec("addresses"),
    "virtual-servers": FieldSpec("virtual_servers"),
    "description": FieldSpec("description"),
    "state": FieldSpec("state"),
    "prober-pool": FieldSpec("prober_pool", ref_kind="gtm prober-pool"),
    "prober-preference": FieldSpec("prober_preference"),
    "prober-fallback": FieldSpec("prober_fallback"),
    "virtual-server-discovery": FieldSpec("virtual_server_discovery"),
    "expose-route-domains": FieldSpec("expose_route_domains"),
    "iq-allow-path": FieldSpec("iq_allow_path"),
    "iq-allow-service-check": FieldSpec("iq_allow_service_check"),
    "iq-allow-snmp": FieldSpec("iq_allow_snmp"),
    "limit-max-bps": FieldSpec("limit_max_bps"),
    "limit-max-connections": FieldSpec("limit_max_connections"),
    "limit-max-pps": FieldSpec("limit_max_pps"),
}

_GTM_POOL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "record-type": FieldSpec("record_type"),
    "members": FieldSpec("members"),
    "monitor": FieldSpec("monitor"),
    "alternate-mode": FieldSpec("alternate_mode"),
    "fallback-mode": FieldSpec("fallback_mode"),
    "load-balancing-mode": FieldSpec("load_balancing_mode"),
    "ttl": FieldSpec("ttl"),
    "description": FieldSpec("description"),
    "state": FieldSpec("state"),
    "verify-member-availability": FieldSpec("verify_member_availability"),
    "fallback-ip": FieldSpec("fallback_ip"),
    "max-answers-returned": FieldSpec("max_answers_returned"),
    "qos-hit-ratio": FieldSpec("qos_hit_ratio"),
    "qos-hops": FieldSpec("qos_hops"),
    "qos-kbps": FieldSpec("qos_kbps"),
    "qos-lcs": FieldSpec("qos_lcs"),
    "qos-packet-rate": FieldSpec("qos_packet_rate"),
    "qos-rtt": FieldSpec("qos_rtt"),
    "qos-topology": FieldSpec("qos_topology"),
    "qos-vs-capacity": FieldSpec("qos_vs_capacity"),
    "qos-vs-score": FieldSpec("qos_vs_score"),
}

# Wideip ``pools[]`` is a list of paths into ``gtm pool <record-type>``
# — because pools are merged into one container keyed by full-path,
# the deref target is the unified ``gtm pool`` kind, and the
# record-type carried on each side is just metadata.
_GTM_WIDEIP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "record-type": FieldSpec("record_type"),
    "pools": FieldSpec("pools", ref_kind="gtm pool", list_ref=True),
    "aliases": FieldSpec("aliases"),
    "pool-lb-mode": FieldSpec("pool_lb_mode"),
    "last-resort-pool": FieldSpec("last_resort_pool", ref_kind="gtm pool"),
    "description": FieldSpec("description"),
    "state": FieldSpec("state"),
    "failure-rcode": FieldSpec("failure_rcode"),
    "failure-rcode-response": FieldSpec("failure_rcode_response"),
    "failure-rcode-ttl": FieldSpec("failure_rcode_ttl"),
    "minimal-response": FieldSpec("minimal_response"),
    "persistence": FieldSpec("persistence"),
    "persist-cidr-ipv4": FieldSpec("persist_cidr_ipv4"),
    "persist-cidr-ipv6": FieldSpec("persist_cidr_ipv6"),
    "topology-prefer-edns0-client-subnet": FieldSpec("topology_prefer_edns0_client_subnet"),
    "ttl-persistence": FieldSpec("ttl_persistence"),
}

_GTM_PROBER_POOL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "load-balancing-mode": FieldSpec("load_balancing_mode"),
    "members": FieldSpec("members", ref_kind="gtm server", list_ref=True),
    "state": FieldSpec("state"),
}

_GTM_REGION_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "region-members": FieldSpec("region_members"),
}

_GTM_RULE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "body": FieldSpec("source"),
    "description": FieldSpec("description"),
}

# Bundle 12 — gtm listeners / link / topology / distributed-app /
# global-settings.
_GTM_LISTENER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "address": FieldSpec("address"),
    "port": FieldSpec("port"),
    "ip-protocol": FieldSpec("ip_protocol"),
    "mask": FieldSpec("mask"),
    "pool": FieldSpec("pool", ref_kind="gtm pool"),
    "profiles": FieldSpec("profiles", ref_kind="ltm profile", list_ref=True),
    "rules": FieldSpec("rules", ref_kind="gtm rule", list_ref=True),
    "source-address-translation": FieldSpec("source_address_translation"),
    "state": FieldSpec("state"),
    "vlans": FieldSpec("vlans", ref_kind="net vlan", list_ref=True),
    "vlans-disabled": FieldSpec("vlans_disabled"),
    "vlans-enabled": FieldSpec("vlans_enabled"),
}

_GTM_LISTENER_DOH_PROXY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "address": FieldSpec("address"),
    "port": FieldSpec("port"),
    "pool": FieldSpec("pool", ref_kind="gtm pool"),
}

_GTM_LISTENER_DOH_SERVER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "address": FieldSpec("address"),
    "port": FieldSpec("port"),
    "pool": FieldSpec("pool", ref_kind="gtm pool"),
}

_GTM_LINK_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "datacenter": FieldSpec("datacenter", ref_kind="gtm datacenter"),
    "monitor": FieldSpec("monitor", ref_kind="gtm monitor"),
    "prober-pool": FieldSpec("prober_pool", ref_kind="gtm prober-pool"),
    "state": FieldSpec("state"),
    "weight": FieldSpec("weight"),
}

_GTM_DISTRIBUTED_APP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "wide-ips": FieldSpec("wide_ips", ref_kind="gtm wideip", list_ref=True),
    "persist-cidr": FieldSpec("persist_cidr"),
    "dependency-level": FieldSpec("dependency_level"),
}

_GTM_TOPOLOGY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "order": FieldSpec("order"),
    "score": FieldSpec("score"),
}

_GTM_GLOBAL_SETTINGS_GENERAL_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "auto-discovery": FieldSpec("auto_discovery"),
    "synchronization": FieldSpec("synchronization"),
    "synchronization-group-name": FieldSpec("synchronization_group_name"),
}

_GTM_GLOBAL_SETTINGS_LOAD_BALANCING_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "topology-longest-match": FieldSpec("topology_longest_match"),
    "ignore-path-ttl": FieldSpec("ignore_path_ttl"),
    "respect-dependent-objects": FieldSpec("respect_dependent_objects"),
    "verify-vs-availability": FieldSpec("verify_vs_availability"),
}

_GTM_GLOBAL_SETTINGS_METRICS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "metrics-collection-protocols": FieldSpec("metrics_collection_protocols"),
}

_GTM_GLOBAL_SETTINGS_METRICS_EXCLUSIONS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "addresses": FieldSpec("addresses"),
}

_PEM_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "rules": FieldSpec("rules"),
}

_PEM_RULE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "body": FieldSpec("source"),
    "description": FieldSpec("description"),
}

_PEM_LISTENER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "profile-spm": FieldSpec("profile_spm", ref_kind="pem profile"),
    "profile-subscriber-mgmt": FieldSpec("profile_subscriber_mgmt", ref_kind="pem profile"),
    "virtual-servers": FieldSpec("virtual_servers", ref_kind="ltm virtual", list_ref=True),
}

_PEM_FORWARDING_ENDPOINT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "pool": FieldSpec("pool", ref_kind="ltm pool"),
    "snat-pool": FieldSpec("snat_pool", ref_kind="ltm snatpool"),
    "source-ip": FieldSpec("source_ip"),
    "destination-ip": FieldSpec("destination_ip"),
    "type": FieldSpec("type_"),
    "persistence": FieldSpec("persistence"),
    "translate-address": FieldSpec("translate_address"),
    "translate-service": FieldSpec("translate_service"),
    "fallback": FieldSpec("fallback"),
}

_PEM_INTERCEPTION_ENDPOINT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "pool": FieldSpec("pool", ref_kind="ltm pool"),
    "persistence": FieldSpec("persistence"),
}

_PEM_SERVICE_CHAIN_ENDPOINT_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "service-endpoints": FieldSpec("service_endpoints"),
    "steering-policy": FieldSpec("steering_policy", ref_kind="pem policy"),
}

_PEM_PROFILE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "type": FieldSpec("profile_type"),
    "defaults-from": FieldSpec("defaults_from", ref_kind="pem profile"),
    "description": FieldSpec("description"),
}

_PEM_RATING_GROUP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "rating-group-id": FieldSpec("rating_group_id"),
    "default-quota": FieldSpec("default_quota"),
    "default-quota-holding-time": FieldSpec("default_quota_holding_time"),
    "default-validity-time": FieldSpec("default_validity_time"),
    "default-threshold": FieldSpec("default_threshold"),
    "total-octets": FieldSpec("total_octets"),
    "input-octets": FieldSpec("input_octets"),
    "output-octets": FieldSpec("output_octets"),
    "time": FieldSpec("time"),
    "consumption-time": FieldSpec("consumption_time"),
    "usage-time": FieldSpec("usage_time"),
    "volume": FieldSpec("volume"),
}

_AUTH_PARTITION_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "default-route-domain": FieldSpec("default_route_domain", ref_kind="net route-domain"),
    "inherited-traffic-group": FieldSpec("inherited_traffic_group"),
}

_AUTH_USER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "description": FieldSpec("description"),
    "partition": FieldSpec("partition", ref_kind="auth partition"),
    "shell": FieldSpec("shell"),
    "encrypted-password": FieldSpec("encrypted_password"),
    "partition-access": FieldSpec("partition_access"),
}

_AUTH_PASSWORD_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "expiration-warning": FieldSpec("expiration_warning"),
    "minimum-length": FieldSpec("minimum_length"),
    "policy": FieldSpec("policy"),
}

_AUTH_PASSWORD_POLICY_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "expiration-warning": FieldSpec("expiration_warning"),
    "max-duration": FieldSpec("max_duration"),
    "max-login-failures": FieldSpec("max_login_failures"),
    "min-duration": FieldSpec("min_duration"),
    "minimum-length": FieldSpec("minimum_length"),
    "minimum-regular-characters": FieldSpec("minimum_regular_characters"),
    "password-memory": FieldSpec("password_memory"),
    "policy-enforcement": FieldSpec("policy_enforcement"),
    "required-lowercase": FieldSpec("required_lowercase"),
    "required-numeric": FieldSpec("required_numeric"),
    "required-special": FieldSpec("required_special"),
    "required-uppercase": FieldSpec("required_uppercase"),
}

_AUTH_SOURCE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "fallback": FieldSpec("fallback"),
    "type": FieldSpec("type_"),
}

_AUTH_REMOTE_ROLE_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "role-info": FieldSpec("role_info"),
}

_AUTH_REMOTE_USER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "default-partition": FieldSpec("default_partition"),
    "default-role": FieldSpec("default_role"),
    "remote-console-access": FieldSpec("remote_console_access"),
}

_AUTH_LOGIN_FAILURES_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
}

_AUTH_LDAP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "bind-dn": FieldSpec("bind_dn"),
    "bind-pw": FieldSpec("bind_pw"),
    "bind-timeout": FieldSpec("bind_timeout"),
    "check-host-attr": FieldSpec("check_host_attr"),
    "check-roles-group": FieldSpec("check_roles_group"),
    "filter": FieldSpec("filter_"),
    "group-dn": FieldSpec("group_dn"),
    "group-member-attribute": FieldSpec("group_member_attribute"),
    "idle-timeout": FieldSpec("idle_timeout"),
    "ignore-auth-info-unavail": FieldSpec("ignore_auth_info_unavail"),
    "ignore-unknown-user": FieldSpec("ignore_unknown_user"),
    "login-attribute": FieldSpec("login_attribute"),
    "port": FieldSpec("port"),
    "scope": FieldSpec("scope"),
    "search-base-dn": FieldSpec("search_base_dn"),
    "search-timeout": FieldSpec("search_timeout"),
    "servers": FieldSpec("servers"),
    "ssl": FieldSpec("ssl"),
    "ssl-ca-cert": FieldSpec("ssl_ca_cert"),
    "ssl-check-peer": FieldSpec("ssl_check_peer"),
    "ssl-client-cert": FieldSpec("ssl_client_cert"),
    "ssl-client-key": FieldSpec("ssl_client_key"),
    "user-template": FieldSpec("user_template"),
    "version": FieldSpec("version"),
}

_AUTH_RADIUS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "service-type": FieldSpec("service_type"),
    "servers": FieldSpec("servers", ref_kind="auth radius-server", list_ref=True),
}

_AUTH_RADIUS_SERVER_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "server": FieldSpec("server"),
    "port": FieldSpec("port"),
    "secret": FieldSpec("secret"),
    "timeout": FieldSpec("timeout"),
}

_AUTH_TACACS_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "protocol": FieldSpec("protocol"),
    "secret": FieldSpec("secret"),
    "service": FieldSpec("service"),
    "servers": FieldSpec("servers"),
    "accounting": FieldSpec("accounting"),
    "authentication": FieldSpec("authentication"),
    "debug": FieldSpec("debug"),
    "encryption": FieldSpec("encryption"),
}

_AUTH_CERT_LDAP_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "bind-dn": FieldSpec("bind_dn"),
    "bind-pw": FieldSpec("bind_pw"),
    "bind-timeout": FieldSpec("bind_timeout"),
    "idle-timeout": FieldSpec("idle_timeout"),
    "login-attribute": FieldSpec("login_attribute"),
    "port": FieldSpec("port"),
    "scope": FieldSpec("scope"),
    "search-base-dn": FieldSpec("search_base_dn"),
    "search-timeout": FieldSpec("search_timeout"),
    "servers": FieldSpec("servers"),
    "ssl": FieldSpec("ssl"),
    "user-template": FieldSpec("user_template"),
    "version": FieldSpec("version"),
}

_AUTH_APM_AUTH_FIELDS: dict[str, FieldSpec] = {
    "name": FieldSpec("name"),
    "full-path": FieldSpec("full_path"),
    "profile": FieldSpec("profile", ref_kind="apm policy access-policy"),
}


_KIND_FIELD_MAPS: dict[str, tuple[type, dict[str, FieldSpec]]] = {
    "ltm virtual": (BigipVirtualServer, _VS_FIELDS),
    "ltm virtual-address": (BigipVirtualAddress, _VIRTUAL_ADDRESS_FIELDS),
    # Bundle 13 — ltm.* cross-cutting infra.
    "ltm cipher group": (BigipLtmCipherGroup, _LTM_CIPHER_GROUP_FIELDS),
    "ltm cipher rule": (BigipLtmCipherRule, _LTM_CIPHER_RULE_FIELDS),
    "ltm nat": (BigipLtmNat, _LTM_NAT_FIELDS),
    "ltm snat": (BigipLtmSnat, _LTM_SNAT_FIELDS),
    "ltm snat-translation": (BigipLtmSnatTranslation, _LTM_SNAT_TRANSLATION_FIELDS),
    "ltm policy-strategy": (BigipLtmPolicyStrategy, _LTM_POLICY_STRATEGY_FIELDS),
    "ltm traffic-class": (BigipLtmTrafficClass, _LTM_TRAFFIC_CLASS_FIELDS),
    "ltm traffic-matching-criteria": (
        BigipLtmTrafficMatchingCriteria,
        _LTM_TRAFFIC_MATCHING_CRITERIA_FIELDS,
    ),
    "ltm ifile": (BigipLtmIfile, _LTM_IFILE_FIELDS),
    "ltm eviction-policy": (BigipLtmEvictionPolicy, _LTM_EVICTION_POLICY_FIELDS),
    # Bundle 14 — ltm dns.* (DNS Express).
    "ltm dns nameserver": (BigipLtmDnsNameserver, _LTM_DNS_NAMESERVER_FIELDS),
    "ltm dns tsig-key": (BigipLtmDnsTsigKey, _LTM_DNS_TSIG_KEY_FIELDS),
    "ltm dns zone": (BigipLtmDnsZone, _LTM_DNS_ZONE_FIELDS),
    "ltm dns dnssec key": (BigipLtmDnsDnssecKey, _LTM_DNS_DNSSEC_KEY_FIELDS),
    "ltm dns dnssec zone": (BigipLtmDnsDnssecZone, _LTM_DNS_DNSSEC_ZONE_FIELDS),
    "ltm dns cache resolver": (BigipLtmDnsCacheResolver, _LTM_DNS_CACHE_RESOLVER_FIELDS),
    "ltm dns cache transparent": (
        BigipLtmDnsCacheTransparent,
        _LTM_DNS_CACHE_TRANSPARENT_FIELDS,
    ),
    "ltm dns cache validating-resolver": (
        BigipLtmDnsCacheValidatingResolver,
        _LTM_DNS_CACHE_VALIDATING_RESOLVER_FIELDS,
    ),
    "ltm dns cache global-settings": (
        BigipLtmDnsCacheGlobalSettings,
        _LTM_DNS_CACHE_GLOBAL_SETTINGS_FIELDS,
    ),
    "ltm dns cache records": (BigipLtmDnsCacheRecord, _LTM_DNS_CACHE_RECORD_FIELDS),
    "ltm dns hpke key": (BigipLtmDnsHpkeKey, _LTM_DNS_HPKE_KEY_FIELDS),
    "ltm dns hpke profile": (BigipLtmDnsHpkeProfile, _LTM_DNS_HPKE_PROFILE_FIELDS),
    "ltm dns analytics global-settings": (
        BigipLtmDnsAnalyticsGlobalSettings,
        _LTM_DNS_ANALYTICS_GLOBAL_SETTINGS_FIELDS,
    ),
    # Bundle 15 — ltm message-routing.* (20 kinds, shared shape).
    "ltm message-routing diameter peer": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing diameter route": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing diameter profile router": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing diameter profile session": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing diameter transport-config": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing sip peer": (BigipLtmMessageRoutingObject, _LTM_MESSAGE_ROUTING_FIELDS),
    "ltm message-routing sip route": (BigipLtmMessageRoutingObject, _LTM_MESSAGE_ROUTING_FIELDS),
    "ltm message-routing sip profile router": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing sip profile session": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing sip transport-config": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing mqtt peer": (BigipLtmMessageRoutingObject, _LTM_MESSAGE_ROUTING_FIELDS),
    "ltm message-routing mqtt route": (BigipLtmMessageRoutingObject, _LTM_MESSAGE_ROUTING_FIELDS),
    "ltm message-routing mqtt profile router": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing mqtt profile session": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing mqtt transport-config": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing generic peer": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing generic protocol": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing generic route": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing generic router": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    "ltm message-routing generic transport-config": (
        BigipLtmMessageRoutingObject,
        _LTM_MESSAGE_ROUTING_FIELDS,
    ),
    # Bundle 16 — ltm auth.* profiles (11 kinds).
    "ltm auth profile": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth ldap": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth radius": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth radius-server": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth tacacs": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth crldp-server": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth ocsp-responder": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth kerberos-delegation": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth ssl-cc-ldap": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth ssl-crldp": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    "ltm auth ssl-ocsp": (BigipLtmAuthObject, _LTM_AUTH_FIELDS),
    # Bundle 17 — ltm CGNAT / LSN (3 kinds).
    "ltm lsn-pool": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm lsn-log-profile": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm alg-log-profile": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    # Bundle 18 — ltm global-settings + misc singletons.
    "ltm default-node-monitor": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm global-settings connection": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm global-settings general": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm global-settings rule": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm global-settings traffic-control": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm rule-profiler": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    # Bundle 19 — ltm classification + clientssl (11 kinds).
    "ltm classification application": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm classification auto-update settings": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm classification category": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm classification ce": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm classification signature-update-schedule": (
        BigipLtmMinimalObject,
        _LTM_MINIMAL_FIELDS,
    ),
    "ltm classification url-cat-policy": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm classification url-category": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm classification urldb-feed-list": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm classification urldb-file": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm clientssl ocsp-stapling-responses": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm clientssl-proxy cached-certs": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    # Bundle 20 — ltm tacdb (3 kinds).
    "ltm tacdb customdb": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm tacdb customdb-file": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    "ltm tacdb licenseddb": (BigipLtmMinimalObject, _LTM_MINIMAL_FIELDS),
    # Bundle 21-26 — net.* minimal kinds.
    "net routing access-list": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net routing bfd": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net routing bgp": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net routing community-list": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net routing extcommunity-list": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net routing prefix-list": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net routing profile bgp": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net routing route-map": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net routing debug": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net router-advertisement": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels endpoint": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels etherip": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels fec": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels geneve": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels gre": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels ipip": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels ipsec": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels lw4o6": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels map": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels ppp": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels tcp-forward": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels v6rd": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels vxlan": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net tunnels wccp": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net ipsec ike-daemon": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net ipsec ike-peer": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net ipsec ipsec-policy": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net ipsec manual-security-association": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net ipsec traffic-selector": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net bwc policy": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net bwc priority-group": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net bwc traffic-group": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net cos global-settings": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net cos map-8021p": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net cos map-dscp": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net cos traffic-priority": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net rate-shaping class": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net rate-shaping color-policer": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net rate-shaping drop-policy": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net rate-shaping queue": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net rate-shaping shaping-policy": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net address-list": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net arp": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net dag-globals": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net fdb tunnel": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net fdb vlan": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net interface-cos": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net ipv6-subscriber-prefix-length": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net lacp-globals": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net lldp-globals": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net multicast-globals": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net ndp": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net packet-filter": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net packet-filter-trusted": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net port-mirror": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net rst-cause": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net self-allow": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net service-policy": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net stp-globals": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net timer-policy": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net trunk": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net vlan-group": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net wccp": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net sfc chain": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    "net sfc sf": (BigipNetMinimalObject, _NET_MINIMAL_FIELDS),
    # Bundles 27-31 — apm.* minimal kinds.
    "apm aaa active-directory": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa active-directory-trusted-domains": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa crldp": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa endpoint-management-system": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa f5-mfa-configuration": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa f5-service-connector": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa http": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa http-connector-request": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa http-connector-transport": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa kerberos": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa kerberos-keytab-file": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa ldap": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa oam": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa oauth-provider": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa oauth-request": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa oauth-server": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa ocsp": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa okta-connector": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa radius": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa saml": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa saml-idp-automation": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa saml-idp-connector": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa securid": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm aaa tacacsplus": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm profile access": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm profile connectivity": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm profile exchange": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm profile oauth": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm profile vdi": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso basic": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso form-based": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso form-basedv2": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso kerberos": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso ntlmv1": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso ntlmv2": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso oauth-bearer": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso saml": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso saml-resource": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso saml-sp-automation": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm sso saml-sp-connector": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource address-space": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource app-tunnel": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource client-rate-class": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource client-traffic-classifier": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource ipv6-leasepool": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource leasepool": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource network-access": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource portal-access": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource remote-desktop citrix": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource remote-desktop citrix-client-bundle": (
        BigipApmMinimalObject,
        _APM_MINIMAL_FIELDS,
    ),
    "apm resource remote-desktop citrix-client-package-file": (
        BigipApmMinimalObject,
        _APM_MINIMAL_FIELDS,
    ),
    "apm resource remote-desktop quest": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource remote-desktop rdp": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource remote-desktop vmware-view": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource sandbox": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource webtop": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm resource webtop-link": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm oauth jwk-config": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm oauth jwt-config": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm oauth jwt-provider-list": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm oauth oauth-claim": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm oauth oauth-client-app": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm oauth oauth-resource-server": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm oauth oauth-scope": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm saml artifact-resolution-service": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm saml attribute-consuming-service": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm saml auth-context-class-list": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm ntlm machine-account": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm ntlm ntlm-auth": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm acl": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm log-setting": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm url-filter": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm swg-scheme": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm client image": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm configuration captcha": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm epsec epsec-package": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm apm-avr-config": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm report custom-report-field": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm policy customization-group": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm policy customization-languages": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm policy image-file": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "apm policy windows-group-policy-file": (BigipApmMinimalObject, _APM_MINIMAL_FIELDS),
    "ltm pool": (BigipPool, _POOL_FIELDS),
    "ltm node": (BigipNode, _NODE_FIELDS),
    "ltm rule": (BigipRule, _RULE_FIELDS),
    "ltm profile": (BigipProfile, _PROFILE_FIELDS),
    "ltm monitor": (BigipMonitor, _MONITOR_FIELDS),
    "ltm persistence": (BigipPersistence, _PERSIST_FIELDS),
    "ltm snatpool": (BigipSnatPool, _SNATPOOL_FIELDS),
    "ltm policy": (BigipPolicy, _POLICY_FIELDS),
    "ltm data-group": (BigipDataGroup, _DATAGROUP_FIELDS),
    "net route": (BigipNetRoute, _NET_ROUTE_FIELDS),
    "net vlan": (BigipNetVlan, _NET_VLAN_FIELDS),
    "net self": (BigipNetSelf, _NET_SELF_FIELDS),
    "net route-domain": (BigipNetRouteDomain, _NET_ROUTE_DOMAIN_FIELDS),
    "net port-list": (BigipNetPortList, _NET_PORT_LIST_FIELDS),
    "net interface": (BigipNetInterface, _NET_INTERFACE_FIELDS),
    "net dns-resolver": (BigipNetDnsResolver, _NET_DNS_RESOLVER_FIELDS),
    "net tunnels tunnel": (BigipNetTunnel, _NET_TUNNEL_FIELDS),
    "net stp": (BigipNetStp, _NET_STP_FIELDS),
    "sys dns": (BigipSysDns, _SYS_DNS_FIELDS),
    "sys ntp": (BigipSysNtp, _SYS_NTP_FIELDS),
    "sys snmp": (BigipSysSnmp, _SYS_SNMP_FIELDS),
    "sys global-settings": (BigipSysGlobalSettings, _SYS_GLOBAL_SETTINGS_FIELDS),
    "sys provision": (BigipSysProvision, _SYS_PROVISION_FIELDS),
    "sys folder": (BigipSysFolder, _SYS_FOLDER_FIELDS),
    "sys file ssl-cert": (BigipSysFileSslCert, _SYS_FILE_SSL_CERT_FIELDS),
    "sys file ssl-key": (BigipSysFileSslKey, _SYS_FILE_SSL_KEY_FIELDS),
    "sys management-route": (BigipSysManagementRoute, _SYS_MANAGEMENT_ROUTE_FIELDS),
    "security firewall port-list": (
        BigipSecurityFirewallPortList,
        _SECURITY_FW_PORT_LIST_FIELDS,
    ),
    "security firewall rule-list": (
        BigipSecurityFirewallRuleList,
        _SECURITY_FW_RULE_LIST_FIELDS,
    ),
    "security firewall config-entity-id": (
        BigipSecurityFirewallConfigEntityId,
        _SECURITY_FW_CONFIG_ENTITY_ID_FIELDS,
    ),
    "security firewall policy": (BigipSecurityFirewallPolicy, _SECURITY_FW_POLICY_FIELDS),
    "security firewall address-list": (
        BigipSecurityFirewallAddressList,
        _SECURITY_FW_ADDRESS_LIST_FIELDS,
    ),
    "security firewall global-rules": (
        BigipSecurityFirewallGlobalRules,
        _SECURITY_FW_GLOBAL_RULES_FIELDS,
    ),
    "security firewall management-ip-rules": (
        BigipSecurityFirewallManagementIpRules,
        _SECURITY_FW_MANAGEMENT_IP_RULES_FIELDS,
    ),
    "security firewall schedule": (
        BigipSecurityFirewallSchedule,
        _SECURITY_FW_SCHEDULE_FIELDS,
    ),
    "security firewall user-list": (
        BigipSecurityFirewallUserList,
        _SECURITY_FW_USER_LIST_FIELDS,
    ),
    "security firewall user-domain": (
        BigipSecurityFirewallUserDomain,
        _SECURITY_FW_USER_DOMAIN_FIELDS,
    ),
    "security firewall global-fqdn-policy": (
        BigipSecurityFirewallGlobalFqdnPolicy,
        _SECURITY_FW_GLOBAL_FQDN_POLICY_FIELDS,
    ),
    "security firewall port-misuse-policy": (
        BigipSecurityFirewallPortMisusePolicy,
        _SECURITY_FW_PORT_MISUSE_POLICY_FIELDS,
    ),
    "security firewall on-demand-compilation": (
        BigipSecurityFirewallOnDemandCompilation,
        _SECURITY_FW_ON_DEMAND_COMPILATION_FIELDS,
    ),
    "security firewall on-demand-rule-deploy": (
        BigipSecurityFirewallOnDemandRuleDeploy,
        _SECURITY_FW_ON_DEMAND_RULE_DEPLOY_FIELDS,
    ),
    "security firewall uuid-default-autogenerate": (
        BigipSecurityFirewallUuidDefaultAutogenerate,
        _SECURITY_FW_UUID_DEFAULT_AUTOGENERATE_FIELDS,
    ),
    "security firewall config-change-log": (
        BigipSecurityFirewallConfigChangeLog,
        _SECURITY_FW_CONFIG_CHANGE_LOG_FIELDS,
    ),
    "security nat policy": (BigipSecurityNatPolicy, _SECURITY_NAT_POLICY_FIELDS),
    "security nat source-translation": (
        BigipSecurityNatSourceTranslation,
        _SECURITY_NAT_SOURCE_TRANSLATION_FIELDS,
    ),
    "security nat destination-translation": (
        BigipSecurityNatDestinationTranslation,
        _SECURITY_NAT_DESTINATION_TRANSLATION_FIELDS,
    ),
    "security log profile": (BigipSecurityLogProfile, _SECURITY_LOG_PROFILE_FIELDS),
    "security dos profile": (BigipSecurityDosProfile, _SECURITY_DOS_PROFILE_FIELDS),
    "security ip-intelligence feed-list": (
        BigipSecurityIpIntelligenceFeedList,
        _SECURITY_IP_INTELLIGENCE_FEED_LIST_FIELDS,
    ),
    "security ip-intelligence global-policy": (
        BigipSecurityIpIntelligenceGlobalPolicy,
        _SECURITY_IP_INTELLIGENCE_GLOBAL_POLICY_FIELDS,
    ),
    "security zone": (BigipSecurityZone, _SECURITY_ZONE_FIELDS),
    "security protected zone": (BigipSecurityProtectedZone, _SECURITY_PROTECTED_ZONE_FIELDS),
    "security packet-filter policy": (
        BigipSecurityPacketFilterPolicy,
        _SECURITY_PACKET_FILTER_POLICY_FIELDS,
    ),
    "security packet-filter default-rules": (
        BigipSecurityPacketFilterDefaultRules,
        _SECURITY_PACKET_FILTER_DEFAULT_RULES_FIELDS,
    ),
    "security ssh profile": (BigipSecuritySshProfile, _SECURITY_SSH_PROFILE_FIELDS),
    "security http profile": (BigipSecurityHttpProfile, _SECURITY_HTTP_PROFILE_FIELDS),
    "security bot-defense profile": (
        BigipSecurityBotDefenseProfile,
        _SECURITY_BOT_DEFENSE_PROFILE_FIELDS,
    ),
    # Bundle 10b — minimal security.* kinds.  All 37 share
    # ``BigipSecurityMinimalObject`` + ``_SECURITY_MINIMAL_FIELDS``
    # but get distinct kind strings so the projection routes to
    # the right ``BigipConfig`` attribute.
    "security analytics settings": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security anti-fraud profile": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security anti-fraud signatures-update": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security blacklist-publisher category": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security blacklist-publisher profile": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security bot-defense signature": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security bot-defense signature-category": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security cloud-services connector": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security datasync background-tasks": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security datasync global-profile": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security datasync local-profile": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security debug drop-redirect-stats": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security debug matcher": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security debug register": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security device device-context": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos autodos-file-object": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos behavioral-signature": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos bot-signature": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos bot-signature-category": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security dos device-config": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos dns-nxdomain-stat": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos dos-signature": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos dynamic-signatures": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos ip-uncommon-protolist": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos l4bdos-file-object": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos network-whitelist": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos stress-stats": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos udp-portlist": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security dos virtual": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security flowspec-route-injector profile": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security ip-intelligence blacklist-category": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security protocol-inspection common-config": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security protocol-inspection learning-stats": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security protocol-inspection profile": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security protocol-inspection signature": (
        BigipSecurityMinimalObject,
        _SECURITY_MINIMAL_FIELDS,
    ),
    "security scrubber profile": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security ssh ciphers": (BigipSecurityMinimalObject, _SECURITY_MINIMAL_FIELDS),
    "security ip-intelligence policy": (
        BigipSecurityIpIntelligencePolicy,
        _SECURITY_IP_INTELLIGENCE_POLICY_FIELDS,
    ),
    "security protocol-inspection compliance-map": (
        BigipSecurityProtocolInspectionComplianceMap,
        _SECURITY_PI_COMPLIANCE_MAP_FIELDS,
    ),
    "security protocol-inspection compliance-objects": (
        BigipSecurityProtocolInspectionComplianceObject,
        _SECURITY_PI_COMPLIANCE_OBJECT_FIELDS,
    ),
    "security device-id attribute": (
        BigipSecurityDeviceIdAttribute,
        _SECURITY_DEVICE_ID_ATTRIBUTE_FIELDS,
    ),
    "apm ephemeral-auth ssh-security-config": (
        BigipApmEphemeralAuthSshSecurityConfig,
        _APM_SSH_SECURITY_CONFIG_FIELDS,
    ),
    "apm oauth db-instance": (BigipApmOauthDbInstance, _APM_OAUTH_DB_INSTANCE_FIELDS),
    "apm policy access-policy": (
        BigipApmPolicyAccessPolicy,
        _APM_POLICY_ACCESS_POLICY_FIELDS,
    ),
    "apm policy customization-source": (
        BigipApmPolicyCustomizationSource,
        _APM_POLICY_CUSTOMIZATION_SOURCE_FIELDS,
    ),
    "apm policy policy-item": (BigipApmPolicyItem, _APM_POLICY_ITEM_FIELDS),
    "apm policy agent": (BigipApmPolicyAgent, _APM_POLICY_AGENT_FIELDS),
    "apm report default-report": (
        BigipApmReportDefaultReport,
        _APM_REPORT_DEFAULT_REPORT_FIELDS,
    ),
    "cm cert": (BigipCmCert, _CM_CERT_FIELDS),
    "cm key": (BigipCmKey, _CM_KEY_FIELDS),
    "cm device": (BigipCmDevice, _CM_DEVICE_FIELDS),
    "cm device-group": (BigipCmDeviceGroup, _CM_DEVICE_GROUP_FIELDS),
    "cm traffic-group": (BigipCmTrafficGroup, _CM_TRAFFIC_GROUP_FIELDS),
    "cm trust-domain": (BigipCmTrustDomain, _CM_TRUST_DOMAIN_FIELDS),
    "gtm datacenter": (BigipGtmDatacenter, _GTM_DATACENTER_FIELDS),
    "gtm server": (BigipGtmServer, _GTM_SERVER_FIELDS),
    "gtm pool": (BigipGtmPool, _GTM_POOL_FIELDS),
    "gtm wideip": (BigipGtmWideip, _GTM_WIDEIP_FIELDS),
    "gtm prober-pool": (BigipGtmProberPool, _GTM_PROBER_POOL_FIELDS),
    "gtm region": (BigipGtmRegion, _GTM_REGION_FIELDS),
    "gtm rule": (BigipGtmRule, _GTM_RULE_FIELDS),
    # Bundle 12 — gtm listeners / link / topology / distributed-app /
    # global-settings singletons.
    "gtm listener": (BigipGtmListener, _GTM_LISTENER_FIELDS),
    "gtm listener-doh-proxy": (BigipGtmListenerDohProxy, _GTM_LISTENER_DOH_PROXY_FIELDS),
    "gtm listener-doh-server": (BigipGtmListenerDohServer, _GTM_LISTENER_DOH_SERVER_FIELDS),
    "gtm link": (BigipGtmLink, _GTM_LINK_FIELDS),
    "gtm distributed-app": (BigipGtmDistributedApp, _GTM_DISTRIBUTED_APP_FIELDS),
    "gtm topology": (BigipGtmTopology, _GTM_TOPOLOGY_FIELDS),
    "gtm global-settings general": (
        BigipGtmGlobalSettingsGeneral,
        _GTM_GLOBAL_SETTINGS_GENERAL_FIELDS,
    ),
    "gtm global-settings load-balancing": (
        BigipGtmGlobalSettingsLoadBalancing,
        _GTM_GLOBAL_SETTINGS_LOAD_BALANCING_FIELDS,
    ),
    "gtm global-settings metrics": (
        BigipGtmGlobalSettingsMetrics,
        _GTM_GLOBAL_SETTINGS_METRICS_FIELDS,
    ),
    "gtm global-settings metrics-exclusions": (
        BigipGtmGlobalSettingsMetricsExclusions,
        _GTM_GLOBAL_SETTINGS_METRICS_EXCLUSIONS_FIELDS,
    ),
    # Bundle 11 — gtm monitor.*  Reuses the existing BigipMonitor
    # dataclass + field map; ``monitor-type`` distinguishes the 31
    # protocol variants (bigip, bigip-link, http, https, ldap, …)
    # within the merged container.
    "gtm monitor": (BigipMonitor, _GTM_MONITOR_FIELDS),
    "pem policy": (BigipPemPolicy, _PEM_POLICY_FIELDS),
    "pem rule": (BigipPemRule, _PEM_RULE_FIELDS),
    "pem listener": (BigipPemListener, _PEM_LISTENER_FIELDS),
    "pem forwarding-endpoint": (
        BigipPemForwardingEndpoint,
        _PEM_FORWARDING_ENDPOINT_FIELDS,
    ),
    "pem interception-endpoint": (
        BigipPemInterceptionEndpoint,
        _PEM_INTERCEPTION_ENDPOINT_FIELDS,
    ),
    "pem service-chain-endpoint": (
        BigipPemServiceChainEndpoint,
        _PEM_SERVICE_CHAIN_ENDPOINT_FIELDS,
    ),
    "pem profile": (BigipPemProfile, _PEM_PROFILE_FIELDS),
    "pem quota-mgmt rating-group": (BigipPemRatingGroup, _PEM_RATING_GROUP_FIELDS),
    # Bundle 32 — pem.* minimal kinds.
    "pem global-settings analytics": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem global-settings gx": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem global-settings hsl-flow": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem global-settings hsl-report": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem global-settings insert-content": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem global-settings policy": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem global-settings quota-mgmt": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem global-settings session-mgmt-attributes": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem global-settings subscriber-activity-log": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem protocol diameter-avp": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem protocol radius-avp": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem protocol profile gx": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem protocol profile radius": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem reporting format-script": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem subscriber": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "pem subscriber-attribute": (BigipPemMinimalObject, _PEM_MINIMAL_FIELDS),
    "auth partition": (BigipAuthPartition, _AUTH_PARTITION_FIELDS),
    "auth user": (BigipAuthUser, _AUTH_USER_FIELDS),
    "auth password": (BigipAuthPassword, _AUTH_PASSWORD_FIELDS),
    "auth password-policy": (BigipAuthPasswordPolicy, _AUTH_PASSWORD_POLICY_FIELDS),
    "auth source": (BigipAuthSource, _AUTH_SOURCE_FIELDS),
    "auth remote-role": (BigipAuthRemoteRole, _AUTH_REMOTE_ROLE_FIELDS),
    "auth remote-user": (BigipAuthRemoteUser, _AUTH_REMOTE_USER_FIELDS),
    "auth login-failures": (BigipAuthLoginFailures, _AUTH_LOGIN_FAILURES_FIELDS),
    "auth ldap": (BigipAuthLdap, _AUTH_LDAP_FIELDS),
    "auth radius": (BigipAuthRadius, _AUTH_RADIUS_FIELDS),
    "auth radius-server": (BigipAuthRadiusServer, _AUTH_RADIUS_SERVER_FIELDS),
    "auth tacacs": (BigipAuthTacacs, _AUTH_TACACS_FIELDS),
    "auth cert-ldap": (BigipAuthCertLdap, _AUTH_CERT_LDAP_FIELDS),
    "auth apm-auth": (BigipAuthApmAuth, _AUTH_APM_AUTH_FIELDS),
}

# Per-module kind tables.  Each entry is a mapping from the **container
# key** (what users write between dots — ``virtual``, ``route``, …) to
# ``(BigipConfig attribute, TMSH kind)``.  Adding a new kind: drop a
# dataclass + field map above, register the parser, add an entry here
# and the navigable path ``.<module>.<kind>`` lights up.
_MODULE_KINDS: dict[str, dict[str, tuple[str, str]]] = {
    "ltm": {
        "virtual": ("virtual_servers", "ltm virtual"),
        "virtual-address": ("virtual_addresses", "ltm virtual-address"),
        # Bundle 13 — ltm.* cross-cutting infra.
        "cipher-group": ("ltm_cipher_groups", "ltm cipher group"),
        "cipher-rule": ("ltm_cipher_rules", "ltm cipher rule"),
        "nat": ("ltm_nats", "ltm nat"),
        "snat": ("ltm_snats", "ltm snat"),
        "snat-translation": ("ltm_snat_translations", "ltm snat-translation"),
        "policy-strategy": ("ltm_policy_strategies", "ltm policy-strategy"),
        "traffic-class": ("ltm_traffic_classes", "ltm traffic-class"),
        "traffic-matching-criteria": (
            "ltm_traffic_matching_criteria",
            "ltm traffic-matching-criteria",
        ),
        "ifile": ("ltm_ifiles", "ltm ifile"),
        "eviction-policy": ("ltm_eviction_policies", "ltm eviction-policy"),
        # Bundle 14 — ltm dns.* (DNS Express).
        "dns-nameserver": ("ltm_dns_nameservers", "ltm dns nameserver"),
        "dns-tsig-key": ("ltm_dns_tsig_keys", "ltm dns tsig-key"),
        "dns-zone": ("ltm_dns_zones", "ltm dns zone"),
        "dns-dnssec-key": ("ltm_dns_dnssec_keys", "ltm dns dnssec key"),
        "dns-dnssec-zone": ("ltm_dns_dnssec_zones", "ltm dns dnssec zone"),
        "dns-cache-resolver": ("ltm_dns_cache_resolvers", "ltm dns cache resolver"),
        "dns-cache-transparent": (
            "ltm_dns_cache_transparent",
            "ltm dns cache transparent",
        ),
        "dns-cache-validating-resolver": (
            "ltm_dns_cache_validating_resolvers",
            "ltm dns cache validating-resolver",
        ),
        "dns-cache-global-settings": (
            "ltm_dns_cache_global_settings",
            "ltm dns cache global-settings",
        ),
        "dns-cache-records": ("ltm_dns_cache_records", "ltm dns cache records"),
        "dns-hpke-key": ("ltm_dns_hpke_keys", "ltm dns hpke key"),
        "dns-hpke-profile": ("ltm_dns_hpke_profiles", "ltm dns hpke profile"),
        "dns-analytics-global-settings": (
            "ltm_dns_analytics_global_settings",
            "ltm dns analytics global-settings",
        ),
        # Bundle 15 — ltm message-routing.* (20 kinds).
        "message-routing-diameter-peer": (
            "ltm_mr_diameter_peers",
            "ltm message-routing diameter peer",
        ),
        "message-routing-diameter-route": (
            "ltm_mr_diameter_routes",
            "ltm message-routing diameter route",
        ),
        "message-routing-diameter-profile-router": (
            "ltm_mr_diameter_profile_router",
            "ltm message-routing diameter profile router",
        ),
        "message-routing-diameter-profile-session": (
            "ltm_mr_diameter_profile_session",
            "ltm message-routing diameter profile session",
        ),
        "message-routing-diameter-transport-config": (
            "ltm_mr_diameter_transport_config",
            "ltm message-routing diameter transport-config",
        ),
        "message-routing-sip-peer": ("ltm_mr_sip_peers", "ltm message-routing sip peer"),
        "message-routing-sip-route": ("ltm_mr_sip_routes", "ltm message-routing sip route"),
        "message-routing-sip-profile-router": (
            "ltm_mr_sip_profile_router",
            "ltm message-routing sip profile router",
        ),
        "message-routing-sip-profile-session": (
            "ltm_mr_sip_profile_session",
            "ltm message-routing sip profile session",
        ),
        "message-routing-sip-transport-config": (
            "ltm_mr_sip_transport_config",
            "ltm message-routing sip transport-config",
        ),
        "message-routing-mqtt-peer": ("ltm_mr_mqtt_peers", "ltm message-routing mqtt peer"),
        "message-routing-mqtt-route": ("ltm_mr_mqtt_routes", "ltm message-routing mqtt route"),
        "message-routing-mqtt-profile-router": (
            "ltm_mr_mqtt_profile_router",
            "ltm message-routing mqtt profile router",
        ),
        "message-routing-mqtt-profile-session": (
            "ltm_mr_mqtt_profile_session",
            "ltm message-routing mqtt profile session",
        ),
        "message-routing-mqtt-transport-config": (
            "ltm_mr_mqtt_transport_config",
            "ltm message-routing mqtt transport-config",
        ),
        "message-routing-generic-peer": (
            "ltm_mr_generic_peers",
            "ltm message-routing generic peer",
        ),
        "message-routing-generic-protocol": (
            "ltm_mr_generic_protocols",
            "ltm message-routing generic protocol",
        ),
        "message-routing-generic-route": (
            "ltm_mr_generic_routes",
            "ltm message-routing generic route",
        ),
        "message-routing-generic-router": (
            "ltm_mr_generic_routers",
            "ltm message-routing generic router",
        ),
        "message-routing-generic-transport-config": (
            "ltm_mr_generic_transport_config",
            "ltm message-routing generic transport-config",
        ),
        # Bundle 16 — ltm auth.* profiles (11 kinds).
        "auth-profile": ("ltm_auth_profiles", "ltm auth profile"),
        "auth-ldap": ("ltm_auth_ldap", "ltm auth ldap"),
        "auth-radius": ("ltm_auth_radius", "ltm auth radius"),
        "auth-radius-server": ("ltm_auth_radius_servers", "ltm auth radius-server"),
        "auth-tacacs": ("ltm_auth_tacacs", "ltm auth tacacs"),
        "auth-crldp-server": ("ltm_auth_crldp_servers", "ltm auth crldp-server"),
        "auth-ocsp-responder": ("ltm_auth_ocsp_responders", "ltm auth ocsp-responder"),
        "auth-kerberos-delegation": (
            "ltm_auth_kerberos_delegations",
            "ltm auth kerberos-delegation",
        ),
        "auth-ssl-cc-ldap": ("ltm_auth_ssl_cc_ldap", "ltm auth ssl-cc-ldap"),
        "auth-ssl-crldp": ("ltm_auth_ssl_crldp", "ltm auth ssl-crldp"),
        "auth-ssl-ocsp": ("ltm_auth_ssl_ocsp", "ltm auth ssl-ocsp"),
        # Bundle 17 — ltm CGNAT / LSN (3 kinds).
        "lsn-pool": ("ltm_lsn_pools", "ltm lsn-pool"),
        "lsn-log-profile": ("ltm_lsn_log_profiles", "ltm lsn-log-profile"),
        "alg-log-profile": ("ltm_alg_log_profiles", "ltm alg-log-profile"),
        # Bundle 18 — ltm global-settings + misc singletons.
        "default-node-monitor": ("ltm_default_node_monitor", "ltm default-node-monitor"),
        "global-settings-connection": (
            "ltm_global_settings_connection",
            "ltm global-settings connection",
        ),
        "global-settings-general": (
            "ltm_global_settings_general",
            "ltm global-settings general",
        ),
        "global-settings-rule": ("ltm_global_settings_rule", "ltm global-settings rule"),
        "global-settings-traffic-control": (
            "ltm_global_settings_traffic_control",
            "ltm global-settings traffic-control",
        ),
        "rule-profiler": ("ltm_rule_profiler", "ltm rule-profiler"),
        # Bundle 19 — ltm classification + clientssl (11 kinds).
        "classification-application": (
            "ltm_classification_application",
            "ltm classification application",
        ),
        "classification-auto-update-settings": (
            "ltm_classification_auto_update_settings",
            "ltm classification auto-update settings",
        ),
        "classification-category": (
            "ltm_classification_category",
            "ltm classification category",
        ),
        "classification-ce": ("ltm_classification_ce", "ltm classification ce"),
        "classification-signature-update-schedule": (
            "ltm_classification_signature_update_schedule",
            "ltm classification signature-update-schedule",
        ),
        "classification-url-cat-policy": (
            "ltm_classification_url_cat_policy",
            "ltm classification url-cat-policy",
        ),
        "classification-url-category": (
            "ltm_classification_url_category",
            "ltm classification url-category",
        ),
        "classification-urldb-feed-list": (
            "ltm_classification_urldb_feed_list",
            "ltm classification urldb-feed-list",
        ),
        "classification-urldb-file": (
            "ltm_classification_urldb_file",
            "ltm classification urldb-file",
        ),
        "clientssl-ocsp-stapling-responses": (
            "ltm_clientssl_ocsp_stapling_responses",
            "ltm clientssl ocsp-stapling-responses",
        ),
        "clientssl-proxy-cached-certs": (
            "ltm_clientssl_proxy_cached_certs",
            "ltm clientssl-proxy cached-certs",
        ),
        # Bundle 20 — ltm tacdb (3 kinds).
        "tacdb-customdb": ("ltm_tacdb_customdb", "ltm tacdb customdb"),
        "tacdb-customdb-file": ("ltm_tacdb_customdb_file", "ltm tacdb customdb-file"),
        "tacdb-licenseddb": ("ltm_tacdb_licenseddb", "ltm tacdb licenseddb"),
        "pool": ("pools", "ltm pool"),
        "node": ("nodes", "ltm node"),
        "rule": ("rules", "ltm rule"),
        "profile": ("profiles", "ltm profile"),
        "monitor": ("monitors", "ltm monitor"),
        "persistence": ("persistence", "ltm persistence"),
        "snatpool": ("snat_pools", "ltm snatpool"),
        "policy": ("policies", "ltm policy"),
        "data-group": ("data_groups", "ltm data-group"),
    },
    "net": {
        "route": ("net_routes", "net route"),
        "vlan": ("net_vlans", "net vlan"),
        "self": ("net_selves", "net self"),
        "route-domain": ("net_route_domains", "net route-domain"),
        "port-list": ("net_port_lists", "net port-list"),
        "interface": ("net_interfaces", "net interface"),
        "dns-resolver": ("net_dns_resolvers", "net dns-resolver"),
        "tunnels-tunnel": ("net_tunnels", "net tunnels tunnel"),
        "stp": ("net_stps", "net stp"),
        "routing-access-list": ("net_routing_access_lists", "net routing access-list"),
        "routing-bfd": ("net_routing_bfd", "net routing bfd"),
        "routing-bgp": ("net_routing_bgp", "net routing bgp"),
        "routing-community-list": ("net_routing_community_lists", "net routing community-list"),
        "routing-extcommunity-list": (
            "net_routing_extcommunity_lists",
            "net routing extcommunity-list",
        ),
        "routing-prefix-list": ("net_routing_prefix_lists", "net routing prefix-list"),
        "routing-profile-bgp": ("net_routing_profile_bgp", "net routing profile bgp"),
        "routing-route-map": ("net_routing_route_maps", "net routing route-map"),
        "routing-debug": ("net_routing_debug", "net routing debug"),
        "router-advertisement": ("net_router_advertisements", "net router-advertisement"),
        "tunnels-endpoint": ("net_tunnels_endpoints", "net tunnels endpoint"),
        "tunnels-etherip": ("net_tunnels_etherip", "net tunnels etherip"),
        "tunnels-fec": ("net_tunnels_fec", "net tunnels fec"),
        "tunnels-geneve": ("net_tunnels_geneve", "net tunnels geneve"),
        "tunnels-gre": ("net_tunnels_gre", "net tunnels gre"),
        "tunnels-ipip": ("net_tunnels_ipip", "net tunnels ipip"),
        "tunnels-ipsec": ("net_tunnels_ipsec", "net tunnels ipsec"),
        "tunnels-lw4o6": ("net_tunnels_lw4o6", "net tunnels lw4o6"),
        "tunnels-map": ("net_tunnels_map", "net tunnels map"),
        "tunnels-ppp": ("net_tunnels_ppp", "net tunnels ppp"),
        "tunnels-tcp-forward": ("net_tunnels_tcp_forward", "net tunnels tcp-forward"),
        "tunnels-v6rd": ("net_tunnels_v6rd", "net tunnels v6rd"),
        "tunnels-vxlan": ("net_tunnels_vxlan", "net tunnels vxlan"),
        "tunnels-wccp": ("net_tunnels_wccp", "net tunnels wccp"),
        "ipsec-ike-daemon": ("net_ipsec_ike_daemon", "net ipsec ike-daemon"),
        "ipsec-ike-peer": ("net_ipsec_ike_peers", "net ipsec ike-peer"),
        "ipsec-ipsec-policy": ("net_ipsec_ipsec_policies", "net ipsec ipsec-policy"),
        "ipsec-manual-security-association": (
            "net_ipsec_manual_security_associations",
            "net ipsec manual-security-association",
        ),
        "ipsec-traffic-selector": ("net_ipsec_traffic_selectors", "net ipsec traffic-selector"),
        "bwc-policy": ("net_bwc_policies", "net bwc policy"),
        "bwc-priority-group": ("net_bwc_priority_groups", "net bwc priority-group"),
        "bwc-traffic-group": ("net_bwc_traffic_groups", "net bwc traffic-group"),
        "cos-global-settings": ("net_cos_global_settings", "net cos global-settings"),
        "cos-map-8021p": ("net_cos_map_8021p", "net cos map-8021p"),
        "cos-map-dscp": ("net_cos_map_dscp", "net cos map-dscp"),
        "cos-traffic-priority": ("net_cos_traffic_priority", "net cos traffic-priority"),
        "rate-shaping-class": ("net_rate_shaping_class", "net rate-shaping class"),
        "rate-shaping-color-policer": (
            "net_rate_shaping_color_policer",
            "net rate-shaping color-policer",
        ),
        "rate-shaping-drop-policy": (
            "net_rate_shaping_drop_policy",
            "net rate-shaping drop-policy",
        ),
        "rate-shaping-queue": ("net_rate_shaping_queue", "net rate-shaping queue"),
        "rate-shaping-shaping-policy": (
            "net_rate_shaping_shaping_policy",
            "net rate-shaping shaping-policy",
        ),
        "address-list": ("net_address_lists", "net address-list"),
        "arp": ("net_arp", "net arp"),
        "dag-globals": ("net_dag_globals", "net dag-globals"),
        "fdb-tunnel": ("net_fdb_tunnel", "net fdb tunnel"),
        "fdb-vlan": ("net_fdb_vlan", "net fdb vlan"),
        "interface-cos": ("net_interface_cos", "net interface-cos"),
        "ipv6-subscriber-prefix-length": (
            "net_ipv6_subscriber_prefix_length",
            "net ipv6-subscriber-prefix-length",
        ),
        "lacp-globals": ("net_lacp_globals", "net lacp-globals"),
        "lldp-globals": ("net_lldp_globals", "net lldp-globals"),
        "multicast-globals": ("net_multicast_globals", "net multicast-globals"),
        "ndp": ("net_ndp", "net ndp"),
        "packet-filter": ("net_packet_filter", "net packet-filter"),
        "packet-filter-trusted": ("net_packet_filter_trusted", "net packet-filter-trusted"),
        "port-mirror": ("net_port_mirror", "net port-mirror"),
        "rst-cause": ("net_rst_cause", "net rst-cause"),
        "self-allow": ("net_self_allow", "net self-allow"),
        "service-policy": ("net_service_policy", "net service-policy"),
        "stp-globals": ("net_stp_globals", "net stp-globals"),
        "timer-policy": ("net_timer_policy", "net timer-policy"),
        "trunk": ("net_trunk", "net trunk"),
        "vlan-group": ("net_vlan_group", "net vlan-group"),
        "wccp": ("net_wccp", "net wccp"),
        "sfc-chain": ("net_sfc_chain", "net sfc chain"),
        "sfc-sf": ("net_sfc_sf", "net sfc sf"),
    },
    "sys": {
        "dns": ("sys_dns", "sys dns"),
        "ntp": ("sys_ntp", "sys ntp"),
        "snmp": ("sys_snmp", "sys snmp"),
        "global-settings": ("sys_global_settings", "sys global-settings"),
        "provision": ("sys_provisions", "sys provision"),
        "folder": ("sys_folders", "sys folder"),
        "file-ssl-cert": ("sys_file_ssl_certs", "sys file ssl-cert"),
        "file-ssl-key": ("sys_file_ssl_keys", "sys file ssl-key"),
        "management-route": ("sys_management_routes", "sys management-route"),
    },
    "cm": {
        "cert": ("cm_certs", "cm cert"),
        "key": ("cm_keys", "cm key"),
        "device": ("cm_devices", "cm device"),
        "device-group": ("cm_device_groups", "cm device-group"),
        "traffic-group": ("cm_traffic_groups", "cm traffic-group"),
        "trust-domain": ("cm_trust_domains", "cm trust-domain"),
    },
    "gtm": {
        "datacenter": ("gtm_datacenters", "gtm datacenter"),
        "server": ("gtm_servers", "gtm server"),
        "pool": ("gtm_pools", "gtm pool"),
        "wideip": ("gtm_wideips", "gtm wideip"),
        "prober-pool": ("gtm_prober_pools", "gtm prober-pool"),
        "region": ("gtm_regions", "gtm region"),
        "rule": ("gtm_rules", "gtm rule"),
        "monitor": ("gtm_monitors", "gtm monitor"),
        # Bundle 12 — gtm listeners / link / topology / distributed-app /
        # global-settings singletons.
        "listener": ("gtm_listeners", "gtm listener"),
        "listener-doh-proxy": (
            "gtm_listener_doh_proxies",
            "gtm listener-doh-proxy",
        ),
        "listener-doh-server": (
            "gtm_listener_doh_servers",
            "gtm listener-doh-server",
        ),
        "link": ("gtm_links", "gtm link"),
        "topology": ("gtm_topologies", "gtm topology"),
        "distributed-app": ("gtm_distributed_apps", "gtm distributed-app"),
        "global-settings-general": (
            "gtm_global_settings_general",
            "gtm global-settings general",
        ),
        "global-settings-load-balancing": (
            "gtm_global_settings_load_balancing",
            "gtm global-settings load-balancing",
        ),
        "global-settings-metrics": (
            "gtm_global_settings_metrics",
            "gtm global-settings metrics",
        ),
        "global-settings-metrics-exclusions": (
            "gtm_global_settings_metrics_exclusions",
            "gtm global-settings metrics-exclusions",
        ),
    },
    "apm": {
        "ssh-security-config": (
            "apm_ephemeral_auth_ssh_security_configs",
            "apm ephemeral-auth ssh-security-config",
        ),
        "oauth-db-instance": (
            "apm_oauth_db_instances",
            "apm oauth db-instance",
        ),
        "access-policy": (
            "apm_policy_access_policies",
            "apm policy access-policy",
        ),
        "customization-source": (
            "apm_policy_customization_sources",
            "apm policy customization-source",
        ),
        "policy-item": ("apm_policy_items", "apm policy policy-item"),
        "policy-agent": ("apm_policy_agents", "apm policy agent"),
        "default-report": ("apm_report_default_report", "apm report default-report"),
        "aaa-active-directory": ("apm_aaa_active_directory", "apm aaa active-directory"),
        "aaa-active-directory-trusted-domains": (
            "apm_aaa_active_directory_trusted_domains",
            "apm aaa active-directory-trusted-domains",
        ),
        "aaa-crldp": ("apm_aaa_crldp", "apm aaa crldp"),
        "aaa-endpoint-management-system": (
            "apm_aaa_endpoint_management_system",
            "apm aaa endpoint-management-system",
        ),
        "aaa-f5-mfa-configuration": (
            "apm_aaa_f5_mfa_configuration",
            "apm aaa f5-mfa-configuration",
        ),
        "aaa-f5-service-connector": (
            "apm_aaa_f5_service_connector",
            "apm aaa f5-service-connector",
        ),
        "aaa-http": ("apm_aaa_http", "apm aaa http"),
        "aaa-http-connector-request": (
            "apm_aaa_http_connector_request",
            "apm aaa http-connector-request",
        ),
        "aaa-http-connector-transport": (
            "apm_aaa_http_connector_transport",
            "apm aaa http-connector-transport",
        ),
        "aaa-kerberos": ("apm_aaa_kerberos", "apm aaa kerberos"),
        "aaa-kerberos-keytab-file": (
            "apm_aaa_kerberos_keytab_file",
            "apm aaa kerberos-keytab-file",
        ),
        "aaa-ldap": ("apm_aaa_ldap", "apm aaa ldap"),
        "aaa-oam": ("apm_aaa_oam", "apm aaa oam"),
        "aaa-oauth-provider": ("apm_aaa_oauth_provider", "apm aaa oauth-provider"),
        "aaa-oauth-request": ("apm_aaa_oauth_request", "apm aaa oauth-request"),
        "aaa-oauth-server": ("apm_aaa_oauth_server", "apm aaa oauth-server"),
        "aaa-ocsp": ("apm_aaa_ocsp", "apm aaa ocsp"),
        "aaa-okta-connector": ("apm_aaa_okta_connector", "apm aaa okta-connector"),
        "aaa-radius": ("apm_aaa_radius", "apm aaa radius"),
        "aaa-saml": ("apm_aaa_saml", "apm aaa saml"),
        "aaa-saml-idp-automation": ("apm_aaa_saml_idp_automation", "apm aaa saml-idp-automation"),
        "aaa-saml-idp-connector": ("apm_aaa_saml_idp_connector", "apm aaa saml-idp-connector"),
        "aaa-securid": ("apm_aaa_securid", "apm aaa securid"),
        "aaa-tacacsplus": ("apm_aaa_tacacsplus", "apm aaa tacacsplus"),
        "profile-access": ("apm_profile_access", "apm profile access"),
        "profile-connectivity": ("apm_profile_connectivity", "apm profile connectivity"),
        "profile-exchange": ("apm_profile_exchange", "apm profile exchange"),
        "profile-oauth": ("apm_profile_oauth", "apm profile oauth"),
        "profile-vdi": ("apm_profile_vdi", "apm profile vdi"),
        "sso-basic": ("apm_sso_basic", "apm sso basic"),
        "sso-form-based": ("apm_sso_form_based", "apm sso form-based"),
        "sso-form-basedv2": ("apm_sso_form_basedv2", "apm sso form-basedv2"),
        "sso-kerberos": ("apm_sso_kerberos", "apm sso kerberos"),
        "sso-ntlmv1": ("apm_sso_ntlmv1", "apm sso ntlmv1"),
        "sso-ntlmv2": ("apm_sso_ntlmv2", "apm sso ntlmv2"),
        "sso-oauth-bearer": ("apm_sso_oauth_bearer", "apm sso oauth-bearer"),
        "sso-saml": ("apm_sso_saml", "apm sso saml"),
        "sso-saml-resource": ("apm_sso_saml_resource", "apm sso saml-resource"),
        "sso-saml-sp-automation": ("apm_sso_saml_sp_automation", "apm sso saml-sp-automation"),
        "sso-saml-sp-connector": ("apm_sso_saml_sp_connector", "apm sso saml-sp-connector"),
        "resource-address-space": ("apm_resource_address_space", "apm resource address-space"),
        "resource-app-tunnel": ("apm_resource_app_tunnel", "apm resource app-tunnel"),
        "resource-client-rate-class": (
            "apm_resource_client_rate_class",
            "apm resource client-rate-class",
        ),
        "resource-client-traffic-classifier": (
            "apm_resource_client_traffic_classifier",
            "apm resource client-traffic-classifier",
        ),
        "resource-ipv6-leasepool": ("apm_resource_ipv6_leasepool", "apm resource ipv6-leasepool"),
        "resource-leasepool": ("apm_resource_leasepool", "apm resource leasepool"),
        "resource-network-access": ("apm_resource_network_access", "apm resource network-access"),
        "resource-portal-access": ("apm_resource_portal_access", "apm resource portal-access"),
        "resource-remote-desktop-citrix": (
            "apm_resource_remote_desktop_citrix",
            "apm resource remote-desktop citrix",
        ),
        "resource-remote-desktop-citrix-client-bundle": (
            "apm_resource_remote_desktop_citrix_client_bundle",
            "apm resource remote-desktop citrix-client-bundle",
        ),
        "resource-remote-desktop-citrix-client-package-file": (
            "apm_resource_remote_desktop_citrix_client_package_file",
            "apm resource remote-desktop citrix-client-package-file",
        ),
        "resource-remote-desktop-quest": (
            "apm_resource_remote_desktop_quest",
            "apm resource remote-desktop quest",
        ),
        "resource-remote-desktop-rdp": (
            "apm_resource_remote_desktop_rdp",
            "apm resource remote-desktop rdp",
        ),
        "resource-remote-desktop-vmware-view": (
            "apm_resource_remote_desktop_vmware_view",
            "apm resource remote-desktop vmware-view",
        ),
        "resource-sandbox": ("apm_resource_sandbox", "apm resource sandbox"),
        "resource-webtop": ("apm_resource_webtop", "apm resource webtop"),
        "resource-webtop-link": ("apm_resource_webtop_link", "apm resource webtop-link"),
        "oauth-jwk-config": ("apm_oauth_jwk_config", "apm oauth jwk-config"),
        "oauth-jwt-config": ("apm_oauth_jwt_config", "apm oauth jwt-config"),
        "oauth-jwt-provider-list": ("apm_oauth_jwt_provider_list", "apm oauth jwt-provider-list"),
        "oauth-oauth-claim": ("apm_oauth_oauth_claim", "apm oauth oauth-claim"),
        "oauth-oauth-client-app": ("apm_oauth_oauth_client_app", "apm oauth oauth-client-app"),
        "oauth-oauth-resource-server": (
            "apm_oauth_oauth_resource_server",
            "apm oauth oauth-resource-server",
        ),
        "oauth-oauth-scope": ("apm_oauth_oauth_scope", "apm oauth oauth-scope"),
        "saml-artifact-resolution-service": (
            "apm_saml_artifact_resolution_service",
            "apm saml artifact-resolution-service",
        ),
        "saml-attribute-consuming-service": (
            "apm_saml_attribute_consuming_service",
            "apm saml attribute-consuming-service",
        ),
        "saml-auth-context-class-list": (
            "apm_saml_auth_context_class_list",
            "apm saml auth-context-class-list",
        ),
        "ntlm-machine-account": ("apm_ntlm_machine_account", "apm ntlm machine-account"),
        "ntlm-ntlm-auth": ("apm_ntlm_ntlm_auth", "apm ntlm ntlm-auth"),
        "acl": ("apm_acl", "apm acl"),
        "log-setting": ("apm_log_setting", "apm log-setting"),
        "url-filter": ("apm_url_filter", "apm url-filter"),
        "swg-scheme": ("apm_swg_scheme", "apm swg-scheme"),
        "client-image": ("apm_client_image", "apm client image"),
        "configuration-captcha": ("apm_configuration_captcha", "apm configuration captcha"),
        "epsec-epsec-package": ("apm_epsec_epsec_package", "apm epsec epsec-package"),
        "apm-avr-config": ("apm_apm_avr_config", "apm apm-avr-config"),
        "report-custom-report-field": (
            "apm_report_custom_report_field",
            "apm report custom-report-field",
        ),
        "policy-customization-group": (
            "apm_policy_customization_group",
            "apm policy customization-group",
        ),
        "policy-customization-languages": (
            "apm_policy_customization_languages",
            "apm policy customization-languages",
        ),
        "policy-image-file": ("apm_policy_image_file", "apm policy image-file"),
        "policy-windows-group-policy-file": (
            "apm_policy_windows_group_policy_file",
            "apm policy windows-group-policy-file",
        ),
    },
    "security": {
        "firewall-port-list": (
            "security_firewall_port_lists",
            "security firewall port-list",
        ),
        "firewall-rule-list": (
            "security_firewall_rule_lists",
            "security firewall rule-list",
        ),
        "firewall-config-entity-id": (
            "security_firewall_config_entity_ids",
            "security firewall config-entity-id",
        ),
        "firewall-policy": ("security_firewall_policies", "security firewall policy"),
        "firewall-address-list": (
            "security_firewall_address_lists",
            "security firewall address-list",
        ),
        "firewall-global-rules": (
            "security_firewall_global_rules",
            "security firewall global-rules",
        ),
        "firewall-management-ip-rules": (
            "security_firewall_management_ip_rules",
            "security firewall management-ip-rules",
        ),
        "firewall-schedule": ("security_firewall_schedules", "security firewall schedule"),
        "firewall-user-list": (
            "security_firewall_user_lists",
            "security firewall user-list",
        ),
        "firewall-user-domain": (
            "security_firewall_user_domains",
            "security firewall user-domain",
        ),
        "firewall-global-fqdn-policy": (
            "security_firewall_global_fqdn_policy",
            "security firewall global-fqdn-policy",
        ),
        "firewall-port-misuse-policy": (
            "security_firewall_port_misuse_policies",
            "security firewall port-misuse-policy",
        ),
        "firewall-on-demand-compilation": (
            "security_firewall_on_demand_compilation",
            "security firewall on-demand-compilation",
        ),
        "firewall-on-demand-rule-deploy": (
            "security_firewall_on_demand_rule_deploy",
            "security firewall on-demand-rule-deploy",
        ),
        "firewall-uuid-default-autogenerate": (
            "security_firewall_uuid_default_autogenerate",
            "security firewall uuid-default-autogenerate",
        ),
        "firewall-config-change-log": (
            "security_firewall_config_change_log",
            "security firewall config-change-log",
        ),
        "nat-policy": ("security_nat_policies", "security nat policy"),
        "nat-source-translation": (
            "security_nat_source_translations",
            "security nat source-translation",
        ),
        "nat-destination-translation": (
            "security_nat_destination_translations",
            "security nat destination-translation",
        ),
        "log-profile": ("security_log_profiles", "security log profile"),
        "dos-profile": ("security_dos_profiles", "security dos profile"),
        "ip-intelligence-feed-list": (
            "security_ip_intelligence_feed_lists",
            "security ip-intelligence feed-list",
        ),
        "ip-intelligence-global-policy": (
            "security_ip_intelligence_global_policy",
            "security ip-intelligence global-policy",
        ),
        "zone": ("security_zones", "security zone"),
        "protected-zone": ("security_protected_zones", "security protected zone"),
        "packet-filter-policy": (
            "security_packet_filter_policies",
            "security packet-filter policy",
        ),
        "packet-filter-default-rules": (
            "security_packet_filter_default_rules",
            "security packet-filter default-rules",
        ),
        "ssh-profile": ("security_ssh_profiles", "security ssh profile"),
        "http-profile": ("security_http_profiles", "security http profile"),
        "bot-defense-profile": (
            "security_bot_defense_profiles",
            "security bot-defense profile",
        ),
        # Bundle 10b — minimal-shape security.* kinds (37).
        "analytics-settings": ("security_analytics_settings", "security analytics settings"),
        "anti-fraud-profile": (
            "security_anti_fraud_profiles",
            "security anti-fraud profile",
        ),
        "anti-fraud-signatures-update": (
            "security_anti_fraud_signatures_update",
            "security anti-fraud signatures-update",
        ),
        "blacklist-publisher-category": (
            "security_blacklist_publisher_categories",
            "security blacklist-publisher category",
        ),
        "blacklist-publisher-profile": (
            "security_blacklist_publisher_profiles",
            "security blacklist-publisher profile",
        ),
        "bot-defense-signature": (
            "security_bot_defense_signatures",
            "security bot-defense signature",
        ),
        "bot-defense-signature-category": (
            "security_bot_defense_signature_categories",
            "security bot-defense signature-category",
        ),
        "cloud-services-connector": (
            "security_cloud_services_connectors",
            "security cloud-services connector",
        ),
        "datasync-background-tasks": (
            "security_datasync_background_tasks",
            "security datasync background-tasks",
        ),
        "datasync-global-profile": (
            "security_datasync_global_profiles",
            "security datasync global-profile",
        ),
        "datasync-local-profile": (
            "security_datasync_local_profiles",
            "security datasync local-profile",
        ),
        "debug-drop-redirect-stats": (
            "security_debug_drop_redirect_stats",
            "security debug drop-redirect-stats",
        ),
        "debug-matcher": ("security_debug_matcher", "security debug matcher"),
        "debug-register": ("security_debug_register", "security debug register"),
        "device-device-context": (
            "security_device_device_context",
            "security device device-context",
        ),
        "dos-autodos-file-object": (
            "security_dos_autodos_file_objects",
            "security dos autodos-file-object",
        ),
        "dos-behavioral-signature": (
            "security_dos_behavioral_signatures",
            "security dos behavioral-signature",
        ),
        "dos-bot-signature": (
            "security_dos_bot_signatures",
            "security dos bot-signature",
        ),
        "dos-bot-signature-category": (
            "security_dos_bot_signature_categories",
            "security dos bot-signature-category",
        ),
        "dos-device-config": (
            "security_dos_device_config",
            "security dos device-config",
        ),
        "dos-dns-nxdomain-stat": (
            "security_dos_dns_nxdomain_stat",
            "security dos dns-nxdomain-stat",
        ),
        "dos-dos-signature": (
            "security_dos_dos_signatures",
            "security dos dos-signature",
        ),
        "dos-dynamic-signatures": (
            "security_dos_dynamic_signatures",
            "security dos dynamic-signatures",
        ),
        "dos-ip-uncommon-protolist": (
            "security_dos_ip_uncommon_protolists",
            "security dos ip-uncommon-protolist",
        ),
        "dos-l4bdos-file-object": (
            "security_dos_l4bdos_file_objects",
            "security dos l4bdos-file-object",
        ),
        "dos-network-whitelist": (
            "security_dos_network_whitelists",
            "security dos network-whitelist",
        ),
        "dos-stress-stats": ("security_dos_stress_stats", "security dos stress-stats"),
        "dos-udp-portlist": ("security_dos_udp_portlists", "security dos udp-portlist"),
        "dos-virtual": ("security_dos_virtuals", "security dos virtual"),
        "flowspec-route-injector-profile": (
            "security_flowspec_route_injector_profiles",
            "security flowspec-route-injector profile",
        ),
        "ip-intelligence-blacklist-category": (
            "security_ip_intelligence_blacklist_categories",
            "security ip-intelligence blacklist-category",
        ),
        "protocol-inspection-common-config": (
            "security_protocol_inspection_common_config",
            "security protocol-inspection common-config",
        ),
        "protocol-inspection-learning-stats": (
            "security_protocol_inspection_learning_stats",
            "security protocol-inspection learning-stats",
        ),
        "protocol-inspection-profile": (
            "security_protocol_inspection_profiles",
            "security protocol-inspection profile",
        ),
        "protocol-inspection-signature": (
            "security_protocol_inspection_signatures",
            "security protocol-inspection signature",
        ),
        "scrubber-profile": (
            "security_scrubber_profiles",
            "security scrubber profile",
        ),
        "ssh-ciphers": ("security_ssh_ciphers", "security ssh ciphers"),
        "ip-intelligence-policy": (
            "security_ip_intelligence_policies",
            "security ip-intelligence policy",
        ),
        "protocol-inspection-compliance-map": (
            "security_pi_compliance_maps",
            "security protocol-inspection compliance-map",
        ),
        "protocol-inspection-compliance-objects": (
            "security_pi_compliance_objects",
            "security protocol-inspection compliance-objects",
        ),
        "device-id-attribute": (
            "security_device_id_attributes",
            "security device-id attribute",
        ),
    },
    "pem": {
        "policy": ("pem_policies", "pem policy"),
        "irule": ("pem_rules", "pem rule"),
        "listener": ("pem_listeners", "pem listener"),
        "forwarding-endpoint": (
            "pem_forwarding_endpoints",
            "pem forwarding-endpoint",
        ),
        "interception-endpoint": (
            "pem_interception_endpoints",
            "pem interception-endpoint",
        ),
        "service-chain-endpoint": (
            "pem_service_chain_endpoints",
            "pem service-chain-endpoint",
        ),
        "profile": ("pem_profiles", "pem profile"),
        "rating-group": ("pem_rating_groups", "pem quota-mgmt rating-group"),
        "global-settings-analytics": ("pem_gs_analytics", "pem global-settings analytics"),
        "global-settings-gx": ("pem_gs_gx", "pem global-settings gx"),
        "global-settings-hsl-flow": ("pem_gs_hsl_flow", "pem global-settings hsl-flow"),
        "global-settings-hsl-report": ("pem_gs_hsl_report", "pem global-settings hsl-report"),
        "global-settings-insert-content": (
            "pem_gs_insert_content",
            "pem global-settings insert-content",
        ),
        "global-settings-policy": ("pem_gs_policy", "pem global-settings policy"),
        "global-settings-quota-mgmt": ("pem_gs_quota_mgmt", "pem global-settings quota-mgmt"),
        "global-settings-session-mgmt-attributes": (
            "pem_gs_session_mgmt_attributes",
            "pem global-settings session-mgmt-attributes",
        ),
        "global-settings-subscriber-activity-log": (
            "pem_gs_subscriber_activity_log",
            "pem global-settings subscriber-activity-log",
        ),
        "protocol-diameter-avp": ("pem_protocol_diameter_avp", "pem protocol diameter-avp"),
        "protocol-radius-avp": ("pem_protocol_radius_avp", "pem protocol radius-avp"),
        "protocol-profile-gx": ("pem_protocol_profile_gx", "pem protocol profile gx"),
        "protocol-profile-radius": ("pem_protocol_profile_radius", "pem protocol profile radius"),
        "reporting-format-script": ("pem_reporting_format_script", "pem reporting format-script"),
        "subscriber": ("pem_subscriber", "pem subscriber"),
        "subscriber-attribute": ("pem_subscriber_attribute", "pem subscriber-attribute"),
    },
    "auth": {
        "partition": ("auth_partitions", "auth partition"),
        "user": ("auth_users", "auth user"),
        "password": ("auth_password", "auth password"),
        "password-policy": ("auth_password_policy", "auth password-policy"),
        "source": ("auth_source", "auth source"),
        "remote-role": ("auth_remote_role", "auth remote-role"),
        "remote-user": ("auth_remote_user", "auth remote-user"),
        "login-failures": ("auth_login_failures", "auth login-failures"),
        "ldap": ("auth_ldaps", "auth ldap"),
        "radius": ("auth_radii", "auth radius"),
        "radius-server": ("auth_radius_servers", "auth radius-server"),
        "tacacs": ("auth_tacacs", "auth tacacs"),
        "cert-ldap": ("auth_cert_ldaps", "auth cert-ldap"),
        "apm-auth": ("auth_apm_auths", "auth apm-auth"),
    },
}

_OBJECT_KIND_ALIASES = frozenset(kind for mod in _MODULE_KINDS.values() for _, kind in mod.values())


# Public aliases so other consumers (builtins, runner, tests) can
# enumerate kinds without reaching into single-underscore names.  The
# ``LTM_KINDS`` alias is kept for backwards compatibility — callers
# that only care about ``ltm`` use it; everyone else uses
# ``MODULE_KINDS``.
LTM_KINDS = _MODULE_KINDS["ltm"]
MODULE_KINDS = _MODULE_KINDS


# ---------------------------------------------------------------------------
# Building containers and object refs
# ---------------------------------------------------------------------------


def root_container(root: Root) -> Container:
    """Return the synthetic top-level container.

    Holds one child per known module (``ltm``, ``net``, …).  Each
    module container is lazy — its kind containers and per-object
    refs only materialise when navigated into.
    """
    return Container(kind="<root>", root=root, _entry_source="")


def _build_entries(container: Container) -> dict[str, Any]:
    root = container.root
    if container.kind == "<root>":
        return {
            module: Container(kind=module, root=root, _entry_source=module)
            for module in _MODULE_KINDS
        }

    # Module-level container (``.ltm``, ``.net``, …).
    if container.kind in _MODULE_KINDS:
        module = container.kind
        return {
            label: Container(
                kind=tmsh_kind,
                root=root,
                _entry_source=f"{module}.{label}",
            )
            for label, (_, tmsh_kind) in _MODULE_KINDS[module].items()
        }

    # Leaf kind container: project each object.
    label = container.kind
    if label in _OBJECT_KIND_ALIASES:
        cls_attr = _kind_to_attr(label)
        objects = getattr(root.config, cls_attr)
        entries: dict[str, Any] = {}
        for full_path, obj in objects.items():
            entries[full_path] = _build_object_ref(label, full_path, obj, root)
        return entries

    return {}


def _kind_to_attr(kind: str) -> str:
    for module in _MODULE_KINDS.values():
        for label, (attr, k) in module.items():
            if k == kind:
                return attr
    raise EvalError(f"unknown object kind: {kind!r}")


def _build_object_ref(
    kind: str,
    full_path: str,
    obj: object,
    root: Root,
) -> ObjectRef:
    # Key the cache by (kind, full_path).  BIG-IP allows different
    # object kinds to live under the same path string (a pool, a node,
    # and an iRule can all share ``/Common/shared``), so a single
    # ``full_path``-only key would let one kind's :class:`ObjectRef`
    # leak into a query that asks for another kind.
    cache_key = (kind, full_path)
    if cache_key in root._object_cache:
        return root._object_cache[cache_key]

    _, field_map = _KIND_FIELD_MAPS[kind]
    fields: dict[str, Any] = {}
    for tmsh_name, spec in field_map.items():
        value = _project_field(kind, obj, spec, root)
        fields[tmsh_name] = value
    field_slots = _collect_field_slots(kind, obj, field_map, root)

    stanza_slot = None
    rng = getattr(obj, "range", None)
    if rng is not None:
        # The dataclass range covers the brace block ``{ ... }``; extend
        # it backwards over the header so SCF stanza output includes
        # ``ltm virtual /Common/web_vs`` along with the body.
        header_start = _scan_back_to_line_start(root.source, rng.start.offset)
        stanza_slot = FieldSlot(
            source_uri=root.uri,
            start=header_start,
            end=rng.end.offset,
            raw_text=root.source[header_start : rng.end.offset],
        )

    ref = ObjectRef(
        kind=kind,
        full_path=full_path,
        fields=fields,
        field_slots=field_slots,
        stanza_slot=stanza_slot,
        config_uri=root.uri,
    )
    root._object_cache[cache_key] = ref
    return ref


def _project_field(
    kind: str,
    obj: object,
    spec: FieldSpec,
    root: Root,
) -> Any:
    # Synthesised fields first.
    if spec.attr == "__refs__":
        if kind == "ltm rule" and isinstance(obj, BigipRule):
            return _rule_refs_value(obj, root)
        return None

    if not hasattr(obj, spec.attr):
        return None
    raw = getattr(obj, spec.attr)

    if spec.ref_kind and spec.list_ref:
        # Tuple/list of full-path strings.
        return [PathRef(full_path=p, root=root, expected_kind=spec.ref_kind) for p in raw or ()]
    if spec.ref_kind:
        return PathRef(full_path=raw or "", root=root, expected_kind=spec.ref_kind)
    if kind == "ltm pool" and spec.attr == "members":
        return [_member_object_ref(m, root) for m in raw or ()]
    if kind == "ltm policy" and spec.attr == "rules":
        return [_policy_rule_object_ref(r, root) for r in raw or ()]
    if isinstance(raw, tuple):
        return list(raw)
    return raw


def _member_object_ref(member: BigipPoolMember, root: Root) -> ObjectRef:
    return ObjectRef(
        kind="ltm pool-member",
        full_path=member.name,
        fields={
            "name": member.name,
            "address": member.address,
            "port": member.port,
            "monitor": PathRef(full_path=member.monitor, root=root, expected_kind="ltm monitor")
            if member.monitor
            else "",
            "description": member.description,
            "state": member.state,
            "ratio": member.ratio,
            "priority-group": member.priority_group,
            "connection-limit": member.connection_limit,
            "rate-limit": member.rate_limit,
        },
    )


def _policy_condition_object_ref(cond: BigipPolicyCondition) -> ObjectRef:
    return ObjectRef(
        kind="ltm policy-condition",
        full_path=str(cond.index),
        fields={
            "index": cond.index,
            "operand": cond.operand,
            "selector": cond.selector,
            "operator": cond.operator,
            "values": list(cond.values),
            "name": cond.name,
            "negate": cond.negate,
            "case-insensitive": cond.case_insensitive,
            "event": cond.event,
        },
    )


def _policy_action_object_ref(action: BigipPolicyAction, root: Root) -> ObjectRef:
    return ObjectRef(
        kind="ltm policy-action",
        full_path=str(action.index),
        fields={
            "index": action.index,
            "target": action.target,
            "verb": action.verb,
            # ``pool`` is a PathRef into ``ltm pool`` — chaining
            # ``.pool.members`` from a policy action walks into the
            # target pool.
            "pool": PathRef(full_path=action.pool, root=root, expected_kind="ltm pool"),
            "location": action.location,
            "name": action.name,
            "value": action.value,
            "path": action.path,
            "query": action.query,
            "host": action.host,
            "event": action.event,
        },
    )


def _policy_rule_object_ref(rule: BigipPolicyRule, root: Root) -> ObjectRef:
    return ObjectRef(
        kind="ltm policy-rule",
        full_path=rule.name,
        fields={
            "name": rule.name,
            "ordinal": rule.ordinal,
            "conditions": [_policy_condition_object_ref(c) for c in rule.conditions],
            "actions": [_policy_action_object_ref(a, root) for a in rule.actions],
        },
    )


def _rule_refs_value(obj: BigipRule, root: Root) -> ObjectRef:
    """Build the synthesised ``.ltm.rule[].refs`` object.

    Each ref slot is a list of :class:`PathRef`s drawn from the same
    reference graph :mod:`core.bigip.grep` walks, so the query DSL and
    the grep verb always agree on what an iRule "uses".
    """
    pools, persists, data_groups = _extract_rule_refs(obj, root)
    return ObjectRef(
        kind="ltm rule-refs",
        full_path=obj.full_path,
        fields={
            "pools": [PathRef(p, root=root, expected_kind="ltm pool") for p in pools],
            "persists": [PathRef(p, root=root, expected_kind="ltm persistence") for p in persists],
            "data-groups": [
                PathRef(p, root=root, expected_kind="ltm data-group") for p in data_groups
            ],
        },
    )


def _extract_rule_refs(obj: BigipRule, root: Root) -> tuple[list[str], list[str], list[str]]:
    from ..grep import compute_grep

    report = compute_grep(
        sources={root.uri: root.source},
        configs={root.uri: root.config},
        pattern=obj.full_path,
        use_regex=False,
        use_cidr=False,
        direction="forward",
        max_depth=1,
        max_nodes=1024,
        include_body=False,
        recurse=True,
    )
    pools: set[str] = set()
    persists: set[str] = set()
    data_groups: set[str] = set()
    for node in report.related:
        if node.full_path == obj.full_path:
            continue
        if node.module == "ltm" and node.object_type == "pool":
            pools.add(node.full_path)
        elif node.module == "ltm" and node.object_type.startswith("persistence"):
            persists.add(node.full_path)
        elif node.object_type.startswith("data-group"):
            data_groups.add(node.full_path)
    return sorted(pools), sorted(persists), sorted(data_groups)


# ---------------------------------------------------------------------------
# Field-slot byte-range discovery
# ---------------------------------------------------------------------------


_KEY_LINE_RE = re.compile(
    r"^(?P<indent>[ \t]*)(?P<key>[A-Za-z0-9_./\-]+)[ \t]+(?P<value>[^\n{]+?)[ \t]*$",
    re.MULTILINE,
)


def _scan_back_to_line_start(source: str, offset: int) -> int:
    """Return the offset of the start of the line containing *offset*.

    Used to extend a stanza range backwards over its header so SCF
    output includes ``ltm virtual /Common/foo`` along with the body.
    """
    i = offset
    while i > 0 and source[i - 1] != "\n":
        i -= 1
    return i


def _collect_field_slots(
    kind: str,
    obj: object,
    field_map: dict[str, FieldSpec],
    root: Root,
) -> dict[str, FieldSlot]:
    """Locate each top-level property's value span inside its stanza.

    Returns a mapping of TMSH-spelt field name → :class:`FieldSlot` for
    every field that appears as a single-line property in the source.
    Fields with no slot (compound list/sub-block values, identity
    fields whose location is the header) are simply absent — the edit
    planner falls back to its own strategy for those.
    """
    rng = getattr(obj, "range", None)
    if rng is None:
        return {}
    body_start = rng.start.offset
    body_end = rng.end.offset
    body_text = root.source[body_start:body_end]
    slots: dict[str, FieldSlot] = {}
    for match in _KEY_LINE_RE.finditer(body_text):
        key = match.group("key")
        # Map TMSH key back to our field-map name.  Keys with spaces or
        # nested braces are not handled here (those are sub-blocks).
        if key not in field_map:
            continue
        value_text = match.group("value")
        value_start = match.start("value") + body_start
        value_end = value_start + len(value_text)
        slots[key] = FieldSlot(
            source_uri=root.uri,
            start=value_start,
            end=value_end,
            raw_text=value_text,
        )
    return slots
