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
use tcl_bigip::model::BigipDataGroup;
use tcl_bigip::model::{
    BigipApmEphemeralAuthSshSecurityConfig, BigipApmOauthDbInstance, BigipApmPolicyAccessPolicy,
    BigipApmPolicyAgent, BigipApmPolicyCustomizationSource, BigipApmPolicyItem,
    BigipApmReportDefaultReport, BigipCmCert, BigipCmDevice, BigipCmDeviceGroup, BigipCmHaGroup,
    BigipCmKey, BigipCmTrafficGroup, BigipCmTrustDomain, BigipGtmDatacenter, BigipGtmListener,
    BigipGtmPool, BigipGtmPoolMember, BigipGtmServer, BigipGtmWideip, BigipLtmSnatTranslation,
    BigipMonitor, BigipNetDnsResolver, BigipNetInterface, BigipNetPortList, BigipNetRoute,
    BigipNetRouteDomain, BigipNetSelf, BigipNetStp, BigipNetTunnel, BigipNetVlan, BigipNode,
    BigipPersistence, BigipPolicy, BigipPolicyAction, BigipPolicyCondition, BigipPolicyRule,
    BigipPool, BigipPoolMember, BigipProfile, BigipRule, BigipSecurityFirewallAddressList,
    BigipSecurityFirewallPolicy, BigipSecurityFirewallPortList, BigipSecurityFirewallRuleList,
    BigipSecurityNatDestinationTranslation, BigipSecurityNatPolicy,
    BigipSecurityNatSourceTranslation, BigipSnatPool, BigipSysDns, BigipSysFileSslCert,
    BigipSysFileSslKey, BigipSysFolder, BigipSysGlobalSettings, BigipSysManagementRoute,
    BigipSysNtp, BigipSysNtpRestrict, BigipSysProvision, BigipSysSnmp, BigipSysSnmpDiskMonitor,
    BigipSysSnmpProcessMonitor, BigipSysSnmpTrap, BigipSysSnmpUser, BigipVirtualAddress,
    BigipVirtualServer, DataGroupType, ModelObject, ProfileType,
};
use tcl_bigip::parser::Placed;
use tcl_bigip::value::{BigipList, ListItemValue, MonitorExpression};
use tcl_bigip::value::{FirewallEndpoint, FirewallRule};

use crate::errors::QueryError;
use crate::eval::Root;
use crate::value::{FieldSlot, ObjectRef, PathRef, Value};

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
            let full = format!("/Common/{key}");
            let suffix = format!("/{key}");
            let matches: Vec<&String> = ents.keys().filter(|k| k.ends_with(&suffix)).collect();
            if let Some(v) = ents.get(&full) {
                return Ok(v.clone());
            }
            if matches.len() == 1 {
                return Ok(ents[matches[0]].clone());
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

// Module / kind tables

/// The module names exposed at `<root>`.
const MODULE_NAMES: &[&str] = &[
    "ltm",
    "net",
    "sys",
    "cm",
    "gtm",
    "apm",
    "security",
    "pem",
    "auth",
    "vcmp",
    "cli",
    "api-protection",
    "asm",
    "ilx",
    "wom",
    "analytics",
];

/// `(label, tmsh_kind)` for the LTM kinds the projection covers. The
/// long-tail LTM kinds the Rust model carries no typed struct for are
/// simply absent, so navigating into them reports `no entry`.
const LTM_KINDS: &[(&str, &str)] = &[
    ("virtual", "ltm virtual"),
    ("virtual-address", "ltm virtual-address"),
    ("pool", "ltm pool"),
    ("node", "ltm node"),
    ("rule", "ltm rule"),
    ("profile", "ltm profile"),
    ("monitor", "ltm monitor"),
    ("persistence", "ltm persistence"),
    ("snatpool", "ltm snatpool"),
    ("policy", "ltm policy"),
    ("data-group", "ltm data-group"),
    ("snat-translation", "ltm snat-translation"),
];

/// `(label, tmsh_kind)` for the GTM kinds the projection covers. GTM matters to
/// the estate report because a GTM (DNS) tier fronts one or more LTM tiers: a
/// `gtm server`'s `virtual-servers` destinations are the downstream LTM virtual
/// addresses, which is how the report links a GTM to the LTMs it load-balances.
const GTM_KINDS: &[(&str, &str)] = &[
    ("datacenter", "gtm datacenter"),
    ("server", "gtm server"),
    ("pool", "gtm pool"),
    ("wideip", "gtm wideip"),
    ("listener", "gtm listener"),
];

/// `(label, tmsh_kind)` for the AFM `security` kinds the projection covers:
/// firewall policies / rule-lists and the address-/port-lists they reference,
/// plus the NAT policies and source/destination translations. These let the
/// report surface the firewall + NAT posture alongside the LTM/GTM estate.
const SECURITY_KINDS: &[(&str, &str)] = &[
    ("firewall-policy", "security firewall policy"),
    ("firewall-rule-list", "security firewall rule-list"),
    ("firewall-address-list", "security firewall address-list"),
    ("firewall-port-list", "security firewall port-list"),
    ("nat-policy", "security nat policy"),
    ("nat-source-translation", "security nat source-translation"),
    (
        "nat-destination-translation",
        "security nat destination-translation",
    ),
];

/// `(label, tmsh_kind)` for the `net` kinds the projection covers — the L2/L3
/// underlay the LTM tier sits on. `net self` / `net vlan` / `net route-domain`
/// are what a self-IP or VLAN-binding audit walks, and `virtual.vlans[]` /
/// `self.vlan` path-refs deref into `net vlan` so `.net.self[].vlan.tag`
/// resolves the whole chain.
const NET_KINDS: &[(&str, &str)] = &[
    ("route", "net route"),
    ("vlan", "net vlan"),
    ("self", "net self"),
    ("route-domain", "net route-domain"),
    ("port-list", "net port-list"),
    ("interface", "net interface"),
    ("dns-resolver", "net dns-resolver"),
    ("tunnel", "net tunnels tunnel"),
    ("stp", "net stp"),
];

/// `(label, tmsh_kind)` for the `sys` kinds the projection covers. The
/// filestore kinds (`file-ssl-cert` / `file-ssl-key`) carry the cert metadata
/// `x509_from_config` / `ucs_cert` project, and are the entry point for every
/// cert-expiry audit. `dns` / `ntp` / `snmp` / `global-settings` are TMSH
/// singletons: they parse with an empty full-path, so they hold exactly one
/// entry each and are read by streaming (`.sys.dns[]`).
const SYS_KINDS: &[(&str, &str)] = &[
    ("dns", "sys dns"),
    ("ntp", "sys ntp"),
    ("snmp", "sys snmp"),
    ("global-settings", "sys global-settings"),
    ("provision", "sys provision"),
    ("folder", "sys folder"),
    ("file-ssl-cert", "sys file ssl-cert"),
    ("file-ssl-key", "sys file ssl-key"),
    ("management-route", "sys management-route"),
];

/// `(label, tmsh_kind)` for the `cm` (device-cluster) kinds. `cm device` +
/// `cm device-group` are the HA topology an estate report joins on; `cm cert`
/// / `cm key` are the device-trust key material, carrying the same cert
/// metadata fields as `sys file ssl-cert`.
const CM_KINDS: &[(&str, &str)] = &[
    ("cert", "cm cert"),
    ("key", "cm key"),
    ("device", "cm device"),
    ("device-group", "cm device-group"),
    ("traffic-group", "cm traffic-group"),
    ("trust-domain", "cm trust-domain"),
    ("ha-group", "cm ha-group"),
];

/// `(label, tmsh_kind)` for the APM kinds. An `apm policy access-policy`'s
/// `start-item` / `items[]` deref into `apm policy policy-item`, and an item's
/// `agents[]` into `apm policy agent`, so a policy walk
/// (`.apm["access-policy"][].items[].caption`) resolves in one chain.
const APM_KINDS: &[(&str, &str)] = &[
    ("access-policy", "apm policy access-policy"),
    ("policy-item", "apm policy policy-item"),
    ("policy-agent", "apm policy agent"),
    ("customization-source", "apm policy customization-source"),
    ("oauth-db-instance", "apm oauth db-instance"),
    (
        "ssh-security-config",
        "apm ephemeral-auth ssh-security-config",
    ),
    ("default-report", "apm report default-report"),
];

/// Every covered kind table, in module order. Iterated by the per-module entry
/// builder and the kind/label lookups.
const KIND_TABLES: &[&[(&str, &str)]] = &[
    LTM_KINDS,
    NET_KINDS,
    SYS_KINDS,
    CM_KINDS,
    GTM_KINDS,
    APM_KINDS,
    SECURITY_KINDS,
];

/// The `(label, tmsh_kind)` table for a module, or empty for an uncovered one.
fn module_kinds(module: &str) -> &'static [(&'static str, &'static str)] {
    match module {
        "ltm" => LTM_KINDS,
        "net" => NET_KINDS,
        "sys" => SYS_KINDS,
        "cm" => CM_KINDS,
        "gtm" => GTM_KINDS,
        "apm" => APM_KINDS,
        "security" => SECURITY_KINDS,
        _ => &[],
    }
}

