// @generated — do not edit.
//! Generated BIG-IP `security` model structs.

// Generated BIG-IP config records are flat data structs of
// independent, orthogonal boolean attributes (mirroring the tmsh
// object schema) — not state machines, so struct_excessive_bools is
// a false positive here and is allowed deliberately.
#![allow(clippy::struct_excessive_bools)]
#![allow(unused_imports)]

use super::*;

/// `BigipSecurityBotDefenseProfile`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityBotDefenseProfile {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `app_service`
    pub app_service: String,
    /// `template`
    pub template: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityDeviceIdAttribute`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityDeviceIdAttribute {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `id_`
    pub id_: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityDosProfile`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityDosProfile {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `app_service`
    pub app_service: String,
    /// `threshold_sensitivity`
    pub threshold_sensitivity: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallAddressList`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallAddressList {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `addresses`
    pub addresses: Vec<String>,
    /// `address_lists`
    pub address_lists: Vec<String>,
    /// `fqdns`
    pub fqdns: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallConfigChangeLog`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallConfigChangeLog {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `log_publisher`
    pub log_publisher: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallConfigEntityId`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallConfigEntityId {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `entity_id`
    pub entity_id: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallGlobalFqdnPolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallGlobalFqdnPolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `context`
    pub context: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallGlobalRules`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallGlobalRules {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `rules`
    pub rules: Vec<String>,
    /// `enforced_policy`
    pub enforced_policy: String,
    /// `staged_policy`
    pub staged_policy: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallManagementIpRules`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallManagementIpRules {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `rules`
    pub rules: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallOnDemandCompilation`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallOnDemandCompilation {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallOnDemandRuleDeploy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallOnDemandRuleDeploy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallPolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallPolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `rules`
    pub rules: Vec<String>,
    /// `rule_lists`
    pub rule_lists: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallPortList`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallPortList {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `ports`
    pub ports: Vec<String>,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallPortMisusePolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallPortMisusePolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `default_log`
    pub default_log: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallRuleList`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallRuleList {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `rules`
    pub rules: Vec<String>,
    /// `rule_objects`
    pub rule_objects: Vec<crate::value::FirewallRule>,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallSchedule`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallSchedule {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `daily_hour_end`
    pub daily_hour_end: String,
    /// `daily_hour_start`
    pub daily_hour_start: String,
    /// `days_of_week`
    pub days_of_week: Vec<String>,
    /// `date_valid_end`
    pub date_valid_end: String,
    /// `date_valid_start`
    pub date_valid_start: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallUserDomain`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallUserDomain {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `domain`
    pub domain: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallUserList`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallUserList {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `users`
    pub users: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityFirewallUuidDefaultAutogenerate`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityFirewallUuidDefaultAutogenerate {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `auto_generate_uuid`
    pub auto_generate_uuid: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityHttpProfile`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityHttpProfile {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `defaults_from`
    pub defaults_from: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityIpIntelligenceFeedList`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityIpIntelligenceFeedList {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `feeds`
    pub feeds: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityIpIntelligenceGlobalPolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityIpIntelligenceGlobalPolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `log_blacklist_category`
    pub log_blacklist_category: String,
    /// `log_publisher`
    pub log_publisher: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityIpIntelligencePolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityIpIntelligencePolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `default_action`
    pub default_action: String,
    /// `default_log_blacklist_hit_only`
    pub default_log_blacklist_hit_only: String,
    /// `default_log_blacklist_category`
    pub default_log_blacklist_category: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityLogProfile`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityLogProfile {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `application_data`
    pub application_data: String,
    /// `network_data`
    pub network_data: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityNatDestinationTranslation`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityNatDestinationTranslation {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `type_`
    pub type_: String,
    /// `addresses`
    pub addresses: Vec<String>,
    /// `ports`
    pub ports: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityNatPolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityNatPolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `rules`
    pub rules: Vec<String>,
    /// `rule_lists`
    pub rule_lists: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityNatSourceTranslation`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityNatSourceTranslation {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `type_`
    pub type_: String,
    /// `addresses`
    pub addresses: Vec<String>,
    /// `ports`
    pub ports: Vec<String>,
    /// `traffic_group`
    pub traffic_group: String,
    /// `egress_interfaces_disabled`
    pub egress_interfaces_disabled: bool,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityPacketFilterDefaultRules`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityPacketFilterDefaultRules {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `action`
    pub action: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityPacketFilterPolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityPacketFilterPolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `rules`
    pub rules: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityProtectedZone`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityProtectedZone {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `enabled`
    pub enabled: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityProtocolInspectionComplianceMap`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityProtocolInspectionComplianceMap {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `insp_id`
    pub insp_id: String,
    /// `key_type`
    pub key_type: String,
    /// `value_type`
    pub value_type: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityProtocolInspectionComplianceObject`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityProtocolInspectionComplianceObject {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `insp_id`
    pub insp_id: String,
    /// `type_`
    pub type_: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecuritySshProfile`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecuritySshProfile {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `defaults_from`
    pub defaults_from: String,
    /// `timeout`
    pub timeout: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSecurityZone`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSecurityZone {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `vlans`
    pub vlans: Vec<String>,
    /// `tunnels`
    pub tunnels: Vec<String>,
    /// `interfaces`
    pub interfaces: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}
