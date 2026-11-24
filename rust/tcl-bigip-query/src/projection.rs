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

//! Lazy projection over a parsed `BigipConfig`.
//!
//! Turns a [`Root`] backed by a [`BigipConfig`] into a navigable tree of
//! [`Container`]s. The synthetic `<root>` container holds one child per
//! module (`ltm`, …); each module container holds one child per kind
//! (`ltm virtual`, `ltm pool`, …); each kind container projects its
//! objects into [`ObjectRef`]s. Everything is lazy: a kind's objects only
//! materialise when navigated into, and the resulting refs are memoised on
//! `Root.object_cache` keyed by `(kind, full_path)`.
//!
//! Seven modules carry a projection — `ltm`, `net`, `sys`, `cm`, `gtm`,
//! `apm`, `security` — and their `(label, tmsh_kind)` tables below are the
//! contract for which kinds are navigable: a kind absent from a table is
//! absent from the DSL, and the module container reports `no entry`. The
//! remaining modules in `MODULE_NAMES` appear at the root with no kinds.
//!
//! Each object's top-level scalar properties get a `field_slot` (the byte
//! range of the value half) so the edit-plan engine can rewrite a single
//! property in place; pool members get their slots from
//! `BigipPoolMember.field_offsets`. `stanza_slot` is populated from each
//! object's range so `--scf` / auto output matches the canonical layout.
//! The synthesised `ltm rule .refs` sub-object is built by
//! `rule_refs_value`.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use tcl_bigip::model::{DataGroupType, ModelObject, ProfileType};
use tcl_bigip::object_names::{ObjectNamePolicy, object_name_candidates};
use tcl_bigip::parser::Placed;
use tcl_bigip::value::{BigipList, ListItemValue, MonitorExpression};

use crate::errors::QueryError;
use crate::eval::Root;
use crate::value::{ObjectRef, PathRef, Value};

mod apm;
mod cm;
mod gtm;
mod kinds;
mod ltm;
mod net;
mod object_refs;
mod security;
mod sys;

use apm::{
    project_apm_access_policy, project_apm_customization_source, project_apm_default_report,
    project_apm_oauth_db_instance, project_apm_policy_agent, project_apm_policy_item,
    project_apm_ssh_security_config,
};
use cm::{
    project_cm_cert, project_cm_device, project_cm_device_group, project_cm_ha_group,
    project_cm_key, project_cm_traffic_group, project_cm_trust_domain,
};
use gtm::{
    project_gtm_datacenter, project_gtm_listener, project_gtm_pool, project_gtm_server,
    project_gtm_wideip,
};
#[cfg(test)]
use kinds::KINDS;
#[cfg(test)]
pub(crate) use kinds::documented_kind_labels;
use kinds::{MODULE_NAMES, is_object_kind_alias, kind_to_label, module_kinds};
use ltm::{
    project_data_group, project_monitor, project_node, project_persistence, project_policy,
    project_pool, project_profile, project_rule, project_snat_translation, project_snatpool,
    project_virtual, project_virtual_address,
};
use net::{
    project_net_dns_resolver, project_net_interface, project_net_port_list, project_net_route,
    project_net_route_domain, project_net_self, project_net_stp, project_net_tunnel,
    project_net_vlan,
};
use object_refs::build_object_ref;
use security::{
    project_fw_address_list, project_fw_policy, project_fw_port_list, project_fw_rule_list,
    project_nat_destination_translation, project_nat_policy, project_nat_source_translation,
};
use sys::{
    project_sys_dns, project_sys_file_ssl_cert, project_sys_file_ssl_key, project_sys_folder,
    project_sys_global_settings, project_sys_management_route, project_sys_ntp,
    project_sys_provision, project_sys_snmp,
};

// Container

/// A navigable namespace / kind container projected from a `BigipConfig`
///
/// `kind` carries either a module name (`"ltm"`), the synthetic
/// `"<root>"`, or a full TMSH module+type (`"ltm virtual"`). Entries are
/// built lazily on first access and cached.
pub struct Container {
    pub kind: String,
    pub root: Rc<Root>,
    entries: RefCell<Option<IndexMap<String, Value>>>,
}

impl std::fmt::Debug for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl Container {
    fn new(kind: impl Into<String>, root: Rc<Root>) -> Rc<Self> {
        Rc::new(Container {
            kind: kind.into(),
            root,
            entries: RefCell::new(None),
        })
    }