/// The set of leaf object kinds, restricted to the covered subset. Used by
/// `Container.is_object_kind`.
fn is_object_kind_alias(kind: &str) -> bool {
    KIND_TABLES
        .iter()
        .any(|table| table.iter().any(|(_, k)| *k == kind))
}

/// Map a kind to its label (for `PathRef` container navigation).
fn kind_to_label(kind: &str) -> Option<&'static str> {
    KIND_TABLES
        .iter()
        .flat_map(|table| table.iter())
        .find(|(_, k)| *k == kind)
        .map(|(label, _)| *label)
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
                (*label).to_owned(),
                Value::Container(Container::new(*tmsh_kind, Rc::clone(root))),
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

// ObjectRef building

fn build_object_ref(kind: &str, full_path: &str, obj: &ModelObject, root: &Rc<Root>) -> Value {
    let cache_key = (kind.to_owned(), full_path.to_owned());
    if let Some(cached) = root.object_cache.borrow().get(&cache_key) {
        return Value::ObjectRef(Rc::clone(cached));
    }

    let fields = project_fields(kind, obj, root);
    let stanza_slot = stanza_slot_for(obj, root);
    let field_slots = collect_field_slots(obj, &fields, root);

    let object_ref = Rc::new(ObjectRef {
        kind: kind.to_owned(),
        full_path: full_path.to_owned(),
        fields,
        field_slots,
        stanza_slot,
        config_uri: root.uri.clone(),
    });
    root.object_cache
        .borrow_mut()
        .insert(cache_key, Rc::clone(&object_ref));
    Value::ObjectRef(object_ref)
}

/// Build the whole-stanza slot from the object's range, extending back to
/// the header's line start.
fn stanza_slot_for(obj: &ModelObject, root: &Rc<Root>) -> Option<FieldSlot> {
    let rng = model_range(obj)?;
    let start = rng.start.offset as usize;
    let end = rng.end.offset as usize;
    let header_start = scan_back_to_line_start(&root.source, start);
    let raw_text = root.source.get(header_start..end)?.to_owned();
    Some(FieldSlot {
        source_uri: root.uri.clone(),
        start: header_start,
        end,
        raw_text,
    })
}

fn scan_back_to_line_start(source: &str, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = offset.min(bytes.len());
    while i > 0 && bytes[i - 1] != b'\n' {
        i -= 1;
    }
    i
}

fn model_range(obj: &ModelObject) -> Option<tcl_bigip::range::Range> {
    match obj {
        ModelObject::VirtualServer(o) => o.range,
        ModelObject::VirtualAddress(o) => o.range,
        ModelObject::Pool(o) => o.range,
        ModelObject::Node(o) => o.range,
        ModelObject::Rule(o) => o.range,
        ModelObject::Profile(o) => o.range,
        ModelObject::Monitor(o) => o.range,
        ModelObject::Persistence(o) => o.range,
        ModelObject::SnatPool(o) => o.range,
        ModelObject::Policy(o) => o.range,
        ModelObject::DataGroup(o) => o.range,
        ModelObject::LtmSnatTranslation(o) => o.range,
        ModelObject::NetRoute(o) => o.range,
        ModelObject::NetVlan(o) => o.range,
        ModelObject::NetSelf(o) => o.range,
        ModelObject::NetRouteDomain(o) => o.range,
        ModelObject::NetPortList(o) => o.range,
        ModelObject::NetInterface(o) => o.range,
        ModelObject::NetDnsResolver(o) => o.range,
        ModelObject::NetTunnel(o) => o.range,
        ModelObject::NetStp(o) => o.range,
        ModelObject::SysDns(o) => o.range,
        ModelObject::SysNtp(o) => o.range,
        ModelObject::SysSnmp(o) => o.range,
        ModelObject::SysGlobalSettings(o) => o.range,
        ModelObject::SysProvision(o) => o.range,
        ModelObject::SysFolder(o) => o.range,
        ModelObject::SysFileSslCert(o) => o.range,
        ModelObject::SysFileSslKey(o) => o.range,
        ModelObject::SysManagementRoute(o) => o.range,
        ModelObject::CmCert(o) => o.range,
        ModelObject::CmKey(o) => o.range,
        ModelObject::CmDevice(o) => o.range,
        ModelObject::CmDeviceGroup(o) => o.range,
        ModelObject::CmTrafficGroup(o) => o.range,
        ModelObject::CmTrustDomain(o) => o.range,
        ModelObject::CmHaGroup(o) => o.range,
        ModelObject::ApmPolicyAccessPolicy(o) => o.range,
        ModelObject::ApmPolicyItem(o) => o.range,
        ModelObject::ApmPolicyAgent(o) => o.range,
        ModelObject::ApmPolicyCustomizationSource(o) => o.range,
        ModelObject::ApmOauthDbInstance(o) => o.range,
        ModelObject::ApmEphemeralAuthSshSecurityConfig(o) => o.range,
        ModelObject::ApmReportDefaultReport(o) => o.range,
        ModelObject::GtmDatacenter(o) => o.range,
        ModelObject::GtmServer(o) => o.range,
        ModelObject::GtmPool(o) => o.range,
        ModelObject::GtmWideip(o) => o.range,
        ModelObject::GtmListener(o) => o.range,
        ModelObject::SecurityFirewallPolicy(o) => o.range,
        ModelObject::SecurityFirewallRuleList(o) => o.range,
        ModelObject::SecurityFirewallAddressList(o) => o.range,
        ModelObject::SecurityFirewallPortList(o) => o.range,
        ModelObject::SecurityNatPolicy(o) => o.range,
        ModelObject::SecurityNatSourceTranslation(o) => o.range,
        ModelObject::SecurityNatDestinationTranslation(o) => o.range,
        _ => None,
    }
}

