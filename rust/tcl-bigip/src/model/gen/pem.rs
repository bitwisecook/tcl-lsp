// @generated — do not edit.
//! Generated BIG-IP `pem` model structs.

// Generated BIG-IP config records are flat data structs of
// independent, orthogonal boolean attributes (mirroring the tmsh
// object schema) — not state machines, so struct_excessive_bools is
// a false positive here and is allowed deliberately.
#![allow(clippy::struct_excessive_bools)]
#![allow(unused_imports)]

use super::*;

/// `BigipPemForwardingEndpoint`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipPemForwardingEndpoint {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `pool`
    pub pool: String,
    /// `snat_pool`
    pub snat_pool: String,
    /// `source_ip`
    pub source_ip: String,
    /// `destination_ip`
    pub destination_ip: String,
    /// `type_`
    pub type_: String,
    /// `persistence`
    pub persistence: String,
    /// `translate_address`
    pub translate_address: String,
    /// `translate_service`
    pub translate_service: String,
    /// `fallback`
    pub fallback: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipPemInterceptionEndpoint`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipPemInterceptionEndpoint {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `pool`
    pub pool: String,
    /// `persistence`
    pub persistence: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipPemListener`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipPemListener {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `profile_spm`
    pub profile_spm: String,
    /// `profile_subscriber_mgmt`
    pub profile_subscriber_mgmt: String,
    /// `virtual_servers`
    pub virtual_servers: Vec<String>,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipPemPolicy`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipPemPolicy {
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

/// `BigipPemProfile`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipPemProfile {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `profile_type`
    pub profile_type: String,
    /// `defaults_from`
    pub defaults_from: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipPemRatingGroup`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipPemRatingGroup {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `rating_group_id`
    pub rating_group_id: String,
    /// `default_quota`
    pub default_quota: String,
    /// `default_quota_holding_time`
    pub default_quota_holding_time: String,
    /// `default_validity_time`
    pub default_validity_time: String,
    /// `default_threshold`
    pub default_threshold: String,
    /// `total_octets`
    pub total_octets: String,
    /// `input_octets`
    pub input_octets: String,
    /// `output_octets`
    pub output_octets: String,
    /// `time`
    pub time: String,
    /// `consumption_time`
    pub consumption_time: String,
    /// `usage_time`
    pub usage_time: String,
    /// `volume`
    pub volume: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipPemRule`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipPemRule {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `source`
    pub source: String,
    /// `description`
    pub description: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}

/// `BigipPemServiceChainEndpoint`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BigipPemServiceChainEndpoint {
    /// `name`
    pub name: String,
    /// `full_path`
    pub full_path: String,
    /// `description`
    pub description: String,
    /// `service_endpoints`
    pub service_endpoints: Vec<String>,
    /// `steering_policy`
    pub steering_policy: String,
    /// `range`
    pub range: Option<crate::range::Range>,
}
