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

//! Field projection of system objects: DNS, NTP, SNMP, provisioning, folders and certificate files.

use super::ltm::str_list;
use super::{Fields, path_ref};
use crate::value::Value;
use indexmap::IndexMap;
use tcl_bigip::model::{
    BigipSysDns, BigipSysFileSslCert, BigipSysFileSslKey, BigipSysFolder, BigipSysGlobalSettings,
    BigipSysManagementRoute, BigipSysNtp, BigipSysNtpRestrict, BigipSysProvision, BigipSysSnmp,
    BigipSysSnmpDiskMonitor, BigipSysSnmpProcessMonitor, BigipSysSnmpTrap, BigipSysSnmpUser,
};

// Sys projections

pub(super) fn project_sys_dns(o: &BigipSysDns) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("name-servers", str_list(&o.name_servers))
        .v("search", str_list(&o.search))
        .done()
}

pub(super) fn project_sys_ntp(o: &BigipSysNtp) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("servers", str_list(&o.servers))
        .s("timezone", &o.timezone)
        .v(
            "restrict",
            Value::List(o.restrict.iter().map(ntp_restrict_value).collect()),
        )
        .done()
}

fn ntp_restrict_value(r: &BigipSysNtpRestrict) -> Value {
    Value::Object(
        Fields::new()
            .s("name", &r.name)
            .s("address", &r.address)
            .s("mask", &r.mask)
            .s("default-entry", &r.default_entry)
            .v("flags", str_list(&r.flags))
            .s("description", &r.description)
            .done(),
    )
}

pub(super) fn project_sys_snmp(o: &BigipSysSnmp) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("agent-addresses", str_list(&o.agent_addresses))
        .v("communities", str_list(&o.communities))
        .s("sys-contact", &o.sys_contact)
        .s("sys-location", &o.sys_location)
        .s("sys-services", &o.sys_services)
        .s("trap-community", &o.trap_community)
        .v(
            "users",
            Value::List(o.users.iter().map(snmp_user_value).collect()),
        )
        .v(
            "traps",
            Value::List(o.traps.iter().map(snmp_trap_value).collect()),
        )
        .v(
            "process-monitors",
            Value::List(
                o.process_monitors
                    .iter()
                    .map(snmp_process_monitor_value)
                    .collect(),
            ),
        )
        .v(
            "disk-monitors",
            Value::List(
                o.disk_monitors
                    .iter()
                    .map(snmp_disk_monitor_value)
                    .collect(),
            ),
        )
        .done()
}

fn snmp_user_value(u: &BigipSysSnmpUser) -> Value {
    Value::Object(
        Fields::new()
            .s("name", &u.name)
            .s("username", &u.username)
            .s("security-level", &u.security_level)
            .s("auth-protocol", &u.auth_protocol)
            .s("privacy-protocol", &u.privacy_protocol)
            .s("oid-subset", &u.oid_subset)
            .s("description", &u.description)
            .done(),
    )
}

fn snmp_trap_value(t: &BigipSysSnmpTrap) -> Value {
    Value::Object(
        Fields::new()
            .s("name", &t.name)
            .s("host", &t.host)
            .s("port", &t.port)
            .s("version", &t.version)
            .s("community", &t.community)
            .s("security-name", &t.security_name)
            .s("security-level", &t.security_level)
            .s("auth-protocol", &t.auth_protocol)
            .s("privacy-protocol", &t.privacy_protocol)
            .s("network", &t.network)
            .s("description", &t.description)
            .done(),
    )
}

fn snmp_process_monitor_value(m: &BigipSysSnmpProcessMonitor) -> Value {
    Value::Object(
        Fields::new()
            .s("name", &m.name)
            .s("process", &m.process)
            .s("max-processes", &m.max_processes)
            .s("min-processes", &m.min_processes)
            .s("description", &m.description)
            .done(),
    )
}

fn snmp_disk_monitor_value(m: &BigipSysSnmpDiskMonitor) -> Value {
    Value::Object(
        Fields::new()
            .s("name", &m.name)
            .s("partition", &m.partition)
            .s("min-space", &m.min_space)
            .s("description", &m.description)
            .done(),
    )
}

pub(super) fn project_sys_global_settings(o: &BigipSysGlobalSettings) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("hostname", &o.hostname)
        .s("gui-setup", &o.gui_setup)
        .s("mgmt-dhcp", &o.mgmt_dhcp)
        .done()
}

pub(super) fn project_sys_provision(o: &BigipSysProvision) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("level", &o.level)
        .s("cpu-ratio", &o.cpu_ratio)
        .s("memory-ratio", &o.memory_ratio)
        .s("disk-ratio", &o.disk_ratio)
        .done()
}

pub(super) fn project_sys_folder(o: &BigipSysFolder) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("device-group", path_ref(&o.device_group, "cm device-group"))
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .s("hidden", &o.hidden)
        .s("description", &o.description)
        .s("inherited-device-group", &o.inherited_device_group)
        .s("inherited-traffic-group", &o.inherited_traffic_group)
        .done()
}

/// `sys file ssl-cert` — the filestore cert record.
///
/// The x509 metadata fields (`subject` / `issuer` / `fingerprint` /
/// `expiration-string` / `key-type` / …) keep their TMSH spelling because
/// `x509_from_config` reads them by that name, and `cache-path` /
/// `source-path` are what `ucs_cert` uses to find the PEM inside a UCS.
pub(super) fn project_sys_file_ssl_cert(o: &BigipSysFileSslCert) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("source-path", &o.source_path)
        .s("cache-path", &o.cache_path)
        .s("revision", &o.revision)
        .s("description", &o.description)
        .s("issuer", &o.issuer)
        .s("subject", &o.subject)
        .s("subject-alternative-name", &o.subject_alternative_name)
        .s("expiration-string", &o.expiration_string)
        .s("expiration-date", &o.expiration_date)
        .s("fingerprint", &o.fingerprint)
        .s("serial-number", &o.serial_number)
        .s("version", &o.version)
        .s("key-size", &o.key_size)
        .s("key-type", &o.key_type)
        .s("certificate-key-size", &o.certificate_key_size)
        .s("is-bundle", &o.is_bundle)
        .v("issuer-cert", path_ref(&o.issuer_cert, "sys file ssl-cert"))
        .v("bundle-certificates", str_list(&o.bundle_certificates))
        .v(
            "cert-validation-options",
            str_list(&o.cert_validation_options),
        )
        .v("cert-validators", str_list(&o.cert_validators))
        .s("checksum", &o.checksum)
        .s("mode", &o.mode)
        .s("size", &o.size)
        .s("create-time", &o.create_time)
        .s("created-by", &o.created_by)
        .s("last-update-time", &o.last_update_time)
        .s("updated-by", &o.updated_by)
        .done()
}

pub(super) fn project_sys_file_ssl_key(o: &BigipSysFileSslKey) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("source-path", &o.source_path)
        .s("cache-path", &o.cache_path)
        .s("revision", &o.revision)
        .s("passphrase", &o.passphrase)
        .s("description", &o.description)
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

pub(super) fn project_sys_management_route(o: &BigipSysManagementRoute) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("network", &o.network)
        .s("gateway", &o.gateway)
        .s("mtu", &o.mtu)
        .s("description", &o.description)
        .done()
}