// Field-slot byte-range discovery

/// Locate each top-level scalar property's value span inside its stanza.
///
/// Returns a map of TMSH field name → [`FieldSlot`] for every field that
/// appears as a single-line `key value` property in the source *and* is a
/// known projected field. Compound list / sub-block values and identity
/// fields (whose location is the header) are simply absent.
fn collect_field_slots(
    obj: &ModelObject,
    fields: &IndexMap<String, Value>,
    root: &Rc<Root>,
) -> IndexMap<String, FieldSlot> {
    let Some(rng) = model_range(obj) else {
        return IndexMap::new();
    };
    let body_start = rng.start.offset as usize;
    let body_end = rng.end.offset as usize;
    let Some(body_text) = root.source.get(body_start..body_end) else {
        return IndexMap::new();
    };
    let mut slots: IndexMap<String, FieldSlot> = IndexMap::new();
    for (key, value_start, value_end, value_text) in iter_top_level_scalar_slots(body_text) {
        if !fields.contains_key(&key) {
            continue;
        }
        slots.insert(
            key,
            FieldSlot {
                source_uri: root.uri.clone(),
                start: body_start + value_start,
                end: body_start + value_end,
                raw_text: value_text,
            },
        );
    }
    slots
}

/// Yield `(key, value_start, value_end, value_text)` for scalar lines at the
/// stanza's top brace-depth.
fn iter_top_level_scalar_slots(body: &str) -> Vec<(String, usize, usize, String)> {
    let target_depth = i32::from(body.trim_start().starts_with('{'));
    let mut depth = 0i32;
    let mut line_start = 0usize;
    let mut out = Vec::new();
    for line in split_keep_ends(body) {
        let line_end = line_start + line.len();
        let stripped = line.trim();
        if !stripped.is_empty()
            && !stripped.starts_with('#')
            && depth == target_depth
            && let Some(parsed) = parse_scalar_slot_line(line, line_start)
        {
            out.push(parsed);
        }
        depth = brace_depth_after_line(line, depth);
        line_start = line_end;
    }
    out
}

/// Split *body* into lines keeping the trailing `\n` — splits on the
/// `\n`-only line endings SCF uses, keeping the newline on each line.
fn split_keep_ends(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let bytes = body.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            out.push(&body[start..=i]);
            start = i + 1;
        }
    }
    if start < body.len() {
        out.push(&body[start..]);
    }
    out
}

/// Parse one top-level scalar property line.
fn parse_scalar_slot_line(line: &str, line_start: usize) -> Option<(String, usize, usize, String)> {
    let content = line.strip_suffix('\n').unwrap_or(line);
    let bytes = content.as_bytes();
    let mut pos = 0usize;
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }
    if pos >= bytes.len() || matches!(bytes[pos], b'{' | b'}' | b'#') {
        return None;
    }
    let key_start = pos;
    while pos < bytes.len() && !matches!(bytes[pos], b' ' | b'\t' | b'{' | b'}') {
        pos += 1;
    }
    let key = &content[key_start..pos];
    if key.is_empty() {
        return None;
    }
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }
    if pos >= bytes.len() {
        return None;
    }
    let value = content[pos..].trim_end_matches([' ', '\t']);
    if value.is_empty() || value.contains('{') || value.contains('}') {
        return None;
    }
    let value_start = line_start + pos;
    let value_end = value_start + value.len();
    Some((key.to_owned(), value_start, value_end, value.to_owned()))
}

/// Update brace depth for one line, ignoring quoted strings.
fn brace_depth_after_line(line: &str, depth: i32) -> i32 {
    let mut depth = depth;
    let mut in_quote = false;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_quote {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = (depth - 1).max(0);
        }
    }
    depth
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

/// The string a `ListItemValue` projects to when read as a path (iterating
/// `item.value`, where `PathRef(full_path=p)` coerces via string conversion).
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

/// Project a `monitor` field: parse the raw string as a
/// [`MonitorExpression`] and re-render it, so `min 1 of { a b }` and its
/// spacing variants normalise to one spelling. Empty / unparseable
/// strings pass through verbatim.
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

fn project_virtual(o: &BigipVirtualServer, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("destination", typed_str(o.destination.as_ref()))
        .v("pool", path_ref(&o.pool, "ltm pool"))
        // `rules` / `policies` / `vlans` name other objects, so they
        // project as PathRefs and deref on field access.
        .v("rules", path_ref_list(&o.rules, "ltm rule"))
        // `profiles` / `persist` carry per-item context (a context tag, a
        // sub-block), so they project as their rendered spelling instead.
        .v("profiles", list_str_values(&o.profiles))
        .v("persist", list_str_values(&o.persist))
        .v("policies", path_ref_list(&o.policies, "ltm policy"))
        .v("snatpool", path_ref(&o.snatpool, "ltm snatpool"))
        .s("source-address-translation", &o.source_address_translation)
        .s("description", &o.description)
        .s("mask", &o.mask)
        .s("source", &o.source)
        .s("ip-protocol", &o.ip_protocol)
        .s("connection-limit", &o.connection_limit)
        .s("rate-limit", &o.rate_limit)
        .s("rate-limit-mode", &o.rate_limit_mode)
        .s("rate-limit-dst-mask", &o.rate_limit_dst_mask)
        .s("rate-limit-src-mask", &o.rate_limit_src_mask)
        .s("auto-lasthop", &o.auto_lasthop)
        .s("translate-address", &o.translate_address)
        .s("translate-port", &o.translate_port)
        .s("state", &o.state)
        .s("address-status", &o.address_status)
        .s("auto-discovery", &o.auto_discovery)
        .s("cmp-enabled", &o.cmp_enabled)
        .s("eviction-protected", &o.eviction_protected)
        .b("dhcp-relay", o.dhcp_relay)
        .b("internal", o.internal)
        .b("ip-forward", o.ip_forward)
        .b("l2-forward", o.l2_forward)
        .b("reject", o.reject)
        .s("nat64", &o.nat64)
        .s("gtm-score", &o.gtm_score)
        .s("mirror", &o.mirror)
        .s(
            "service-down-immediate-action",
            &o.service_down_immediate_action,
        )
        .s("source-port", &o.source_port)
        .s("serverssl-use-sni", &o.serverssl_use_sni)
        .v("rate-class", path_ref(&o.rate_class, "ltm rate-class"))
        .v(
            "per-flow-request-access-policy",
            path_ref(
                &o.per_flow_request_access_policy,
                "apm policy access-policy",
            ),
        )
        .v(
            "transparent-nexthop",
            path_ref(&o.transparent_nexthop, "net vlan"),
        )
        .v("vlans", path_ref_list(&o.vlans, "net vlan"))
        .b("vlans-disabled", o.vlans_disabled)
        .b("vlans-enabled", o.vlans_enabled)
        .v(
            "fallback-persistence",
            path_ref(&o.fallback_persistence, "ltm persistence"),
        )
        .v("last-hop-pool", path_ref(&o.last_hop_pool, "ltm pool"))
        .s("fw-enforced-policy", &o.fw_enforced_policy)
        .s("fw-staged-policy", &o.fw_staged_policy)
        .s("flow-eviction-policy", &o.flow_eviction_policy)
        .s("service-policy", &o.service_policy)
        .v("auth", path_ref_list_strs(&o.auth_profiles, "ltm profile"))
        .v("traffic-classes", str_list(&o.traffic_classes))
        .v(
            "clone-pools",
            path_ref_list_strs(&o.clone_pools, "ltm pool"),
        )
        .done()
}