    /// The lazily-built entry map. Keys are the user-visible identifiers —
    /// full-paths for object kinds, plain TMSH type names for module
    /// namespaces.
    #[must_use]
    pub fn entries(&self) -> IndexMap<String, Value> {
        if self.entries.borrow().is_none() {
            let built = build_entries(self);
            *self.entries.borrow_mut() = Some(built);
        }
        self.entries.borrow().clone().unwrap_or_default()
    }

    /// Look up *key* in this container's entries, with partition shorthand
    /// (`<name>` -> `/Common/<name>`).
    ///
    /// # Errors
    /// Returns an eval error for a missing or ambiguous entry.
    pub fn lookup(&self, key: &str) -> Result<Value, QueryError> {
        let ents = self.entries();
        if let Some(v) = ents.get(key) {
            return Ok(v.clone());
        }
        if self.is_object_kind() && !key.starts_with('/') {
            let candidates = object_name_candidates(
                key,
                ents.keys().map(|path| (path.as_str(), path)),
                ObjectNamePolicy::CommonThenSuffix,
            );
            let matches = candidates.candidates();
            if let [path] = matches {
                return Ok(ents[*path].clone());
            }
            if matches.len() > 1 {
                return Err(QueryError::eval(format!(
                    "{}: name {} is ambiguous ({} matches; use a full path)",
                    self.kind,
                    crate::eval::pyr_pub(key),
                    matches.len()
                )));
            }
        }
        Err(QueryError::eval(format!(
            "{}: no entry {}",
            self.kind,
            crate::eval::pyr_pub(key)
        )))
    }

    /// Keys matching *pattern* (regex search).
    ///
    /// # Errors
    /// Returns an eval error if the pattern fails the shared regex guard.
    pub fn regex_keys(&self, pattern: &str) -> Result<Vec<String>, QueryError> {
        let rx = crate::builtins::safe_regex_compile(pattern, "regex subscript")
            .map_err(crate::eval::eval_from_builtin_pub)?;
        Ok(self
            .entries()
            .keys()
            .filter(|k| rx.is_match(k))
            .cloned()
            .collect())
    }

    fn is_object_kind(&self) -> bool {
        self.kind.contains(' ') || is_object_kind_alias(&self.kind)
    }
}

// Public entry point

/// Return the synthetic top-level container or external JSON.
#[must_use]
pub fn root_container(root: &Rc<Root>) -> Value {
    if let Some(v) = &root.json_value {
        return v.clone();
    }
    Value::Container(Container::new("<root>", Rc::clone(root)))
}

// Entry building

fn build_entries(container: &Container) -> IndexMap<String, Value> {
    let root = &container.root;

    // Synthetic `<root>`: one Container per module.
    if container.kind == "<root>" {
        let mut out = IndexMap::new();
        for module in MODULE_NAMES {
            out.insert(
                (*module).to_owned(),
                Value::Container(Container::new(*module, Rc::clone(root))),
            );
        }
        return out;
    }

    // Module-level container (`.ltm` / `.gtm`): one Container per covered kind.
    if MODULE_NAMES.contains(&container.kind.as_str()) {
        let mut out = IndexMap::new();
        for (label, tmsh_kind) in module_kinds(&container.kind) {
            out.insert(
                label.to_owned(),
                Value::Container(Container::new(tmsh_kind, Rc::clone(root))),
            );
        }
        return out;
    }

    // Leaf kind container: project each object of this kind.
    let kind = container.kind.as_str();
    if !is_object_kind_alias(kind) {
        return IndexMap::new();
    }
    let mut out = IndexMap::new();
    for placed in &root.config.objects {
        if placed_kind(placed) != Some(kind) {
            continue;
        }
        let ref_value = build_object_ref(kind, &placed.full_path, &placed.object, root);
        out.insert(placed.full_path.clone(), ref_value);
    }
    out
}

