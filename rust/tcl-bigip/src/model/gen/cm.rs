// @generated — do not edit.
//! Generated BIG-IP `cm` model structs.

// Generated BIG-IP config records are flat data structs of
// independent, orthogonal boolean attributes (mirroring the tmsh
// object schema) — not state machines, so struct_excessive_bools is
// a false positive here and is allowed deliberately.
#![allow(clippy::struct_excessive_bools)]
#![allow(unused_imports)]

use super::*;

/// `BigipCmCert`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipCmCert {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `cache_path`
    pub cache_path: String,
    /// `checksum`
    pub checksum: String,
    /// `revision`
    pub revision: String,
    /// `issuer`
    pub issuer: String,
    /// `subject`
    pub subject: String,
    /// `subject_alternative_name`
    pub subject_alternative_name: String,
    /// `expiration_date`
    pub expiration_date: String,
    /// `expiration_string`
    pub expiration_string: String,
    /// `fingerprint`
    pub fingerprint: String,
    /// `serial_number`
    pub serial_number: String,
    /// `version`
    pub version: String,
    /// `key_type`
    pub key_type: String,
    /// `certificate_key_size`
    pub certificate_key_size: String,
    /// `is_bundle`
    pub is_bundle: String,
    /// `email`
    pub email: String,
    /// `source_path`
    pub source_path: String,
    /// `system_path`
    pub system_path: String,
    /// `size`
    pub size: String,
    /// `mode`
    pub mode: String,
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

/// `BigipCmDevice`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipCmDevice {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `hostname`
    pub hostname: String,
    /// `management_ip`
    pub management_ip: String,
    /// `base_mac`
    pub base_mac: String,
    /// `build`
    pub build: String,
    /// `edition`
    pub edition: String,
    /// `version`
    pub version: String,
    /// `product`
    pub product: String,
    /// `platform_id`
    pub platform_id: String,
    /// `chassis_id`
    pub chassis_id: String,
    /// `marketing_name`
    pub marketing_name: String,
    /// `self_device`
    pub self_device: String,
    /// `time_zone`
    pub time_zone: String,
    /// `cert`
    pub cert: String,
    /// `key`
    pub key: String,
    /// `description`
    pub description: String,
    /// `comment`
    pub comment: String,
    /// `contact`
    pub contact: String,
    /// `location`
    pub location: String,
    /// `mirror_ip`
    pub mirror_ip: String,
    /// `mirror_secondary_ip`
    pub mirror_secondary_ip: String,
    /// `multicast_interface`
    pub multicast_interface: String,
    /// `multicast_ip`
    pub multicast_ip: String,
    /// `multicast_port`
    pub multicast_port: String,
    /// `unicast_address`
    pub unicast_address: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipCmDeviceGroup`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipCmDeviceGroup {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `auto_sync`
    pub auto_sync: String,
    /// `network_failover`
    pub network_failover: String,
    /// `hidden`
    pub hidden: String,
    /// `devices`
    pub devices: Vec<String>,
    /// `description`
    pub description: String,
    /// `type_`
    pub type_: String,
    /// `save_on_auto_sync`
    pub save_on_auto_sync: String,
    /// `full_load_on_sync`
    pub full_load_on_sync: String,
    /// `asm_sync`
    pub asm_sync: String,
    /// `incremental_config_sync_size_max`
    pub incremental_config_sync_size_max: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipCmHaGroup`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipCmHaGroup {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `enabled_state`
    pub enabled_state: String,
    /// `active_bonus`
    pub active_bonus: String,
    /// `pools`
    pub pools: Vec<String>,
    /// `trunks`
    pub trunks: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipCmKey`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipCmKey {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `cache_path`
    pub cache_path: String,
    /// `checksum`
    pub checksum: String,
    /// `revision`
    pub revision: String,
    /// `key_size`
    pub key_size: String,
    /// `key_type`
    pub key_type: String,
    /// `security_type`
    pub security_type: String,
    /// `source_path`
    pub source_path: String,
    /// `system_path`
    pub system_path: String,
    /// `size`
    pub size: String,
    /// `mode`
    pub mode: String,
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

/// `BigipCmTrafficGroup`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipCmTrafficGroup {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `unit_id`
    pub unit_id: String,
    /// `description`
    pub description: String,
    /// `default_device`
    pub default_device: String,
    /// `ha_load_factor`
    pub ha_load_factor: String,
    /// `ha_order`
    pub ha_order: Vec<String>,
    /// `ha_group`
    pub ha_group: String,
    /// `auto_failback_enabled`
    pub auto_failback_enabled: String,
    /// `auto_failback_time`
    pub auto_failback_time: String,
    /// `mac`
    pub mac: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipCmTrustDomain`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipCmTrustDomain {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `ca_cert`
    pub ca_cert: String,
    /// `ca_cert_bundle`
    pub ca_cert_bundle: String,
    /// `ca_key`
    pub ca_key: String,
    /// `ca_devices`
    pub ca_devices: Vec<String>,
    /// `guid`
    pub guid: String,
    /// `status`
    pub status: String,
    /// `trust_group`
    pub trust_group: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}
