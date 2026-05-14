"""The top-level :class:`BigipConfig` container.

Holds every parsed object dictionary across every F5 module, plus
convenience resolvers (``resolve_pool`` / ``resolve_persistence``
/ ``profiles_for_virtual`` / …) used by the projection layer
and the cross-config validators.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field

from ._apm import (
    BigipApmEphemeralAuthSshSecurityConfig,
    BigipApmOauthDbInstance,
    BigipApmPolicyAccessPolicy,
    BigipApmPolicyAgent,
    BigipApmPolicyCustomizationSource,
    BigipApmPolicyItem,
    BigipApmReportDefaultReport,
)
from ._auth import (
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
)
from ._cm import (
    BigipCmCert,
    BigipCmDevice,
    BigipCmDeviceGroup,
    BigipCmKey,
    BigipCmTrafficGroup,
    BigipCmTrustDomain,
)
from ._enums import ProfileType
from ._gtm import (
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
)
from ._ltm import (
    BigipDataGroup,
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
    BigipLtmNat,
    BigipLtmPolicyStrategy,
    BigipLtmSnat,
    BigipLtmSnatTranslation,
    BigipLtmTrafficClass,
    BigipLtmTrafficMatchingCriteria,
    BigipMonitor,
    BigipNode,
    BigipPersistence,
    BigipPolicy,
    BigipPool,
    BigipProfile,
    BigipRule,
    BigipSnatPool,
    BigipVirtualAddress,
    BigipVirtualServer,
)
from ._minimal import (
    BigipAnalyticsMinimalObject,
    BigipApiProtectionMinimalObject,
    BigipApmMinimalObject,
    BigipAsmMinimalObject,
    BigipCliMinimalObject,
    BigipCmMinimalObject,
    BigipGenericObject,
    BigipIlxMinimalObject,
    BigipLtmMinimalObject,
    BigipNetMinimalObject,
    BigipPemMinimalObject,
    BigipSecurityMinimalObject,
    BigipSysMinimalObject,
    BigipVcmpMinimalObject,
    BigipWomMinimalObject,
)
from ._net import (
    BigipNetDnsResolver,
    BigipNetInterface,
    BigipNetPortList,
    BigipNetRoute,
    BigipNetRouteDomain,
    BigipNetSelf,
    BigipNetStp,
    BigipNetTunnel,
    BigipNetVlan,
)
from ._pem import (
    BigipPemForwardingEndpoint,
    BigipPemInterceptionEndpoint,
    BigipPemListener,
    BigipPemPolicy,
    BigipPemProfile,
    BigipPemRatingGroup,
    BigipPemRule,
    BigipPemServiceChainEndpoint,
)
from ._security import (
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
)
from ._sys import (
    BigipSysDns,
    BigipSysFileSslCert,
    BigipSysFileSslKey,
    BigipSysFolder,
    BigipSysGlobalSettings,
    BigipSysManagementRoute,
    BigipSysNtp,
    BigipSysProvision,
    BigipSysSnmp,
)


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

    def merge(self, other: "BigipConfig") -> None:
        """In-place merge of every dict-valued field on *other* into ``self``.

        Iterates the dataclass fields by introspection rather than a
        hand-rolled ``self.x.update(other.x)`` list so future kinds
        added to :class:`BigipConfig` are merged automatically — the
        legacy hand-rolled list in ``lsp.workspace.scanner`` missed
        every kind beyond the v1 ten and silently dropped data when
        callers used ``merged_bigip_config``.

        Standard ``dict.update`` semantics: keys from *other* win on
        conflict.  Callers wanting partition / source-origin
        precedence should pre-filter ``other`` themselves.
        """
        from dataclasses import fields

        for fld in fields(self):
            mine = getattr(self, fld.name)
            theirs = getattr(other, fld.name)
            if isinstance(mine, dict) and isinstance(theirs, dict):
                mine.update(theirs)

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
        for pref in vs.profiles.paths:
            resolved = self.resolve_profile(pref)
            if resolved and resolved in self.profiles:
                result.append(self.profiles[resolved])
        return result

    def profile_types_for_virtual(self, vs_name: str) -> frozenset[ProfileType]:
        """Return the set of profile types attached to a virtual server."""
        return frozenset(p.profile_type for p in self.profiles_for_virtual(vs_name))