/// The tmsh kind of a placed object, restricted to the covered subset.
fn placed_kind(placed: &Placed) -> Option<&'static str> {
    match &placed.object {
        ModelObject::VirtualServer(_) => Some("ltm virtual"),
        ModelObject::VirtualAddress(_) => Some("ltm virtual-address"),
        ModelObject::Pool(_) => Some("ltm pool"),
        ModelObject::Node(_) => Some("ltm node"),
        ModelObject::Rule(_) => Some("ltm rule"),
        ModelObject::Profile(_) => Some("ltm profile"),
        ModelObject::Monitor(_) => Some("ltm monitor"),
        ModelObject::Persistence(_) => Some("ltm persistence"),
        ModelObject::SnatPool(_) => Some("ltm snatpool"),
        ModelObject::Policy(_) => Some("ltm policy"),
        ModelObject::DataGroup(_) => Some("ltm data-group"),
        ModelObject::LtmSnatTranslation(_) => Some("ltm snat-translation"),
        ModelObject::NetRoute(_) => Some("net route"),
        ModelObject::NetVlan(_) => Some("net vlan"),
        ModelObject::NetSelf(_) => Some("net self"),
        ModelObject::NetRouteDomain(_) => Some("net route-domain"),
        ModelObject::NetPortList(_) => Some("net port-list"),
        ModelObject::NetInterface(_) => Some("net interface"),
        ModelObject::NetDnsResolver(_) => Some("net dns-resolver"),
        ModelObject::NetTunnel(_) => Some("net tunnels tunnel"),
        ModelObject::NetStp(_) => Some("net stp"),
        ModelObject::SysDns(_) => Some("sys dns"),
        ModelObject::SysNtp(_) => Some("sys ntp"),
        ModelObject::SysSnmp(_) => Some("sys snmp"),
        ModelObject::SysGlobalSettings(_) => Some("sys global-settings"),
        ModelObject::SysProvision(_) => Some("sys provision"),
        ModelObject::SysFolder(_) => Some("sys folder"),
        ModelObject::SysFileSslCert(_) => Some("sys file ssl-cert"),
        ModelObject::SysFileSslKey(_) => Some("sys file ssl-key"),
        ModelObject::SysManagementRoute(_) => Some("sys management-route"),
        ModelObject::CmCert(_) => Some("cm cert"),
        ModelObject::CmKey(_) => Some("cm key"),
        ModelObject::CmDevice(_) => Some("cm device"),
        ModelObject::CmDeviceGroup(_) => Some("cm device-group"),
        ModelObject::CmTrafficGroup(_) => Some("cm traffic-group"),
        ModelObject::CmTrustDomain(_) => Some("cm trust-domain"),
        ModelObject::CmHaGroup(_) => Some("cm ha-group"),
        ModelObject::ApmPolicyAccessPolicy(_) => Some("apm policy access-policy"),
        ModelObject::ApmPolicyItem(_) => Some("apm policy policy-item"),
        ModelObject::ApmPolicyAgent(_) => Some("apm policy agent"),
        ModelObject::ApmPolicyCustomizationSource(_) => Some("apm policy customization-source"),
        ModelObject::ApmOauthDbInstance(_) => Some("apm oauth db-instance"),
        ModelObject::ApmEphemeralAuthSshSecurityConfig(_) => {
            Some("apm ephemeral-auth ssh-security-config")
        }
        ModelObject::ApmReportDefaultReport(_) => Some("apm report default-report"),
        ModelObject::GtmDatacenter(_) => Some("gtm datacenter"),
        ModelObject::GtmServer(_) => Some("gtm server"),
        ModelObject::GtmPool(_) => Some("gtm pool"),
        ModelObject::GtmWideip(_) => Some("gtm wideip"),
        ModelObject::GtmListener(_) => Some("gtm listener"),
        ModelObject::SecurityFirewallPolicy(_) => Some("security firewall policy"),
        ModelObject::SecurityFirewallRuleList(_) => Some("security firewall rule-list"),
        ModelObject::SecurityFirewallAddressList(_) => Some("security firewall address-list"),
        ModelObject::SecurityFirewallPortList(_) => Some("security firewall port-list"),
        ModelObject::SecurityNatPolicy(_) => Some("security nat policy"),
        ModelObject::SecurityNatSourceTranslation(_) => Some("security nat source-translation"),
        ModelObject::SecurityNatDestinationTranslation(_) => {
            Some("security nat destination-translation")
        }
        _ => None,
    }
}

// Per-kind field projection