fn project_virtual_address(o: &BigipVirtualAddress) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("address", typed_str(o.address.as_ref()))
        .s("mask", &o.mask)
        .s("arp", &o.arp)
        .s("icmp-echo", &o.icmp_echo)
        .s("auto-delete", &o.auto_delete)
        .s("connection-limit", &o.connection_limit)
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .s("inherited-traffic-group", &o.inherited_traffic_group)
        .s("route-advertisement", &o.route_advertisement)
        .s("server-scope", &o.server_scope)
        .s("spanning", &o.spanning)
        .s("unit", &o.unit)
        .s("description", &o.description)
        .s("state", &o.state)
        .s("floating", &o.floating)
        .s("traffic-group-restored", &o.traffic_group_restored)
        .done()
}

fn project_pool(o: &BigipPool, root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("module", &o.module)
        .v("monitor", monitor_value(&o.monitor))
        .s("load-balancing-mode", &o.load_balancing_mode)
        .v("members", project_pool_members(&o.members, root))
        .s("description", &o.description)
        .s("min-active-members", &o.min_active_members)
        .s("min-up-members", &o.min_up_members)
        .s("service-down-action", &o.service_down_action)
        .s("slow-ramp-time", &o.slow_ramp_time)
        .s("allow-snat", &o.allow_snat)
        .s("allow-nat", &o.allow_nat)
        .s("reselect-tries", &o.reselect_tries)
        .s("queue-depth-limit", &o.queue_depth_limit)
        .s("queue-time-limit", &o.queue_time_limit)
        .s("connection-limit", &o.connection_limit)
        .s("rate-limit", &o.rate_limit)
        .s("ratio", &o.ratio)
        .s("down-interval", &o.down_interval)
        .s("interval", &o.interval)
        .s("min-up-members-action", &o.min_up_members_action)
        .s("min-up-members-checking", &o.min_up_members_checking)
        .s("ip-tos-to-client", &o.ip_tos_to_client)
        .s("ip-tos-to-server", &o.ip_tos_to_server)
        .s("link-qos-to-client", &o.link_qos_to_client)
        .s("link-qos-to-server", &o.link_qos_to_server)
        .s("gateway-failsafe-device", &o.gateway_failsafe_device)
        .s("ignore-persisted-weight", &o.ignore_persisted_weight)
        .s("inherit-profile", &o.inherit_profile)
        .s("queue-on-connection-limit", &o.queue_on_connection_limit)
        .s("address-family", &o.address_family)
        .s("autopopulate", &o.autopopulate)
        .v("profiles", path_ref_list_strs(&o.profiles, "ltm profile"))
        .done()
}

fn project_pool_members(members: &BigipList, root: &Rc<Root>) -> Value {
    let mut out = Vec::new();
    for item in &members.items {
        if let ListItemValue::PoolMember(m) = &item.value {
            out.push(member_object_ref(m, root));
        }
    }
    Value::List(out)
}

fn member_object_ref(member: &BigipPoolMember, root: &Rc<Root>) -> Value {
    let fields = Fields::new()
        .s("name", &member.name)
        .v("address", typed_str(member.address.as_ref()))
        .i("port", member.port)
        .v(
            "monitor",
            if member.monitor.is_empty() {
                Value::Str(String::new())
            } else {
                path_ref(&member.monitor, "ltm monitor")
            },
        )
        .s("description", &member.description)
        .s("state", &member.state)
        .s("ratio", &member.ratio)
        .s("priority-group", &member.priority_group)
        .s("connection-limit", &member.connection_limit)
        .s("rate-limit", &member.rate_limit)
        .done();
    // Each captured field offset becomes a `FieldSlot` over the value span,
    // so member properties (`address`, `description`, …) are individually
    // editable even though the member is not a stanza of its own.
    let mut field_slots: IndexMap<String, FieldSlot> = IndexMap::new();
    for (key, (start, end)) in &member.field_offsets {
        if let Some(raw_text) = root.source.get(*start..*end) {
            field_slots.insert(
                key.clone(),
                FieldSlot {
                    source_uri: root.uri.clone(),
                    start: *start,
                    end: *end,
                    raw_text: raw_text.to_owned(),
                },
            );
        }
    }
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm pool-member".to_owned(),
        full_path: member.name.clone(),
        fields,
        field_slots,
        stanza_slot: None,
        config_uri: root.uri.clone(),
    }))
}

fn project_node(o: &BigipNode, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("address", typed_str(o.address.as_ref()))
        .s("description", &o.description)
        .v("monitor", monitor_value(&o.monitor))
        .s("state", &o.state)
        .s("connection-limit", &o.connection_limit)
        .s("rate-limit", &o.rate_limit)
        .s("ratio", &o.ratio)
        .v("fqdn", typed_str(o.fqdn.as_ref()))
        .done()
}

fn project_rule(o: &BigipRule, root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("body", &o.source)
        .s("description", &o.description)
        // `.refs` is the synthesised reference sub-object — the same
        // forward-walk (`max_depth=1`) `f5 grep` uses, classified into
        // pool / persistence / data-group buckets.
        .v("refs", rule_refs_value(o, root))
        .done()
}

/// The synthesised `ltm rule .refs` sub-object.
///
/// Each ref slot is a list of [`PathRef`]s drawn from the same reference
/// graph `f5 grep` walks (forward, `max_depth=1`), so the query DSL and the
/// grep verb always agree on what an iRule "uses". The graph is built once
/// per [`Root`] and memoised (see [`Root::graph`]).
fn rule_refs_value(o: &BigipRule, root: &Rc<Root>) -> Value {
    let (pools, persists, data_groups) = crate::builtins::extract_rule_refs(&o.full_path, root);
    let fields = Fields::new()
        .v("pools", path_ref_list_strs(&pools, "ltm pool"))
        .v("persists", path_ref_list_strs(&persists, "ltm persistence"))
        .v(
            "data-groups",
            path_ref_list_strs(&data_groups, "ltm data-group"),
        )
        .done();
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm rule-refs".to_owned(),
        full_path: o.full_path.clone(),
        fields,
        field_slots: IndexMap::new(),
        stanza_slot: None,
        config_uri: root.uri.clone(),
    }))
}

