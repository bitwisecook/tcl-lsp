// @generated — do not edit.
//! Generated `ModelObject` enum over every placed BIG-IP kind.

// Generated file: the wide enum variant, glob re-exports, and long match arms
// are inherent to codegen and not hand-fixable without editing the generator.
#![allow(clippy::large_enum_variant)]
#![allow(clippy::wildcard_imports, clippy::too_many_lines)]

use super::*;
use crate::model::{BigipGenericObject, BigipMinimalObject};

/// A parsed BIG-IP object of any kind, as produced by the driver.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelObject {
    /// `BigipDataGroup`.
    DataGroup(BigipDataGroup),
    /// `BigipPool`.
    Pool(BigipPool),
    /// `BigipVirtualServer`.
    VirtualServer(BigipVirtualServer),
    /// `BigipVirtualAddress`.
    VirtualAddress(BigipVirtualAddress),
    /// `BigipLtmCipherGroup`.
    LtmCipherGroup(BigipLtmCipherGroup),
    /// `BigipLtmCipherRule`.
    LtmCipherRule(BigipLtmCipherRule),
    /// `BigipLtmNat`.
    LtmNat(BigipLtmNat),
    /// `BigipLtmSnat`.
    LtmSnat(BigipLtmSnat),
    /// `BigipLtmSnatTranslation`.
    LtmSnatTranslation(BigipLtmSnatTranslation),
    /// `BigipLtmPolicyStrategy`.
    LtmPolicyStrategy(BigipLtmPolicyStrategy),
    /// `BigipLtmRateClass`.
    LtmRateClass(BigipLtmRateClass),
    /// `BigipLtmTrafficClass`.
    LtmTrafficClass(BigipLtmTrafficClass),
    /// `BigipLtmTrafficMatchingCriteria`.
    LtmTrafficMatchingCriteria(BigipLtmTrafficMatchingCriteria),
    /// `BigipLtmIfile`.
    LtmIfile(BigipLtmIfile),
    /// `BigipLtmEvictionPolicy`.
    LtmEvictionPolicy(BigipLtmEvictionPolicy),
    /// `BigipLtmDnsNameserver`.
    LtmDnsNameserver(BigipLtmDnsNameserver),
    /// `BigipLtmDnsTsigKey`.
    LtmDnsTsigKey(BigipLtmDnsTsigKey),
    /// `BigipLtmDnsZone`.
    LtmDnsZone(BigipLtmDnsZone),
    /// `BigipLtmDnsDnssecKey`.
    LtmDnsDnssecKey(BigipLtmDnsDnssecKey),
    /// `BigipLtmDnsDnssecZone`.
    LtmDnsDnssecZone(BigipLtmDnsDnssecZone),
    /// `BigipLtmDnsCacheResolver`.
    LtmDnsCacheResolver(BigipLtmDnsCacheResolver),
    /// `BigipLtmDnsCacheTransparent`.
    LtmDnsCacheTransparent(BigipLtmDnsCacheTransparent),
    /// `BigipLtmDnsCacheValidatingResolver`.
    LtmDnsCacheValidatingResolver(BigipLtmDnsCacheValidatingResolver),
    /// `BigipLtmDnsCacheGlobalSettings`.
    LtmDnsCacheGlobalSettings(BigipLtmDnsCacheGlobalSettings),
    /// `BigipLtmDnsCacheRecord`.
    LtmDnsCacheRecord(BigipLtmDnsCacheRecord),
    /// `BigipLtmDnsHpkeKey`.
    LtmDnsHpkeKey(BigipLtmDnsHpkeKey),
    /// `BigipLtmDnsHpkeProfile`.
    LtmDnsHpkeProfile(BigipLtmDnsHpkeProfile),
    /// `BigipLtmDnsAnalyticsGlobalSettings`.
    LtmDnsAnalyticsGlobalSettings(BigipLtmDnsAnalyticsGlobalSettings),
    /// `BigipLtmMessageRoutingObject`.
    LtmMessageRoutingObject(BigipLtmMessageRoutingObject),
    /// `BigipLtmAuthObject`.
    LtmAuthObject(BigipLtmAuthObject),
    /// `BigipMinimalObject`.
    Minimal(BigipMinimalObject),
    /// `BigipNode`.
    Node(BigipNode),
    /// `BigipProfile`.
    Profile(BigipProfile),
    /// `BigipMonitor`.
    Monitor(BigipMonitor),
    /// `BigipSnatPool`.
    SnatPool(BigipSnatPool),
    /// `BigipPersistence`.
    Persistence(BigipPersistence),
    /// `BigipRule`.
    Rule(BigipRule),
    /// `BigipPolicy`.
    Policy(BigipPolicy),
    /// `BigipNetRoute`.
    NetRoute(BigipNetRoute),
    /// `BigipNetVlan`.
    NetVlan(BigipNetVlan),
    /// `BigipNetSelf`.
    NetSelf(BigipNetSelf),
    /// `BigipNetRouteDomain`.
    NetRouteDomain(BigipNetRouteDomain),
    /// `BigipNetPortList`.
    NetPortList(BigipNetPortList),
    /// `BigipNetInterface`.
    NetInterface(BigipNetInterface),
    /// `BigipNetDnsResolver`.
    NetDnsResolver(BigipNetDnsResolver),
    /// `BigipNetTunnel`.
    NetTunnel(BigipNetTunnel),
    /// `BigipNetStp`.
    NetStp(BigipNetStp),
    /// `BigipSysDns`.
    SysDns(BigipSysDns),
    /// `BigipSysNtp`.
    SysNtp(BigipSysNtp),
    /// `BigipSysSnmp`.
    SysSnmp(BigipSysSnmp),
    /// `BigipSysGlobalSettings`.
    SysGlobalSettings(BigipSysGlobalSettings),
    /// `BigipSysProvision`.
    SysProvision(BigipSysProvision),
    /// `BigipSysFolder`.
    SysFolder(BigipSysFolder),
    /// `BigipSysFileSslCert`.
    SysFileSslCert(BigipSysFileSslCert),
    /// `BigipSysFileSslKey`.
    SysFileSslKey(BigipSysFileSslKey),
    /// `BigipSysManagementRoute`.
    SysManagementRoute(BigipSysManagementRoute),
    /// `BigipSecurityFirewallPortList`.
    SecurityFirewallPortList(BigipSecurityFirewallPortList),
    /// `BigipSecurityFirewallRuleList`.
    SecurityFirewallRuleList(BigipSecurityFirewallRuleList),
    /// `BigipSecurityFirewallConfigEntityId`.
    SecurityFirewallConfigEntityId(BigipSecurityFirewallConfigEntityId),
    /// `BigipSecurityFirewallPolicy`.
    SecurityFirewallPolicy(BigipSecurityFirewallPolicy),
    /// `BigipSecurityFirewallAddressList`.
    SecurityFirewallAddressList(BigipSecurityFirewallAddressList),
    /// `BigipSecurityFirewallGlobalRules`.
    SecurityFirewallGlobalRules(BigipSecurityFirewallGlobalRules),
    /// `BigipSecurityFirewallManagementIpRules`.
    SecurityFirewallManagementIpRules(BigipSecurityFirewallManagementIpRules),
    /// `BigipSecurityFirewallSchedule`.
    SecurityFirewallSchedule(BigipSecurityFirewallSchedule),
    /// `BigipSecurityFirewallUserList`.
    SecurityFirewallUserList(BigipSecurityFirewallUserList),
    /// `BigipSecurityFirewallUserDomain`.
    SecurityFirewallUserDomain(BigipSecurityFirewallUserDomain),
    /// `BigipSecurityFirewallGlobalFqdnPolicy`.
    SecurityFirewallGlobalFqdnPolicy(BigipSecurityFirewallGlobalFqdnPolicy),
    /// `BigipSecurityFirewallPortMisusePolicy`.
    SecurityFirewallPortMisusePolicy(BigipSecurityFirewallPortMisusePolicy),
    /// `BigipSecurityFirewallOnDemandCompilation`.
    SecurityFirewallOnDemandCompilation(BigipSecurityFirewallOnDemandCompilation),
    /// `BigipSecurityFirewallOnDemandRuleDeploy`.
    SecurityFirewallOnDemandRuleDeploy(BigipSecurityFirewallOnDemandRuleDeploy),
    /// `BigipSecurityFirewallUuidDefaultAutogenerate`.
    SecurityFirewallUuidDefaultAutogenerate(BigipSecurityFirewallUuidDefaultAutogenerate),
    /// `BigipSecurityFirewallConfigChangeLog`.
    SecurityFirewallConfigChangeLog(BigipSecurityFirewallConfigChangeLog),
    /// `BigipSecurityNatPolicy`.
    SecurityNatPolicy(BigipSecurityNatPolicy),
    /// `BigipSecurityNatSourceTranslation`.
    SecurityNatSourceTranslation(BigipSecurityNatSourceTranslation),
    /// `BigipSecurityNatDestinationTranslation`.
    SecurityNatDestinationTranslation(BigipSecurityNatDestinationTranslation),
    /// `BigipSecurityLogProfile`.
    SecurityLogProfile(BigipSecurityLogProfile),
    /// `BigipSecurityDosProfile`.
    SecurityDosProfile(BigipSecurityDosProfile),
    /// `BigipSecurityIpIntelligenceFeedList`.
    SecurityIpIntelligenceFeedList(BigipSecurityIpIntelligenceFeedList),
    /// `BigipSecurityIpIntelligenceGlobalPolicy`.
    SecurityIpIntelligenceGlobalPolicy(BigipSecurityIpIntelligenceGlobalPolicy),
    /// `BigipSecurityZone`.
    SecurityZone(BigipSecurityZone),
    /// `BigipSecurityProtectedZone`.
    SecurityProtectedZone(BigipSecurityProtectedZone),
    /// `BigipSecurityPacketFilterPolicy`.
    SecurityPacketFilterPolicy(BigipSecurityPacketFilterPolicy),
    /// `BigipSecurityPacketFilterDefaultRules`.
    SecurityPacketFilterDefaultRules(BigipSecurityPacketFilterDefaultRules),
    /// `BigipSecuritySshProfile`.
    SecuritySshProfile(BigipSecuritySshProfile),
    /// `BigipSecurityHttpProfile`.
    SecurityHttpProfile(BigipSecurityHttpProfile),
    /// `BigipSecurityBotDefenseProfile`.
    SecurityBotDefenseProfile(BigipSecurityBotDefenseProfile),
    /// `BigipSecurityIpIntelligencePolicy`.
    SecurityIpIntelligencePolicy(BigipSecurityIpIntelligencePolicy),
    /// `BigipSecurityProtocolInspectionComplianceMap`.
    SecurityProtocolInspectionComplianceMap(BigipSecurityProtocolInspectionComplianceMap),
    /// `BigipSecurityProtocolInspectionComplianceObject`.
    SecurityProtocolInspectionComplianceObject(BigipSecurityProtocolInspectionComplianceObject),
    /// `BigipSecurityDeviceIdAttribute`.
    SecurityDeviceIdAttribute(BigipSecurityDeviceIdAttribute),
    /// `BigipApmEphemeralAuthSshSecurityConfig`.
    ApmEphemeralAuthSshSecurityConfig(BigipApmEphemeralAuthSshSecurityConfig),
    /// `BigipApmOauthDbInstance`.
    ApmOauthDbInstance(BigipApmOauthDbInstance),
    /// `BigipApmPolicyAccessPolicy`.
    ApmPolicyAccessPolicy(BigipApmPolicyAccessPolicy),
    /// `BigipApmPolicyCustomizationSource`.
    ApmPolicyCustomizationSource(BigipApmPolicyCustomizationSource),
    /// `BigipApmPolicyItem`.
    ApmPolicyItem(BigipApmPolicyItem),
    /// `BigipApmPolicyAgent`.
    ApmPolicyAgent(BigipApmPolicyAgent),
    /// `BigipApmReportDefaultReport`.
    ApmReportDefaultReport(BigipApmReportDefaultReport),
    /// `BigipCmCert`.
    CmCert(BigipCmCert),
    /// `BigipCmKey`.
    CmKey(BigipCmKey),
    /// `BigipCmDevice`.
    CmDevice(BigipCmDevice),
    /// `BigipCmDeviceGroup`.
    CmDeviceGroup(BigipCmDeviceGroup),
    /// `BigipCmTrafficGroup`.
    CmTrafficGroup(BigipCmTrafficGroup),
    /// `BigipCmTrustDomain`.
    CmTrustDomain(BigipCmTrustDomain),
    /// `BigipGtmDatacenter`.
    GtmDatacenter(BigipGtmDatacenter),
    /// `BigipGtmServer`.
    GtmServer(BigipGtmServer),
    /// `BigipGtmPool`.
    GtmPool(BigipGtmPool),
    /// `BigipGtmWideip`.
    GtmWideip(BigipGtmWideip),
    /// `BigipGtmProberPool`.
    GtmProberPool(BigipGtmProberPool),
    /// `BigipGtmRegion`.
    GtmRegion(BigipGtmRegion),
    /// `BigipGtmRule`.
    GtmRule(BigipGtmRule),
    /// `BigipGtmListener`.
    GtmListener(BigipGtmListener),
    /// `BigipGtmListenerDohProxy`.
    GtmListenerDohProxy(BigipGtmListenerDohProxy),
    /// `BigipGtmListenerDohServer`.
    GtmListenerDohServer(BigipGtmListenerDohServer),
    /// `BigipGtmLink`.
    GtmLink(BigipGtmLink),
    /// `BigipGtmTopology`.
    GtmTopology(BigipGtmTopology),
    /// `BigipGtmDistributedApp`.
    GtmDistributedApp(BigipGtmDistributedApp),
    /// `BigipGtmGlobalSettingsGeneral`.
    GtmGlobalSettingsGeneral(BigipGtmGlobalSettingsGeneral),
    /// `BigipGtmGlobalSettingsLoadBalancing`.
    GtmGlobalSettingsLoadBalancing(BigipGtmGlobalSettingsLoadBalancing),
    /// `BigipGtmGlobalSettingsMetrics`.
    GtmGlobalSettingsMetrics(BigipGtmGlobalSettingsMetrics),
    /// `BigipGtmGlobalSettingsMetricsExclusions`.
    GtmGlobalSettingsMetricsExclusions(BigipGtmGlobalSettingsMetricsExclusions),
    /// `BigipPemPolicy`.
    PemPolicy(BigipPemPolicy),
    /// `BigipPemRule`.
    PemRule(BigipPemRule),
    /// `BigipPemListener`.
    PemListener(BigipPemListener),
    /// `BigipPemForwardingEndpoint`.
    PemForwardingEndpoint(BigipPemForwardingEndpoint),
    /// `BigipPemInterceptionEndpoint`.
    PemInterceptionEndpoint(BigipPemInterceptionEndpoint),
    /// `BigipPemServiceChainEndpoint`.
    PemServiceChainEndpoint(BigipPemServiceChainEndpoint),
    /// `BigipPemProfile`.
    PemProfile(BigipPemProfile),
    /// `BigipPemRatingGroup`.
    PemRatingGroup(BigipPemRatingGroup),
    /// `BigipAuthPartition`.
    AuthPartition(BigipAuthPartition),
    /// `BigipAuthUser`.
    AuthUser(BigipAuthUser),
    /// `BigipAuthPassword`.
    AuthPassword(BigipAuthPassword),
    /// `BigipAuthPasswordPolicy`.
    AuthPasswordPolicy(BigipAuthPasswordPolicy),
    /// `BigipAuthSource`.
    AuthSource(BigipAuthSource),
    /// `BigipAuthRemoteRole`.
    AuthRemoteRole(BigipAuthRemoteRole),
    /// `BigipAuthRemoteUser`.
    AuthRemoteUser(BigipAuthRemoteUser),
    /// `BigipAuthLoginFailures`.
    AuthLoginFailures(BigipAuthLoginFailures),
    /// `BigipAuthLdap`.
    AuthLdap(BigipAuthLdap),
    /// `BigipAuthRadius`.
    AuthRadius(BigipAuthRadius),
    /// `BigipAuthRadiusServer`.
    AuthRadiusServer(BigipAuthRadiusServer),
    /// `BigipAuthTacacs`.
    AuthTacacs(BigipAuthTacacs),
    /// `BigipAuthCertLdap`.
    AuthCertLdap(BigipAuthCertLdap),
    /// `BigipAuthApmAuth`.
    AuthApmAuth(BigipAuthApmAuth),
    /// `BigipCmHaGroup`.
    CmHaGroup(BigipCmHaGroup),
    /// `BigipGenericObject`.
    Generic(BigipGenericObject),
}

