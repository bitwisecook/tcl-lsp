// @generated — do not edit.
//! Generated BIG-IP `sys` model structs.

// Generated BIG-IP config records are flat data structs of
// independent, orthogonal boolean attributes (mirroring the tmsh
// object schema) — not state machines, so struct_excessive_bools is
// a false positive here and is allowed deliberately.
#![allow(clippy::struct_excessive_bools)]
#![allow(unused_imports)]

use super::*;

/// `BigipSysDns`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysDns {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `name_servers`
    pub name_servers: Vec<String>,
    /// `search`
    pub search: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysFileSslCert`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysFileSslCert {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `source_path`
    pub source_path: String,
    /// `cache_path`
    pub cache_path: String,
    /// `revision`
    pub revision: String,
    /// `description`
    pub description: String,
    /// `issuer`
    pub issuer: String,
    /// `subject`
    pub subject: String,
    /// `expiration_string`
    pub expiration_string: String,
    /// `expiration_date`
    pub expiration_date: String,
    /// `fingerprint`
    pub fingerprint: String,
    /// `key_size`
    pub key_size: String,
    /// `key_type`
    pub key_type: String,
    /// `is_bundle`
    pub is_bundle: String,
    /// `certificate_key_size`
    pub certificate_key_size: String,
    /// `issuer_cert`
    pub issuer_cert: String,
    /// `serial_number`
    pub serial_number: String,
    /// `version`
    pub version: String,
    /// `subject_alternative_name`
    pub subject_alternative_name: String,
    /// `bundle_certificates`
    pub bundle_certificates: Vec<String>,
    /// `cert_validation_options`
    pub cert_validation_options: Vec<String>,
    /// `cert_validators`
    pub cert_validators: Vec<String>,
    /// `checksum`
    pub checksum: String,
    /// `mode`
    pub mode: String,
    /// `size`
    pub size: String,
    /// `create_time`
    pub create_time: String,
    /// `created_by`
    pub created_by: String,
    /// `last_update_time`
    pub last_update_time: String,
    /// `updated_by`
    pub updated_by: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysFileSslKey`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysFileSslKey {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `source_path`
    pub source_path: String,
    /// `cache_path`
    pub cache_path: String,
    /// `revision`
    pub revision: String,
    /// `passphrase`
    pub passphrase: String,
    /// `description`
    pub description: String,
    /// `key_size`
    pub key_size: String,
    /// `key_type`
    pub key_type: String,
    /// `security_type`
    pub security_type: String,
    /// `checksum`
    pub checksum: String,
    /// `mode`
    pub mode: String,
    /// `size`
    pub size: String,
    /// `create_time`
    pub create_time: String,
    /// `created_by`
    pub created_by: String,
    /// `last_update_time`
    pub last_update_time: String,
    /// `updated_by`
    pub updated_by: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysFolder`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysFolder {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `device_group`
    pub device_group: String,
    /// `traffic_group`
    pub traffic_group: String,
    /// `hidden`
    pub hidden: String,
    /// `description`
    pub description: String,
    /// `inherited_device_group`
    pub inherited_device_group: String,
    /// `inherited_traffic_group`
    pub inherited_traffic_group: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysGlobalSettings`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysGlobalSettings {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `hostname`
    pub hostname: String,
    /// `gui_setup`
    pub gui_setup: String,
    /// `mgmt_dhcp`
    pub mgmt_dhcp: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysManagementRoute`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysManagementRoute {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `gateway`
    pub gateway: String,
    /// `network`
    pub network: String,
    /// `mtu`
    pub mtu: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysNtp`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysNtp {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `servers`
    pub servers: Vec<String>,
    /// `timezone`
    pub timezone: String,
    /// `restrict`
    pub restrict: Vec<BigipSysNtpRestrict>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysNtpRestrict`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysNtpRestrict {
    /// `name`
    pub name: String,
    /// `address`
    pub address: String,
    /// `mask`
    pub mask: String,
    /// `default_entry`
    pub default_entry: String,
    /// `flags`
    pub flags: Vec<String>,
    /// `description`
    pub description: String,
}

/// `BigipSysProvision`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysProvision {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `level`
    pub level: String,
    /// `cpu_ratio`
    pub cpu_ratio: String,
    /// `memory_ratio`
    pub memory_ratio: String,
    /// `disk_ratio`
    pub disk_ratio: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysSnmp`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysSnmp {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `agent_addresses`
    pub agent_addresses: Vec<String>,
    /// `communities`
    pub communities: Vec<String>,
    /// `sys_contact`
    pub sys_contact: String,
    /// `sys_location`
    pub sys_location: String,
    /// `sys_services`
    pub sys_services: String,
    /// `trap_community`
    pub trap_community: String,
    /// `users`
    pub users: Vec<BigipSysSnmpUser>,
    /// `traps`
    pub traps: Vec<BigipSysSnmpTrap>,
    /// `process_monitors`
    pub process_monitors: Vec<BigipSysSnmpProcessMonitor>,
    /// `disk_monitors`
    pub disk_monitors: Vec<BigipSysSnmpDiskMonitor>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipSysSnmpDiskMonitor`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysSnmpDiskMonitor {
    /// `name`
    pub name: String,
    /// `partition`
    pub partition: String,
    /// `min_space`
    pub min_space: String,
    /// `description`
    pub description: String,
}

/// `BigipSysSnmpProcessMonitor`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysSnmpProcessMonitor {
    /// `name`
    pub name: String,
    /// `process`
    pub process: String,
    /// `max_processes`
    pub max_processes: String,
    /// `min_processes`
    pub min_processes: String,
    /// `description`
    pub description: String,
}

/// `BigipSysSnmpTrap`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysSnmpTrap {
    /// `name`
    pub name: String,
    /// `host`
    pub host: String,
    /// `port`
    pub port: String,
    /// `version`
    pub version: String,
    /// `community`
    pub community: String,
    /// `security_name`
    pub security_name: String,
    /// `security_level`
    pub security_level: String,
    /// `auth_protocol`
    pub auth_protocol: String,
    /// `privacy_protocol`
    pub privacy_protocol: String,
    /// `network`
    pub network: String,
    /// `description`
    pub description: String,
}

/// `BigipSysSnmpUser`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipSysSnmpUser {
    /// `name`
    pub name: String,
    /// `username`
    pub username: String,
    /// `security_level`
    pub security_level: String,
    /// `auth_protocol`
    pub auth_protocol: String,
    /// `privacy_protocol`
    pub privacy_protocol: String,
    /// `oid_subset`
    pub oid_subset: String,
    /// `description`
    pub description: String,
}