fn project_fields(kind: &str, obj: &ModelObject, root: &Rc<Root>) -> IndexMap<String, Value> {
    match (kind, obj) {
        ("ltm virtual", ModelObject::VirtualServer(o)) => project_virtual(o, root),
        ("ltm virtual-address", ModelObject::VirtualAddress(o)) => project_virtual_address(o),
        ("ltm pool", ModelObject::Pool(o)) => project_pool(o, root),
        ("ltm node", ModelObject::Node(o)) => project_node(o, root),
        ("ltm rule", ModelObject::Rule(o)) => project_rule(o, root),
        ("ltm profile", ModelObject::Profile(o)) => project_profile(o, root),
        ("ltm monitor", ModelObject::Monitor(o)) => project_monitor(o, root),
        ("ltm persistence", ModelObject::Persistence(o)) => project_persistence(o, root),
        ("ltm snatpool", ModelObject::SnatPool(o)) => project_snatpool(o),
        ("ltm policy", ModelObject::Policy(o)) => project_policy(o, root),
        ("ltm data-group", ModelObject::DataGroup(o)) => project_data_group(o),
        ("ltm snat-translation", ModelObject::LtmSnatTranslation(o)) => project_snat_translation(o),
        ("net route", ModelObject::NetRoute(o)) => project_net_route(o),
        ("net vlan", ModelObject::NetVlan(o)) => project_net_vlan(o),
        ("net self", ModelObject::NetSelf(o)) => project_net_self(o),
        ("net route-domain", ModelObject::NetRouteDomain(o)) => project_net_route_domain(o),
        ("net port-list", ModelObject::NetPortList(o)) => project_net_port_list(o),
        ("net interface", ModelObject::NetInterface(o)) => project_net_interface(o),
        ("net dns-resolver", ModelObject::NetDnsResolver(o)) => project_net_dns_resolver(o),
        ("net tunnels tunnel", ModelObject::NetTunnel(o)) => project_net_tunnel(o),
        ("net stp", ModelObject::NetStp(o)) => project_net_stp(o),
        ("sys dns", ModelObject::SysDns(o)) => project_sys_dns(o),
        ("sys ntp", ModelObject::SysNtp(o)) => project_sys_ntp(o),
        ("sys snmp", ModelObject::SysSnmp(o)) => project_sys_snmp(o),
        ("sys global-settings", ModelObject::SysGlobalSettings(o)) => {
            project_sys_global_settings(o)
        }
        ("sys provision", ModelObject::SysProvision(o)) => project_sys_provision(o),
        ("sys folder", ModelObject::SysFolder(o)) => project_sys_folder(o),
        ("sys file ssl-cert", ModelObject::SysFileSslCert(o)) => project_sys_file_ssl_cert(o),
        ("sys file ssl-key", ModelObject::SysFileSslKey(o)) => project_sys_file_ssl_key(o),
        ("sys management-route", ModelObject::SysManagementRoute(o)) => {
            project_sys_management_route(o)
        }
        ("cm cert", ModelObject::CmCert(o)) => project_cm_cert(o),
        ("cm key", ModelObject::CmKey(o)) => project_cm_key(o),
        ("cm device", ModelObject::CmDevice(o)) => project_cm_device(o),
        ("cm device-group", ModelObject::CmDeviceGroup(o)) => project_cm_device_group(o),
        ("cm traffic-group", ModelObject::CmTrafficGroup(o)) => project_cm_traffic_group(o),
        ("cm trust-domain", ModelObject::CmTrustDomain(o)) => project_cm_trust_domain(o),
        ("cm ha-group", ModelObject::CmHaGroup(o)) => project_cm_ha_group(o),
        ("apm policy access-policy", ModelObject::ApmPolicyAccessPolicy(o)) => {
            project_apm_access_policy(o)
        }
        ("apm policy policy-item", ModelObject::ApmPolicyItem(o)) => project_apm_policy_item(o),
        ("apm policy agent", ModelObject::ApmPolicyAgent(o)) => project_apm_policy_agent(o),
        ("apm policy customization-source", ModelObject::ApmPolicyCustomizationSource(o)) => {
            project_apm_customization_source(o)
        }
        ("apm oauth db-instance", ModelObject::ApmOauthDbInstance(o)) => {
            project_apm_oauth_db_instance(o)
        }
        (
            "apm ephemeral-auth ssh-security-config",
            ModelObject::ApmEphemeralAuthSshSecurityConfig(o),
        ) => project_apm_ssh_security_config(o),
        ("apm report default-report", ModelObject::ApmReportDefaultReport(o)) => {
            project_apm_default_report(o)
        }
        ("gtm datacenter", ModelObject::GtmDatacenter(o)) => project_gtm_datacenter(o),
        ("gtm server", ModelObject::GtmServer(o)) => project_gtm_server(o),
        ("gtm pool", ModelObject::GtmPool(o)) => project_gtm_pool(o),
        ("gtm wideip", ModelObject::GtmWideip(o)) => project_gtm_wideip(o),
        ("gtm listener", ModelObject::GtmListener(o)) => project_gtm_listener(o),
        ("security firewall policy", ModelObject::SecurityFirewallPolicy(o)) => {
            project_fw_policy(o)
        }
        ("security firewall rule-list", ModelObject::SecurityFirewallRuleList(o)) => {
            project_fw_rule_list(o)
        }
        ("security firewall address-list", ModelObject::SecurityFirewallAddressList(o)) => {
            project_fw_address_list(o)
        }
        ("security firewall port-list", ModelObject::SecurityFirewallPortList(o)) => {
            project_fw_port_list(o)
        }
        ("security nat policy", ModelObject::SecurityNatPolicy(o)) => project_nat_policy(o),
        ("security nat source-translation", ModelObject::SecurityNatSourceTranslation(o)) => {
            project_nat_source_translation(o)
        }
        (
            "security nat destination-translation",
            ModelObject::SecurityNatDestinationTranslation(o),
        ) => project_nat_destination_translation(o),
        _ => IndexMap::new(),
    }
}

