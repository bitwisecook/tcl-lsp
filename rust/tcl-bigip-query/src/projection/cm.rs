// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Field projection of device-cluster objects: certificates, devices, device groups and traffic groups.

use super::ltm::str_list;
use super::{Fields, path_ref, path_ref_list_strs};
use crate::value::Value;
use indexmap::IndexMap;
use tcl_bigip::model::{
    BigipCmCert, BigipCmDevice, BigipCmDeviceGroup, BigipCmHaGroup, BigipCmKey,
    BigipCmTrafficGroup, BigipCmTrustDomain,
};

// CM (device cluster) projections

/// `cm cert` — the device-trust cert. Same x509 metadata spelling as
/// `sys file ssl-cert`, so `x509_from_config` projects either one.
pub(super) fn project_cm_cert(o: &BigipCmCert) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("source-path", &o.source_path)
        .s("cache-path", &o.cache_path)
        .s("system-path", &o.system_path)
        .s("revision", &o.revision)
        .s("issuer", &o.issuer)
        .s("subject", &o.subject)
        .s("subject-alternative-name", &o.subject_alternative_name)
        .s("expiration-string", &o.expiration_string)
        .s("expiration-date", &o.expiration_date)
        .s("fingerprint", &o.fingerprint)
        .s("serial-number", &o.serial_number)
        .s("version", &o.version)
        .s("key-type", &o.key_type)
        .s("certificate-key-size", &o.certificate_key_size)
        .s("is-bundle", &o.is_bundle)
        .s("email", &o.email)
        .s("checksum", &o.checksum)
        .s("mode", &o.mode)
        .s("size", &o.size)
        .s("create-time", &o.create_time)
        .s("created-by", &o.created_by)
        .s("last-update-time", &o.last_update_time)
        .s("updated-by", &o.updated_by)
        .done()
}

pub(super) fn project_cm_key(o: &BigipCmKey) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("source-path", &o.source_path)
        .s("cache-path", &o.cache_path)
        .s("system-path", &o.system_path)
        .s("revision", &o.revision)
        .s("key-size", &o.key_size)
        .s("key-type", &o.key_type)
        .s("security-type", &o.security_type)
        .s("checksum", &o.checksum)
        .s("mode", &o.mode)
        .s("size", &o.size)
        .s("create-time", &o.create_time)
        .s("created-by", &o.created_by)
        .s("last-update-time", &o.last_update_time)
        .s("updated-by", &o.updated_by)
        .done()
}

pub(super) fn project_cm_device(o: &BigipCmDevice) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("hostname", &o.hostname)
        .s("management-ip", &o.management_ip)
        .s("self-device", &o.self_device)
        .s("base-mac", &o.base_mac)
        .s("build", &o.build)
        .s("edition", &o.edition)
        .s("version", &o.version)
        .s("product", &o.product)
        .s("platform-id", &o.platform_id)
        .s("chassis-id", &o.chassis_id)
        .s("marketing-name", &o.marketing_name)
        .s("time-zone", &o.time_zone)
        .v("cert", path_ref(&o.cert, "cm cert"))
        .v("key", path_ref(&o.key, "cm key"))
        .s("description", &o.description)
        .s("comment", &o.comment)
        .s("contact", &o.contact)
        .s("location", &o.location)
        .s("mirror-ip", &o.mirror_ip)
        .s("mirror-secondary-ip", &o.mirror_secondary_ip)
        .s("multicast-interface", &o.multicast_interface)
        .s("multicast-ip", &o.multicast_ip)
        .s("multicast-port", &o.multicast_port)
        .v("unicast-address", str_list(&o.unicast_address))
        .done()
}

pub(super) fn project_cm_device_group(o: &BigipCmDeviceGroup) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.type_)
        .v("devices", path_ref_list_strs(&o.devices, "cm device"))
        .s("auto-sync", &o.auto_sync)
        .s("network-failover", &o.network_failover)
        .s("hidden", &o.hidden)
        .s("description", &o.description)
        .s("save-on-auto-sync", &o.save_on_auto_sync)
        .s("full-load-on-sync", &o.full_load_on_sync)
        .s("asm-sync", &o.asm_sync)
        .s(
            "incremental-config-sync-size-max",
            &o.incremental_config_sync_size_max,
        )
        .done()
}

pub(super) fn project_cm_traffic_group(o: &BigipCmTrafficGroup) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("unit-id", &o.unit_id)
        .s("description", &o.description)
        .v("default-device", path_ref(&o.default_device, "cm device"))
        .s("ha-load-factor", &o.ha_load_factor)
        .v("ha-order", path_ref_list_strs(&o.ha_order, "cm device"))
        .v("ha-group", path_ref(&o.ha_group, "cm ha-group"))
        .s("auto-failback-enabled", &o.auto_failback_enabled)
        .s("auto-failback-time", &o.auto_failback_time)
        .s("mac", &o.mac)
        .done()
}

pub(super) fn project_cm_ha_group(o: &BigipCmHaGroup) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .s("enabled-state", &o.enabled_state)
        .s("active-bonus", &o.active_bonus)
        .v("pools", path_ref_list_strs(&o.pools, "ltm pool"))
        .v("trunks", str_list(&o.trunks))
        .done()
}

pub(super) fn project_cm_trust_domain(o: &BigipCmTrustDomain) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("ca-cert", path_ref(&o.ca_cert, "cm cert"))
        .s("ca-cert-bundle", &o.ca_cert_bundle)
        .v("ca-key", path_ref(&o.ca_key, "cm key"))
        .v("ca-devices", path_ref_list_strs(&o.ca_devices, "cm device"))
        .s("guid", &o.guid)
        .s("status", &o.status)
        .v("trust-group", path_ref(&o.trust_group, "cm device-group"))
        .done()
}