fn project_profile(o: &BigipProfile, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("type", profile_type_str(o.profile_type))
        .v("defaults-from", path_ref(&o.defaults_from, "ltm profile"))
        .s("description", &o.description)
        .s("idle-timeout", &o.idle_timeout)
        .s("insert-xforwarded-for", &o.insert_xforwarded_for)
        .s("request-chunking", &o.request_chunking)
        .s("response-chunking", &o.response_chunking)
        .s("lws-max-columns", &o.lws_max_columns)
        .s("lws-separator", &o.lws_separator)
        .s("server-agent-name", &o.server_agent_name)
        .s("via-request", &o.via_request)
        .s("via-response", &o.via_response)
        .s("ciphers", &o.ciphers)
        .v(
            "cipher-group",
            path_ref(&o.cipher_group, "ltm cipher group"),
        )
        .v("cert", path_ref(&o.cert, "sys file ssl-cert"))
        .v("key", path_ref(&o.key, "sys file ssl-key"))
        .v("chain", path_ref(&o.chain, "sys file ssl-cert"))
        .v("ca-file", path_ref(&o.ca_file, "sys file ssl-cert"))
        .s("crl-file", &o.crl_file)
        .s("cert-extension-includes", &o.cert_extension_includes)
        .s("options", &o.options)
        .s("peer-cert-mode", &o.peer_cert_mode)
        .s("sni-default", &o.sni_default)
        .s("sni-require", &o.sni_require)
        .s("server-name", &o.server_name)
        .s("renegotiation", &o.renegotiation)
        .s("secure-renegotiation", &o.secure_renegotiation)
        .v(
            "proxy-ca-cert",
            path_ref(&o.proxy_ca_cert, "sys file ssl-cert"),
        )
        .v(
            "proxy-ca-key",
            path_ref(&o.proxy_ca_key, "sys file ssl-key"),
        )
        .s("keep-alive-interval", &o.keep_alive_interval)
        .s("ip-tos-to-client", &o.ip_tos_to_client)
        .s("ip-tos-to-server", &o.ip_tos_to_server)
        .s("link-qos-to-client", &o.link_qos_to_client)
        .s("link-qos-to-server", &o.link_qos_to_server)
        .s("nagle", &o.nagle)
        .s("reset-on-timeout", &o.reset_on_timeout)
        .s("send-buffer-size", &o.send_buffer_size)
        .s("receive-window-size", &o.receive_window_size)
        .s("proxy-buffer-low", &o.proxy_buffer_low)
        .s("proxy-buffer-high", &o.proxy_buffer_high)
        .s("pva-acceleration", &o.pva_acceleration)
        .s("pva-dynamic-client-packets", &o.pva_dynamic_client_packets)
        .s("pva-dynamic-server-packets", &o.pva_dynamic_server_packets)
        .s("loose-close", &o.loose_close)
        .s("loose-initialization", &o.loose_initialization)
        .s("datagram-load-balancing", &o.datagram_load_balancing)
        .s("allow-no-payload", &o.allow_no_payload)
        .s("source-mask", &o.source_mask)
        .s("idle-timeout-override", &o.idle_timeout_override)
        .s("max-age", &o.max_age)
        .s("max-reuse", &o.max_reuse)
        .s("max-size", &o.max_size)
        .s("source", &o.source)
        .s("target", &o.target)
        .v("pool", path_ref(&o.pool, "ltm pool"))
        .s(
            "collected-stats-internal-logging",
            &o.collected_stats_internal_logging,
        )
        .s(
            "collected-stats-external-logging",
            &o.collected_stats_external_logging,
        )
        .v(
            "publisher",
            path_ref(&o.publisher, "sys log-config publisher"),
        )
        .s("maximum-bytes", &o.maximum_bytes)
        .s("maximum-entries", &o.maximum_entries)
        .s("maximum-non-json-bytes", &o.maximum_non_json_bytes)
        .s("max-buffered-msg-bytes", &o.max_buffered_msg_bytes)
        .s("max-field-name-size", &o.max_field_name_size)
        .done()
}

fn project_monitor(o: &BigipMonitor, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.monitor_type)
        .s("description", &o.description)
        .s("interval", &o.interval)
        .s("timeout", &o.timeout)
        .s("destination", &o.destination)
        .s("send", &o.send)
        .s("recv", &o.recv)
        .s("recv-disable", &o.recv_disable)
        .s("username", &o.username)
        .s("password", &o.password)
        .s("base", &o.base)
        .s("filter", &o.filter)
        .s("count", &o.count)
        .s("database", &o.database)
        .s("args", &o.args)
        .s("run", &o.run)
        .s("adaptive", &o.adaptive)
        .s("adaptive-divergence-type", &o.adaptive_divergence_type)
        .s("adaptive-divergence-value", &o.adaptive_divergence_value)
        .s("adaptive-limit", &o.adaptive_limit)
        .s("adaptive-sampling-timespan", &o.adaptive_sampling_timespan)
        .s("transparent", &o.transparent)
        .s("reverse", &o.reverse)
        .s("manual-resume", &o.manual_resume)
        .s("ignore-down-response", &o.ignore_down_response)
        .s("ip-dscp", &o.ip_dscp)
        .s("up-interval", &o.up_interval)
        .s("time-until-up", &o.time_until_up)
        .s("cipherlist", &o.cipherlist)
        .v("cert", path_ref(&o.cert, "sys file ssl-cert"))
        .v("key", path_ref(&o.key, "sys file ssl-key"))
        .s("compatibility", &o.compatibility)
        .s("community", &o.community)
        .s("version", &o.version)
        .s("agent-type", &o.agent_type)
        .s("cpu-coefficient", &o.cpu_coefficient)
        .s("cpu-threshold", &o.cpu_threshold)
        .s("disk-coefficient", &o.disk_coefficient)
        .s("disk-threshold", &o.disk_threshold)
        .s("memory-coefficient", &o.memory_coefficient)
        .s("memory-threshold", &o.memory_threshold)
        .s("headers", &o.headers)
        .s("request", &o.request)
        .s("response", &o.response)
        .s("mode", &o.mode)
        .s("alias-address", &o.alias_address)
        .s("alias-service-port", &o.alias_service_port)
        .v("defaults-from", path_ref(&o.defaults_from, "ltm monitor"))
        .done()
}

fn project_persistence(o: &BigipPersistence, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.persistence_type)
        .v(
            "defaults-from",
            path_ref(&o.defaults_from, "ltm persistence"),
        )
        .s("description", &o.description)
        .s("timeout", &o.timeout)
        .s("match-across-pools", &o.match_across_pools)
        .s("match-across-services", &o.match_across_services)
        .s("match-across-virtuals", &o.match_across_virtuals)
        .s("mirror", &o.mirror)
        .s("override-connection-limit", &o.override_connection_limit)
        .s("always-send", &o.always_send)
        .s("cookie-name", &o.cookie_name)
        .s("cookie-encryption", &o.cookie_encryption)
        .s(
            "cookie-encryption-passphrase",
            &o.cookie_encryption_passphrase,
        )
        .s("httponly", &o.httponly)
        .s("secure", &o.secure)
        .s("expiration", &o.expiration)
        .s("method", &o.method)
        .s("hash-length", &o.hash_length)
        .s("hash-offset", &o.hash_offset)
        .s("mcp-encryption-passphrase", &o.mcp_encryption_passphrase)
        .done()
}

fn project_snatpool(o: &BigipSnatPool) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("members", list_str_values(&o.members))
        .s("description", &o.description)
        .done()
}

fn project_policy(o: &BigipPolicy, root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("strategy", &o.strategy)
        .v("controls", str_list(&o.controls))
        .v("requires", str_list(&o.requires))
        .v(
            "rules",
            Value::List(o.rules.iter().map(|r| policy_rule_ref(r, root)).collect()),
        )
        .s("description", &o.description)
        .s("status", &o.status)
        .s("last-modified", &o.last_modified)
        .done()
}