/// A small ordered field-map builder.
struct Fields(IndexMap<String, Value>);

impl Fields {
    fn new() -> Self {
        Fields(IndexMap::new())
    }
    fn s(mut self, key: &str, value: &str) -> Self {
        self.0.insert(key.to_owned(), Value::Str(value.to_owned()));
        self
    }
    fn b(mut self, key: &str, value: bool) -> Self {
        self.0.insert(key.to_owned(), Value::Bool(value));
        self
    }
    fn i(mut self, key: &str, value: i64) -> Self {
        self.0.insert(key.to_owned(), Value::Int(value));
        self
    }
    fn v(mut self, key: &str, value: Value) -> Self {
        self.0.insert(key.to_owned(), value);
        self
    }
    fn done(self) -> IndexMap<String, Value> {
        self.0
    }
}

/// `PathRef` value.
fn path_ref(full_path: &str, expected_kind: &str) -> Value {
    Value::PathRef(Rc::new(PathRef::new(full_path, expected_kind)))
}

/// A list of `PathRef`s over a structured `BigipList` whose items name
/// other objects, taking each item's path-ish string as the target.
fn path_ref_list(list: &BigipList, expected_kind: &str) -> Value {
    let mut out = Vec::with_capacity(list.items.len());
    for item in &list.items {
        let p = list_item_string(&item.value);
        out.push(path_ref(&p, expected_kind));
    }
    Value::List(out)
}

/// A list of `PathRef`s from a `Vec<String>` ref+list field.
fn path_ref_list_strs(paths: &[String], expected_kind: &str) -> Value {
    Value::List(paths.iter().map(|p| path_ref(p, expected_kind)).collect())
}

/// A `BigipList` projected as a list of its items' `str()` renderings
/// (matching the JSON output over a `ListSpec`-projected `BigipList`).
fn list_str_values(list: &BigipList) -> Value {
    Value::List(
        list.items
            .iter()
            .map(|item| Value::Str(list_item_display(&item.value)))
            .collect(),
    )
}

/// The string a `ListItemValue` projects to when read as a path: `Profile`
/// and `Persistence` items give their `path`; everything else falls back to
/// the full display rendering.
fn list_item_string(value: &ListItemValue) -> String {
    match value {
        ListItemValue::Str(s) => s.clone(),
        ListItemValue::Profile(p) => p.path.clone(),
        ListItemValue::Persistence(p) => p.path.clone(),
        other => list_item_display(other),
    }
}

/// Full `str()` of a `ListItemValue` (the structured spelling used in JSON
/// dumps of `profiles` / `persist`).
fn list_item_display(value: &ListItemValue) -> String {
    match value {
        // The variants that occur in the covered LTM kinds' projected
        // list fields (`profiles` / `persist` / `snatpool members`).
        ListItemValue::Str(s) => s.clone(),
        ListItemValue::Profile(p) => p.to_string(),
        ListItemValue::Persistence(p) => p.to_string(),
        ListItemValue::MonitorExpression(m) => m.to_string(),
        ListItemValue::DataGroupRecord(r) => r.to_string(),
        ListItemValue::CertKeyChain(c) => c.to_string(),
        ListItemValue::GtmRegionMember(g) => g.to_string(),
        ListItemValue::PoolMember(m) => m.name.clone(),
        // FirewallRule has no Display surface and never appears in a
        // covered LTM list field; fall back to its key-ish name.
        ListItemValue::FirewallRule(_) => String::new(),
    }
}

