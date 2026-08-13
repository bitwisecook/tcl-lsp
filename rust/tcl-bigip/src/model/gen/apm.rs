// @generated — do not edit.
//! Generated BIG-IP `apm` model structs.

// Generated BIG-IP config records are flat data structs of
// independent, orthogonal boolean attributes (mirroring the tmsh
// object schema) — not state machines, so struct_excessive_bools is
// a false positive here and is allowed deliberately.
#![allow(clippy::struct_excessive_bools)]
#![allow(unused_imports)]

use super::*;

/// `BigipApmEphemeralAuthSshSecurityConfig`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipApmEphemeralAuthSshSecurityConfig {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `ciphers`
    pub ciphers: Vec<String>,
    /// `hmacs`
    pub hmacs: Vec<String>,
    /// `kex_methods`
    pub kex_methods: Vec<String>,
    /// `compressions`
    pub compressions: Vec<String>,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipApmOauthDbInstance`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipApmOauthDbInstance {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `db_name`
    pub db_name: String,
    /// `purge_frequency`
    pub purge_frequency: String,
    /// `purge_time`
    pub purge_time: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipApmPolicyAccessPolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipApmPolicyAccessPolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `start_item`
    pub start_item: String,
    /// `default_ending`
    pub default_ending: String,
    /// `items`
    pub items: Vec<String>,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipApmPolicyAgent`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipApmPolicyAgent {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `agent_type`
    pub agent_type: String,
    /// `customization_group`
    pub customization_group: String,
    /// `auth`
    pub auth: String,
    /// `max_logon_attempt`
    pub max_logon_attempt: String,
    /// `auth_max_logon_attempt`
    pub auth_max_logon_attempt: String,
    /// `fetch_nested_groups`
    pub fetch_nested_groups: String,
    /// `fetch_primary_groups`
    pub fetch_primary_groups: String,
    /// `password_source`
    pub password_source: String,
    /// `query`
    pub query: String,
    /// `query_attrname`
    pub query_attrname: String,
    /// `query_filter`
    pub query_filter: String,
    /// `server`
    pub server: String,
    /// `show_extended_error`
    pub show_extended_error: String,
    /// `upn`
    pub upn: String,
    /// `username_source`
    pub username_source: String,
    /// `attribute_consuming_service`
    pub attribute_consuming_service: String,
    /// `attr_consuming_service_session_var`
    pub attr_consuming_service_session_var: String,
    /// `hints`
    pub hints: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipApmPolicyCustomizationSource`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipApmPolicyCustomizationSource {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipApmPolicyItem`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipApmPolicyItem {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `caption`
    pub caption: String,
    /// `color`
    pub color: String,
    /// `item_type`
    pub item_type: String,
    /// `agents`
    pub agents: Vec<String>,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipApmReportDefaultReport`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipApmReportDefaultReport {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `report_name`
    pub report_name: String,
    /// `user`
    pub user: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}