fn policy_rule_ref(rule: &BigipPolicyRule, root: &Rc<Root>) -> Value {
    let fields = Fields::new()
        .s("name", &rule.name)
        .i("ordinal", rule.ordinal)
        .v(
            "conditions",
            Value::List(rule.conditions.iter().map(policy_condition_ref).collect()),
        )
        .v(
            "actions",
            Value::List(
                rule.actions
                    .iter()
                    .map(|a| policy_action_ref(a, root))
                    .collect(),
            ),
        )
        .done();
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm policy-rule".to_owned(),
        full_path: rule.name.clone(),
        fields,
        field_slots: IndexMap::new(),
        stanza_slot: None,
        config_uri: root.uri.clone(),
    }))
}

fn policy_condition_ref(cond: &BigipPolicyCondition) -> Value {
    let fields = Fields::new()
        .i("index", cond.index)
        .s("operand", &cond.operand)
        .s("selector", &cond.selector)
        .s("operator", &cond.operator)
        .v("values", str_list(&cond.values))
        .s("name", &cond.name)
        .b("negate", cond.negate)
        .b("case-insensitive", cond.case_insensitive)
        .s("event", &cond.event)
        .done();
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm policy-condition".to_owned(),
        full_path: cond.index.to_string(),
        fields,
        field_slots: IndexMap::new(),
        stanza_slot: None,
        config_uri: String::new(),
    }))
}

fn policy_action_ref(action: &BigipPolicyAction, root: &Rc<Root>) -> Value {
    let fields = Fields::new()
        .i("index", action.index)
        .s("target", &action.target)
        .s("verb", &action.verb)
        .v("pool", path_ref(&action.pool, "ltm pool"))
        .s("location", &action.location)
        .s("name", &action.name)
        .s("value", &action.value)
        .s("path", &action.path)
        .s("query", &action.query)
        .s("host", &action.host)
        .s("event", &action.event)
        .done();
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm policy-action".to_owned(),
        full_path: action.index.to_string(),
        fields,
        field_slots: IndexMap::new(),
        stanza_slot: None,
        config_uri: root.uri.clone(),
    }))
}

fn project_data_group(o: &BigipDataGroup) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.value_type)
        .v("kind", data_group_kind_str(o.kind))
        .v("records", str_list(&o.records))
        .s("description", &o.description)
        .done()
}

/// A `Vec<String>` projected as a list of strings.
fn str_list(values: &[String]) -> Value {
    Value::List(values.iter().map(|s| Value::Str(s.clone())).collect())
}

fn project_snat_translation(o: &BigipLtmSnatTranslation) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("address", &o.address)
        .s("description", &o.description)
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .s("inherited-traffic-group", &o.inherited_traffic_group)
        .s("connection-limit", &o.connection_limit)
        .s("ip-idle-timeout", &o.ip_idle_timeout)
        .s("tcp-idle-timeout", &o.tcp_idle_timeout)
        .s("udp-idle-timeout", &o.udp_idle_timeout)
        .s("state", &o.state)
        .done()
}

// Net (L2/L3 underlay) projections

fn project_net_route(o: &BigipNetRoute) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("network", typed_str(o.network.as_ref()))
        .b("is-default-route", o.is_default_route)
        .v("gw", typed_str(o.gw.as_ref()))
        .v("pool", path_ref(&o.pool, "ltm pool"))
        // A route's `interface` carries a VLAN or tunnel path, not a physical
        // interface name — the same target set as `ltm virtual`'s
        // `transparent-nexthop`, which resolves against `net vlan` too.
        .v("interface", path_ref(&o.interface, "net vlan"))
        .b("blackhole", o.blackhole)
        .s("mtu", &o.mtu)
        .s("description", &o.description)
        .done()
}

fn project_net_vlan(o: &BigipNetVlan) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .i("tag", o.tag)
        .v(
            "interfaces",
            path_ref_list_strs(&o.interfaces, "net interface"),
        )
        .s("description", &o.description)
        .s("mtu", &o.mtu)
        .s("cmp-hash", &o.cmp_hash)
        .s("failsafe", &o.failsafe)
        .s("failsafe-action", &o.failsafe_action)
        .s("failsafe-timeout", &o.failsafe_timeout)
        .s("fwd-mode", &o.fwd_mode)
        .s("hardware-syncookie", &o.hardware_syncookie)
        .s("learning", &o.learning)
        .s("tag-mode", &o.tag_mode)
        .s("virtual-wire", &o.virtual_wire)
        .s("auto-lasthop", &o.auto_lasthop)
        .s("source-check", &o.source_check)
        .s("source-checking", &o.source_checking)
        .s("syn-flood-rate-limit", &o.syn_flood_rate_limit)
        .s("syncache-threshold", &o.syncache_threshold)
        .s("service-policy", &o.service_policy)
        .done()
}

fn project_net_self(o: &BigipNetSelf) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("address", typed_str(o.address.as_ref()))
        .v("vlan", path_ref(&o.vlan, "net vlan"))
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .v("allow-service", str_list(&o.allow_service))
        .s("description", &o.description)
        .s("floating", &o.floating)
        .s("unit", &o.unit)
        .s("service-policy", &o.service_policy)
        .s("fw-enforced-policy", &o.fw_enforced_policy)
        .s("fw-staged-policy", &o.fw_staged_policy)
        .s("inherited-traffic-group", &o.inherited_traffic_group)
        .s("address-source", &o.address_source)
        .done()
}

fn project_net_route_domain(o: &BigipNetRouteDomain) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .i("id", o.id)
        .v("vlans", path_ref_list_strs(&o.vlans, "net vlan"))
        .s("description", &o.description)
        .v("parent", path_ref(&o.parent, "net route-domain"))
        .s("strict", &o.strict)
        .s("fw-enforced-policy", &o.fw_enforced_policy)
        .s("fw-staged-policy", &o.fw_staged_policy)
        .s("bwc-policy", &o.bwc_policy)
        .s("connection-limit", &o.connection_limit)
        .s("flow-eviction-policy", &o.flow_eviction_policy)
        .v("routing-protocol", str_list(&o.routing_protocol))
        .s("security-nat-policy", &o.security_nat_policy)
        .s("service-policy", &o.service_policy)
        .done()
}

fn project_net_port_list(o: &BigipNetPortList) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("ports", str_list(&o.ports))
        .s("description", &o.description)
        .done()
}

fn project_net_interface(o: &BigipNetInterface) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("mac-address", &o.mac_address)
        .b("enabled", o.enabled)
        .b("disabled", o.disabled)
        .s("description", &o.description)
        .s("bundle", &o.bundle)
        .s("bundle-speed", &o.bundle_speed)
        .s("lldp-admin", &o.lldp_admin)
        .s("mtu", &o.mtu)
        .s("flow-control", &o.flow_control)
        .s("media-active", &o.media_active)
        .s("media-fixed", &o.media_fixed)
        .s("media-max", &o.media_max)
        .s("media-sfp", &o.media_sfp)
        .s("port-fwd-mode", &o.port_fwd_mode)
        .s("qinq-ethertype", &o.qinq_ethertype)
        .s("stp", &o.stp)
        .s("stp-edge-port", &o.stp_edge_port)
        .s("stp-link-type", &o.stp_link_type)
        .s("stp-auto-edge-port", &o.stp_auto_edge_port)
        .s("stp-reset", &o.stp_reset)
        .s("sflow-poll-interval", &o.sflow_poll_interval)
        .s("sflow-poll-interval-global", &o.sflow_poll_interval_global)
        .s("vendor", &o.vendor)
        .s("vendor-oui", &o.vendor_oui)
        .s("vendor-partnum", &o.vendor_partnum)
        .s("vendor-revision", &o.vendor_revision)
        .s("virtual-wire", &o.virtual_wire)
        .s("transmitter-technology", &o.transmitter_technology)
        .s("lacp-port-priority", &o.lacp_port_priority)
        .done()
}