/// Project a typed value to its canonical string (the `typed=True`
/// branch: string coercion of `raw`, or `""`).
fn typed_str<T: std::fmt::Display>(opt: Option<&T>) -> Value {
    Value::Str(opt.map_or_else(String::new, ToString::to_string))
}

/// Project an LTM monitor expression with its original spelling.
fn monitor_value(raw: &str) -> Value {
    if raw.trim().is_empty() {
        return Value::Str(String::new());
    }
    match MonitorExpression::try_parse(raw) {
        Some(expr) => Value::Str(expr.to_string()),
        None => Value::Str(raw.to_owned()),
    }
}

fn profile_type_str(t: ProfileType) -> Value {
    let name = match t {
        ProfileType::Aimcp => "AIMCP",
        ProfileType::Http => "HTTP",
        ProfileType::Tcp => "TCP",
        ProfileType::Udp => "UDP",
        ProfileType::ClientSsl => "CLIENT_SSL",
        ProfileType::ServerSsl => "SERVER_SSL",
        ProfileType::Ftp => "FTP",
        ProfileType::Dns => "DNS",
        ProfileType::Sip => "SIP",
        ProfileType::Diameter => "DIAMETER",
        ProfileType::Fix => "FIX",
        ProfileType::Radius => "RADIUS",
        ProfileType::Mqtt => "MQTT",
        ProfileType::Websocket => "WEBSOCKET",
        ProfileType::Stream => "STREAM",
        ProfileType::Sse => "SSE",
        ProfileType::Html => "HTML",
        ProfileType::Json => "JSON",
        ProfileType::Rewrite => "REWRITE",
        ProfileType::Fasthttp => "FASTHTTP",
        ProfileType::Fastl4 => "FASTL4",
        ProfileType::OneConnect => "ONE_CONNECT",
        ProfileType::Persistence => "PERSISTENCE",
        ProfileType::Other => "OTHER",
    };
    Value::Str(format!("ProfileType.{name}"))
}

fn data_group_kind_str(k: DataGroupType) -> Value {
    let name = match k {
        DataGroupType::Internal => "INTERNAL",
        DataGroupType::External => "EXTERNAL",
    };
    Value::Str(format!("DataGroupType.{name}"))
}

// PathRef resolution

/// Resolve the `ObjectRef` a `PathRef` points to. Forces the relevant kind's
/// container so
/// the object cache populates, then looks up `(expected_kind, full_path)`.
#[must_use]
pub fn resolve_pathref(reference: &PathRef, root: &Rc<Root>) -> Option<Rc<ObjectRef>> {
    if reference.full_path.is_empty() {
        return None;
    }
    // Fast path: cache hit on the recorded expected kind.
    if !reference.expected_kind.is_empty()
        && let Some(cached) = root
            .object_cache
            .borrow()
            .get(&(reference.expected_kind.clone(), reference.full_path.clone()))
    {
        return Some(Rc::clone(cached));
    }

    let Value::Container(container) = root_container(root) else {
        return None;
    };

    if !reference.expected_kind.is_empty() {
        // Targeted scope: build only the matching kind's container.
        let label = kind_to_label(&reference.expected_kind)?;
        let module = reference.expected_kind.split(' ').next().unwrap_or("");
        let module_container = container.lookup(module).ok()?;
        if let Value::Container(mc) = module_container
            && let Some(Value::Container(kc)) = mc.entries().get(label)
        {
            let _ = kc.entries();
        }
        return root
            .object_cache
            .borrow()
            .get(&(reference.expected_kind.clone(), reference.full_path.clone()))
            .map(Rc::clone);
    }

    // Fallback: no expected kind — build every covered kind, then look up
    // any cache entry sharing the full-path.
    for module in MODULE_NAMES {
        if let Ok(Value::Container(mc)) = container.lookup(module) {
            for (_label, value) in mc.entries() {
                if let Value::Container(kc) = value {
                    let _ = kc.entries();
                }
            }
        }
    }
    for ((_, path), cached) in root.object_cache.borrow().iter() {
        if path == &reference.full_path {
            return Some(Rc::clone(cached));
        }
    }
    None
}

#[cfg(test)]
mod tests;
