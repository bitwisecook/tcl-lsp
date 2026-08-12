// @generated — do not edit.
//! Generated BIG-IP `auth` model structs.

// Generated BIG-IP config records are flat data structs of
// independent, orthogonal boolean attributes (mirroring the tmsh
// object schema) — not state machines, so struct_excessive_bools is
// a false positive here and is allowed deliberately.
#![allow(clippy::struct_excessive_bools)]
#![allow(unused_imports)]

use super::*;

/// `BigipAuthApmAuth`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthApmAuth {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `profile`
    pub profile: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthCertLdap`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthCertLdap {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `bind_dn`
    pub bind_dn: String,
    /// `bind_pw`
    pub bind_pw: String,
    /// `bind_timeout`
    pub bind_timeout: String,
    /// `idle_timeout`
    pub idle_timeout: String,
    /// `login_attribute`
    pub login_attribute: String,
    /// `port`
    pub port: String,
    /// `scope`
    pub scope: String,
    /// `search_base_dn`
    pub search_base_dn: String,
    /// `search_timeout`
    pub search_timeout: String,
    /// `servers`
    pub servers: Vec<String>,
    /// `ssl`
    pub ssl: String,
    /// `user_template`
    pub user_template: String,
    /// `version`
    pub version: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthLdap`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthLdap {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `bind_dn`
    pub bind_dn: String,
    /// `bind_pw`
    pub bind_pw: String,
    /// `bind_timeout`
    pub bind_timeout: String,
    /// `check_host_attr`
    pub check_host_attr: String,
    /// `check_roles_group`
    pub check_roles_group: String,
    /// `filter_`
    pub filter_: String,
    /// `group_dn`
    pub group_dn: String,
    /// `group_member_attribute`
    pub group_member_attribute: String,
    /// `idle_timeout`
    pub idle_timeout: String,
    /// `ignore_auth_info_unavail`
    pub ignore_auth_info_unavail: String,
    /// `ignore_unknown_user`
    pub ignore_unknown_user: String,
    /// `login_attribute`
    pub login_attribute: String,
    /// `port`
    pub port: String,
    /// `scope`
    pub scope: String,
    /// `search_base_dn`
    pub search_base_dn: String,
    /// `search_timeout`
    pub search_timeout: String,
    /// `servers`
    pub servers: Vec<String>,
    /// `ssl`
    pub ssl: String,
    /// `ssl_ca_cert`
    pub ssl_ca_cert: String,
    /// `ssl_check_peer`
    pub ssl_check_peer: String,
    /// `ssl_client_cert`
    pub ssl_client_cert: String,
    /// `ssl_client_key`
    pub ssl_client_key: String,
    /// `user_template`
    pub user_template: String,
    /// `version`
    pub version: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthLoginFailures`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthLoginFailures {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthPartition`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthPartition {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `default_route_domain`
    pub default_route_domain: String,
    /// `inherited_traffic_group`
    pub inherited_traffic_group: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthPassword`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthPassword {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `expiration_warning`
    pub expiration_warning: String,
    /// `minimum_length`
    pub minimum_length: String,
    /// `policy`
    pub policy: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthPasswordPolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthPasswordPolicy {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `expiration_warning`
    pub expiration_warning: String,
    /// `max_duration`
    pub max_duration: String,
    /// `max_login_failures`
    pub max_login_failures: String,
    /// `min_duration`
    pub min_duration: String,
    /// `minimum_length`
    pub minimum_length: String,
    /// `minimum_regular_characters`
    pub minimum_regular_characters: String,
    /// `password_memory`
    pub password_memory: String,
    /// `policy_enforcement`
    pub policy_enforcement: String,
    /// `required_lowercase`
    pub required_lowercase: String,
    /// `required_numeric`
    pub required_numeric: String,
    /// `required_special`
    pub required_special: String,
    /// `required_uppercase`
    pub required_uppercase: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthRadius`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthRadius {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `service_type`
    pub service_type: String,
    /// `servers`
    pub servers: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthRadiusServer`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthRadiusServer {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `server`
    pub server: String,
    /// `port`
    pub port: String,
    /// `secret`
    pub secret: String,
    /// `timeout`
    pub timeout: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthRemoteRole`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthRemoteRole {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `role_info`
    pub role_info: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthRemoteUser`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthRemoteUser {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `default_partition`
    pub default_partition: String,
    /// `default_role`
    pub default_role: String,
    /// `remote_console_access`
    pub remote_console_access: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthSource`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthSource {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `fallback`
    pub fallback: String,
    /// `type_`
    pub type_: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthTacacs`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthTacacs {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `protocol`
    pub protocol: String,
    /// `secret`
    pub secret: String,
    /// `service`
    pub service: String,
    /// `servers`
    pub servers: Vec<String>,
    /// `accounting`
    pub accounting: String,
    /// `authentication`
    pub authentication: String,
    /// `debug`
    pub debug: String,
    /// `encryption`
    pub encryption: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipAuthUser`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipAuthUser {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `partition`
    pub partition: String,
    /// `shell`
    pub shell: String,
    /// `encrypted_password`
    pub encrypted_password: String,
    /// `partition_access`
    pub partition_access: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}