fn project_net_dns_resolver(o: &BigipNetDnsResolver) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v(
            "route-domain",
            path_ref(&o.route_domain, "net route-domain"),
        )
        .v("nameservers", str_list(&o.nameservers))
        .v("forward-zones", str_list(&o.forward_zones))
        .s("description", &o.description)
        .s("cache-size", &o.cache_size)
        .s("randomize-query-name-case", &o.randomize_query_name_case)
        .s("use-ipv4", &o.use_ipv4)
        .s("use-ipv6", &o.use_ipv6)
        .s("use-tcp", &o.use_tcp)
        .s("use-udp", &o.use_udp)
        .s("answer-default-zones", &o.answer_default_zones)
        .s("prefetch", &o.prefetch)
        .s("nameserver-min-rtt", &o.nameserver_min_rtt)
        .s("nameserver-ttl", &o.nameserver_ttl)
        .s("outbound-msg-retry", &o.outbound_msg_retry)
        .done()
}

fn project_net_tunnel(o: &BigipNetTunnel) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("profile", &o.profile)
        .s("local-address", &o.local_address)
        .s("remote-address", &o.remote_address)
        .s("secondary-address", &o.secondary_address)
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .s("description", &o.description)
        .s("mtu", &o.mtu)
        .s("mode", &o.mode)
        .s("idle-timeout", &o.idle_timeout)
        .s("auto-lasthop", &o.auto_lasthop)
        .s("transparent", &o.transparent)
        .s("key", &o.key)
        .s("use-pmtu", &o.use_pmtu)
        .s("tos", &o.tos)
        .done()
}

fn project_net_stp(o: &BigipNetStp) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v(
            "interfaces",
            path_ref_list_strs(&o.interfaces, "net interface"),
        )
        .v("vlans", path_ref_list_strs(&o.vlans, "net vlan"))
        .s("description", &o.description)
        .s("mode", &o.mode)
        .s("priority", &o.priority)
        .s("external-path-cost", &o.external_path_cost)
        .s("internal-path-cost", &o.internal_path_cost)
        .done()
}

// Sys projections

fn project_sys_dns(o: &BigipSysDns) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("name-servers", str_list(&o.name_servers))
        .v("search", str_list(&o.search))
        .done()
}

fn project_sys_ntp(o: &BigipSysNtp) -> IndexMap<String, Value> {
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

fn project_sys_snmp(o: &BigipSysSnmp) -> IndexMap<String, Value> {
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

fn project_sys_global_settings(o: &BigipSysGlobalSettings) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("hostname", &o.hostname)
        .s("gui-setup", &o.gui_setup)
        .s("mgmt-dhcp", &o.mgmt_dhcp)
        .done()
}

fn project_sys_provision(o: &BigipSysProvision) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("level", &o.level)
        .s("cpu-ratio", &o.cpu_ratio)
        .s("memory-ratio", &o.memory_ratio)
        .s("disk-ratio", &o.disk_ratio)
        .done()
}

fn project_sys_folder(o: &BigipSysFolder) -> IndexMap<String, Value> {
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
fn project_sys_file_ssl_cert(o: &BigipSysFileSslCert) -> IndexMap<String, Value> {
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

fn project_sys_file_ssl_key(o: &BigipSysFileSslKey) -> IndexMap<String, Value> {
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

fn project_sys_management_route(o: &BigipSysManagementRoute) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("network", &o.network)
        .s("gateway", &o.gateway)
        .s("mtu", &o.mtu)
        .s("description", &o.description)
        .done()
}

// CM (device cluster) projections

/// `cm cert` — the device-trust cert. Same x509 metadata spelling as
/// `sys file ssl-cert`, so `x509_from_config` projects either one.
fn project_cm_cert(o: &BigipCmCert) -> IndexMap<String, Value> {
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

fn project_cm_key(o: &BigipCmKey) -> IndexMap<String, Value> {
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

fn project_cm_device(o: &BigipCmDevice) -> IndexMap<String, Value> {
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

fn project_cm_device_group(o: &BigipCmDeviceGroup) -> IndexMap<String, Value> {
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

fn project_cm_traffic_group(o: &BigipCmTrafficGroup) -> IndexMap<String, Value> {
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

fn project_cm_ha_group(o: &BigipCmHaGroup) -> IndexMap<String, Value> {
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

fn project_cm_trust_domain(o: &BigipCmTrustDomain) -> IndexMap<String, Value> {
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

// APM projections

/// `apm policy access-policy` — the VPN / webtop policy graph's root.
///
/// `start-item` / `items[]` / `default-ending` are [`PathRef`]s into
/// `apm policy policy-item`, so `.apm["access-policy"][].start-item.caption`
/// walks the policy in one chain.
fn project_apm_access_policy(o: &BigipApmPolicyAccessPolicy) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v(
            "start-item",
            path_ref(&o.start_item, "apm policy policy-item"),
        )
        .v(
            "default-ending",
            path_ref(&o.default_ending, "apm policy policy-item"),
        )
        .v(
            "items",
            path_ref_list_strs(&o.items, "apm policy policy-item"),
        )
        .s("description", &o.description)
        .done()
}

fn project_apm_policy_item(o: &BigipApmPolicyItem) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("caption", &o.caption)
        .s("color", &o.color)
        .s("type", &o.item_type)
        .v("agents", path_ref_list_strs(&o.agents, "apm policy agent"))
        .s("description", &o.description)
        .done()
}

fn project_apm_policy_agent(o: &BigipApmPolicyAgent) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.agent_type)
        // `customization-group` names an `apm policy customization-group`,
        // which has no typed model to navigate into — keep the path as text.
        .s("customization-group", &o.customization_group)
        .s("auth", &o.auth)
        .s("server", &o.server)
        .s("max-logon-attempt", &o.max_logon_attempt)
        .s("auth-max-logon-attempt", &o.auth_max_logon_attempt)
        .s("fetch-nested-groups", &o.fetch_nested_groups)
        .s("fetch-primary-groups", &o.fetch_primary_groups)
        .s("password-source", &o.password_source)
        .s("query", &o.query)
        .s("query-attrname", &o.query_attrname)
        .s("query-filter", &o.query_filter)
        .s("show-extended-error", &o.show_extended_error)
        .s("upn", &o.upn)
        .s("username-source", &o.username_source)
        .s(
            "attribute-consuming-service",
            &o.attribute_consuming_service,
        )
        .s(
            "attr-consuming-service-session-var",
            &o.attr_consuming_service_session_var,
        )
        .s("hints", &o.hints)
        .done()
}

fn project_apm_customization_source(
    o: &BigipApmPolicyCustomizationSource,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .done()
}

fn project_apm_oauth_db_instance(o: &BigipApmOauthDbInstance) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("db-name", &o.db_name)
        .s("purge-frequency", &o.purge_frequency)
        .s("purge-time", &o.purge_time)
        .s("description", &o.description)
        .done()
}

fn project_apm_ssh_security_config(
    o: &BigipApmEphemeralAuthSshSecurityConfig,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("ciphers", str_list(&o.ciphers))
        .v("hmacs", str_list(&o.hmacs))
        .v("kex-methods", str_list(&o.kex_methods))
        .v("compressions", str_list(&o.compressions))
        .s("description", &o.description)
        .done()
}

fn project_apm_default_report(o: &BigipApmReportDefaultReport) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("report-name", &o.report_name)
        .s("user", &o.user)
        .done()
}

// GTM projections

fn project_gtm_datacenter(o: &BigipGtmDatacenter) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("contact", &o.contact)
        .s("location", &o.location)
        .s("description", &o.description)
        .v("prober-pool", path_ref(&o.prober_pool, "gtm prober-pool"))
        .s("prober-preference", &o.prober_preference)
        .s("prober-fallback", &o.prober_fallback)
        .s("state", &o.state)
        .done()
}

fn project_gtm_server(o: &BigipGtmServer) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("datacenter", path_ref(&o.datacenter, "gtm datacenter"))
        .v("monitor", monitor_value(&o.monitor))
        .s("product", &o.product)
        // The device's own (self) IP addresses.
        .v("addresses", str_list(&o.addresses))
        // The destinations of the server's virtual-servers — these are the
        // downstream LTM virtual addresses this GTM balances across, and the
        // signal the report uses to link a GTM tier to its LTM tiers.
        .v("virtual-servers", str_list(&o.virtual_servers))
        .s("description", &o.description)
        .s("state", &o.state)
        .v("prober-pool", path_ref(&o.prober_pool, "gtm prober-pool"))
        .s("virtual-server-discovery", &o.virtual_server_discovery)
        .done()
}

fn project_gtm_pool(o: &BigipGtmPool) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("record-type", &o.record_type)
        .v("monitor", monitor_value(&o.monitor))
        .s("load-balancing-mode", &o.load_balancing_mode)
        .s("alternate-mode", &o.alternate_mode)
        .s("fallback-mode", &o.fallback_mode)
        .s("fallback-ip", &o.fallback_ip)
        .s("ttl", &o.ttl)
        .s("max-answers-returned", &o.max_answers_returned)
        .s("verify-member-availability", &o.verify_member_availability)
        .v("members", project_gtm_pool_members(&o.members))
        .s("description", &o.description)
        .s("state", &o.state)
        .done()
}