impl ModelObject {
    /// Canonical `"d"` field map for this object.
    #[must_use]
    pub fn canon_fields(&self) -> serde_json::Value {
        use crate::canonical::Canon;
        match self {
            Self::DataGroup(v) => v.canon_fields(),
            Self::Pool(v) => v.canon_fields(),
            Self::VirtualServer(v) => v.canon_fields(),
            Self::VirtualAddress(v) => v.canon_fields(),
            Self::LtmCipherGroup(v) => v.canon_fields(),
            Self::LtmCipherRule(v) => v.canon_fields(),
            Self::LtmNat(v) => v.canon_fields(),
            Self::LtmSnat(v) => v.canon_fields(),
            Self::LtmSnatTranslation(v) => v.canon_fields(),
            Self::LtmPolicyStrategy(v) => v.canon_fields(),
            Self::LtmRateClass(v) => v.canon_fields(),
            Self::LtmTrafficClass(v) => v.canon_fields(),
            Self::LtmTrafficMatchingCriteria(v) => v.canon_fields(),
            Self::LtmIfile(v) => v.canon_fields(),
            Self::LtmEvictionPolicy(v) => v.canon_fields(),
            Self::LtmDnsNameserver(v) => v.canon_fields(),
            Self::LtmDnsTsigKey(v) => v.canon_fields(),
            Self::LtmDnsZone(v) => v.canon_fields(),
            Self::LtmDnsDnssecKey(v) => v.canon_fields(),
            Self::LtmDnsDnssecZone(v) => v.canon_fields(),
            Self::LtmDnsCacheResolver(v) => v.canon_fields(),
            Self::LtmDnsCacheTransparent(v) => v.canon_fields(),
            Self::LtmDnsCacheValidatingResolver(v) => v.canon_fields(),
            Self::LtmDnsCacheGlobalSettings(v) => v.canon_fields(),
            Self::LtmDnsCacheRecord(v) => v.canon_fields(),
            Self::LtmDnsHpkeKey(v) => v.canon_fields(),
            Self::LtmDnsHpkeProfile(v) => v.canon_fields(),
            Self::LtmDnsAnalyticsGlobalSettings(v) => v.canon_fields(),
            Self::LtmMessageRoutingObject(v) => v.canon_fields(),
            Self::LtmAuthObject(v) => v.canon_fields(),
            Self::Minimal(v) => v.canon_fields(),
            Self::Node(v) => v.canon_fields(),
            Self::Profile(v) => v.canon_fields(),
            Self::Monitor(v) => v.canon_fields(),
            Self::SnatPool(v) => v.canon_fields(),
            Self::Persistence(v) => v.canon_fields(),
            Self::Rule(v) => v.canon_fields(),
            Self::Policy(v) => v.canon_fields(),
            Self::NetRoute(v) => v.canon_fields(),
            Self::NetVlan(v) => v.canon_fields(),
            Self::NetSelf(v) => v.canon_fields(),
            Self::NetRouteDomain(v) => v.canon_fields(),
            Self::NetPortList(v) => v.canon_fields(),
            Self::NetInterface(v) => v.canon_fields(),
            Self::NetDnsResolver(v) => v.canon_fields(),
            Self::NetTunnel(v) => v.canon_fields(),
            Self::NetStp(v) => v.canon_fields(),
            Self::SysDns(v) => v.canon_fields(),
            Self::SysNtp(v) => v.canon_fields(),
            Self::SysSnmp(v) => v.canon_fields(),
            Self::SysGlobalSettings(v) => v.canon_fields(),
            Self::SysProvision(v) => v.canon_fields(),
            Self::SysFolder(v) => v.canon_fields(),
            Self::SysFileSslCert(v) => v.canon_fields(),
            Self::SysFileSslKey(v) => v.canon_fields(),
            Self::SysManagementRoute(v) => v.canon_fields(),
            Self::SecurityFirewallPortList(v) => v.canon_fields(),
            Self::SecurityFirewallRuleList(v) => v.canon_fields(),
            Self::SecurityFirewallConfigEntityId(v) => v.canon_fields(),
            Self::SecurityFirewallPolicy(v) => v.canon_fields(),
            Self::SecurityFirewallAddressList(v) => v.canon_fields(),
            Self::SecurityFirewallGlobalRules(v) => v.canon_fields(),
            Self::SecurityFirewallManagementIpRules(v) => v.canon_fields(),
            Self::SecurityFirewallSchedule(v) => v.canon_fields(),
            Self::SecurityFirewallUserList(v) => v.canon_fields(),
            Self::SecurityFirewallUserDomain(v) => v.canon_fields(),
            Self::SecurityFirewallGlobalFqdnPolicy(v) => v.canon_fields(),
            Self::SecurityFirewallPortMisusePolicy(v) => v.canon_fields(),
            Self::SecurityFirewallOnDemandCompilation(v) => v.canon_fields(),
            Self::SecurityFirewallOnDemandRuleDeploy(v) => v.canon_fields(),
            Self::SecurityFirewallUuidDefaultAutogenerate(v) => v.canon_fields(),
            Self::SecurityFirewallConfigChangeLog(v) => v.canon_fields(),
            Self::SecurityNatPolicy(v) => v.canon_fields(),
            Self::SecurityNatSourceTranslation(v) => v.canon_fields(),
            Self::SecurityNatDestinationTranslation(v) => v.canon_fields(),
            Self::SecurityLogProfile(v) => v.canon_fields(),
            Self::SecurityDosProfile(v) => v.canon_fields(),
            Self::SecurityIpIntelligenceFeedList(v) => v.canon_fields(),
            Self::SecurityIpIntelligenceGlobalPolicy(v) => v.canon_fields(),
            Self::SecurityZone(v) => v.canon_fields(),
            Self::SecurityProtectedZone(v) => v.canon_fields(),
            Self::SecurityPacketFilterPolicy(v) => v.canon_fields(),
            Self::SecurityPacketFilterDefaultRules(v) => v.canon_fields(),
            Self::SecuritySshProfile(v) => v.canon_fields(),
            Self::SecurityHttpProfile(v) => v.canon_fields(),
            Self::SecurityBotDefenseProfile(v) => v.canon_fields(),
            Self::SecurityIpIntelligencePolicy(v) => v.canon_fields(),
            Self::SecurityProtocolInspectionComplianceMap(v) => v.canon_fields(),
            Self::SecurityProtocolInspectionComplianceObject(v) => v.canon_fields(),
            Self::SecurityDeviceIdAttribute(v) => v.canon_fields(),
            Self::ApmEphemeralAuthSshSecurityConfig(v) => v.canon_fields(),
            Self::ApmOauthDbInstance(v) => v.canon_fields(),
            Self::ApmPolicyAccessPolicy(v) => v.canon_fields(),
            Self::ApmPolicyCustomizationSource(v) => v.canon_fields(),
            Self::ApmPolicyItem(v) => v.canon_fields(),
            Self::ApmPolicyAgent(v) => v.canon_fields(),
            Self::ApmReportDefaultReport(v) => v.canon_fields(),
            Self::CmCert(v) => v.canon_fields(),
            Self::CmKey(v) => v.canon_fields(),
            Self::CmDevice(v) => v.canon_fields(),
            Self::CmDeviceGroup(v) => v.canon_fields(),
            Self::CmTrafficGroup(v) => v.canon_fields(),
            Self::CmTrustDomain(v) => v.canon_fields(),
            Self::GtmDatacenter(v) => v.canon_fields(),
            Self::GtmServer(v) => v.canon_fields(),
            Self::GtmPool(v) => v.canon_fields(),
            Self::GtmWideip(v) => v.canon_fields(),
            Self::GtmProberPool(v) => v.canon_fields(),
            Self::GtmRegion(v) => v.canon_fields(),
            Self::GtmRule(v) => v.canon_fields(),
            Self::GtmListener(v) => v.canon_fields(),
            Self::GtmListenerDohProxy(v) => v.canon_fields(),
            Self::GtmListenerDohServer(v) => v.canon_fields(),
            Self::GtmLink(v) => v.canon_fields(),
            Self::GtmTopology(v) => v.canon_fields(),
            Self::GtmDistributedApp(v) => v.canon_fields(),
            Self::GtmGlobalSettingsGeneral(v) => v.canon_fields(),
            Self::GtmGlobalSettingsLoadBalancing(v) => v.canon_fields(),
            Self::GtmGlobalSettingsMetrics(v) => v.canon_fields(),
            Self::GtmGlobalSettingsMetricsExclusions(v) => v.canon_fields(),
            Self::PemPolicy(v) => v.canon_fields(),
            Self::PemRule(v) => v.canon_fields(),
            Self::PemListener(v) => v.canon_fields(),
            Self::PemForwardingEndpoint(v) => v.canon_fields(),
            Self::PemInterceptionEndpoint(v) => v.canon_fields(),
            Self::PemServiceChainEndpoint(v) => v.canon_fields(),
            Self::PemProfile(v) => v.canon_fields(),
            Self::PemRatingGroup(v) => v.canon_fields(),
            Self::AuthPartition(v) => v.canon_fields(),
            Self::AuthUser(v) => v.canon_fields(),
            Self::AuthPassword(v) => v.canon_fields(),
            Self::AuthPasswordPolicy(v) => v.canon_fields(),
            Self::AuthSource(v) => v.canon_fields(),
            Self::AuthRemoteRole(v) => v.canon_fields(),
            Self::AuthRemoteUser(v) => v.canon_fields(),
            Self::AuthLoginFailures(v) => v.canon_fields(),
            Self::AuthLdap(v) => v.canon_fields(),
            Self::AuthRadius(v) => v.canon_fields(),
            Self::AuthRadiusServer(v) => v.canon_fields(),
            Self::AuthTacacs(v) => v.canon_fields(),
            Self::AuthCertLdap(v) => v.canon_fields(),
            Self::AuthApmAuth(v) => v.canon_fields(),
            Self::CmHaGroup(v) => v.canon_fields(),
            Self::Generic(v) => v.canon_fields(),
        }
    }
}