fn project_gtm_pool_members(members: &[BigipGtmPoolMember]) -> Value {
    Value::List(
        members
            .iter()
            .map(|m| {
                Value::Object(
                    Fields::new()
                        .s("name", &m.name)
                        .s("service-port", &m.service_port)
                        .s("order", &m.order)
                        .s("member-order", &m.member_order)
                        .s("ratio", &m.ratio)
                        .v("monitor", monitor_value(&m.monitor))
                        .s("static-target", &m.static_target)
                        .s("state", &m.state)
                        .s("description", &m.description)
                        .done(),
                )
            })
            .collect(),
    )
}

fn project_gtm_wideip(o: &BigipGtmWideip) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("record-type", &o.record_type)
        .v("pools", path_ref_list_strs(&o.pools, "gtm pool"))
        .v("aliases", str_list(&o.aliases))
        .s("pool-lb-mode", &o.pool_lb_mode)
        .v(
            "last-resort-pool",
            path_ref(&o.last_resort_pool, "gtm pool"),
        )
        .s("persistence", &o.persistence)
        .s("description", &o.description)
        .s("state", &o.state)
        .done()
}

fn project_gtm_listener(o: &BigipGtmListener) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("address", &o.address)
        .s("port", &o.port)
        .s("ip-protocol", &o.ip_protocol)
        .s("mask", &o.mask)
        .v("pool", path_ref(&o.pool, "gtm pool"))
        .v("profiles", path_ref_list_strs(&o.profiles, "ltm profile"))
        .v("rules", path_ref_list_strs(&o.rules, "gtm rule"))
        .s("source-address-translation", &o.source_address_translation)
        .v("vlans", str_list(&o.vlans))
        .b("vlans-disabled", o.vlans_disabled)
        .b("vlans-enabled", o.vlans_enabled)
        .s("state", &o.state)
        .s("description", &o.description)
        .done()
}

// Security (AFM firewall + NAT) projections

/// Project a firewall endpoint (source / destination 5-tuple side).
fn fw_endpoint(e: &FirewallEndpoint) -> Value {
    Value::Object(
        Fields::new()
            .v("addresses", str_list(&e.addresses))
            .v(
                "address-lists",
                path_ref_list_strs(&e.address_lists, "security firewall address-list"),
            )
            .v("ports", str_list(&e.ports))
            .v(
                "port-lists",
                path_ref_list_strs(&e.port_lists, "security firewall port-list"),
            )
            .done(),
    )
}

/// Project one firewall / NAT rule to a structured object.
fn fw_rule(r: &FirewallRule) -> Value {
    Value::Object(
        Fields::new()
            .s("name", &r.name)
            .s("action", &r.action)
            .s("ip-protocol", &r.ip_protocol)
            .b("log", r.log)
            .v("source", fw_endpoint(&r.source))
            .v("destination", fw_endpoint(&r.destination))
            .s("rule-list", &r.rule_list)
            .done(),
    )
}

fn fw_rule_list_value(rules: &[FirewallRule]) -> Value {
    Value::List(rules.iter().map(fw_rule).collect())
}

fn project_fw_policy(o: &BigipSecurityFirewallPolicy) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("rules", str_list(&o.rules))
        .v(
            "rule-lists",
            path_ref_list_strs(&o.rule_lists, "security firewall rule-list"),
        )
        .done()
}

fn project_fw_rule_list(o: &BigipSecurityFirewallRuleList) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("rules", fw_rule_list_value(&o.rule_objects))
        .done()
}

fn project_fw_address_list(o: &BigipSecurityFirewallAddressList) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("addresses", str_list(&o.addresses))
        .v(
            "address-lists",
            path_ref_list_strs(&o.address_lists, "security firewall address-list"),
        )
        .v("fqdns", str_list(&o.fqdns))
        .done()
}

fn project_fw_port_list(o: &BigipSecurityFirewallPortList) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("ports", str_list(&o.ports))
        .done()
}

fn project_nat_policy(o: &BigipSecurityNatPolicy) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("rules", str_list(&o.rules))
        .v(
            "rule-lists",
            path_ref_list_strs(&o.rule_lists, "security nat rule-list"),
        )
        .done()
}

fn project_nat_source_translation(
    o: &BigipSecurityNatSourceTranslation,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .s("type", &o.type_)
        .v("addresses", str_list(&o.addresses))
        .v("ports", str_list(&o.ports))
        .done()
}

fn project_nat_destination_translation(
    o: &BigipSecurityNatDestinationTranslation,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .s("type", &o.type_)
        .v("addresses", str_list(&o.addresses))
        .v("ports", str_list(&o.ports))
        .done()
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
