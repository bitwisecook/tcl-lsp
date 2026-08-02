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
//! The projection covers the **core LTM kinds** the cookbook + common
//! queries use: `ltm virtual`, `ltm virtual-address`, `ltm pool`
//! (+ members), `ltm node`, `ltm monitor`, `ltm rule`, `ltm data-group`,
//! `ltm persistence`, `ltm snatpool`, `ltm profile`, and `ltm policy`
//! (+ rules / conditions / actions). Each object's top-level scalar
//! properties get a `field_slot` (the byte range of the value half) so the
//! edit-plan engine can rewrite a single property in place; pool members
//! get their slots from `BigipPoolMember.field_offsets`. `stanza_slot` is
//! populated from each object's range so `--scf` / auto output matches
//! the canonical layout. The synthesised `ltm rule .refs` sub-object is built by
//! `rule_refs_value`.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use tcl_bigip::model::BigipDataGroup;
use tcl_bigip::model::{
    BigipGtmDatacenter, BigipGtmListener, BigipGtmPool, BigipGtmPoolMember, BigipGtmServer,
    BigipGtmWideip, BigipMonitor, BigipNode, BigipPersistence, BigipPolicy, BigipPolicyAction,
    BigipPolicyCondition, BigipPolicyRule, BigipPool, BigipPoolMember, BigipProfile, BigipRule,
    BigipSecurityFirewallAddressList, BigipSecurityFirewallPolicy, BigipSecurityFirewallPortList,
    BigipSecurityFirewallRuleList, BigipSecurityNatDestinationTranslation, BigipSecurityNatPolicy,
    BigipSecurityNatSourceTranslation, BigipSnatPool, BigipVirtualAddress, BigipVirtualServer,
    DataGroupType, ModelObject, ProfileType,
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
/// order matches the tail of `_MODULE_KINDS["ltm"]`. Labels not listed
/// here (the long-tail LTM kinds the Rust model doesn't carry) are simply
/// absent — navigating into them yields an empty container, matching the
/// "no entry" surface produced for a config that has no such objects.
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

/// Every covered kind table, in module order. Iterated by the per-module entry
/// builder and the kind/label lookups.
const KIND_TABLES: &[&[(&str, &str)]] = &[LTM_KINDS, GTM_KINDS, SECURITY_KINDS];

/// The `(label, tmsh_kind)` table for a module, or empty for an uncovered one.
fn module_kinds(module: &str) -> &'static [(&'static str, &'static str)] {
    match module {
        "ltm" => LTM_KINDS,
        "gtm" => GTM_KINDS,
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

/// A list of `PathRef`s for a via-legacy ref+list field — iterate the
/// `BigipList`'s item values as `for p in raw` does.
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

/// Project a `monitor` field through the `MonitorExpression` pilot — parse
/// the raw string and re-render so the JSON surface matches
/// `MonitorExpressionSpec.project`. Empty / unparseable strings fall back
/// to the raw text (the string is returned verbatim).
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
        // `rules` / `policies` / `vlans` pilot is via-legacy -> PathRef list.
        .v("rules", path_ref_list(&o.rules, "ltm rule"))
        // `profiles` / `persist` pilot is a ListSpec -> structured BigipList.
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
    // Port of `_member_object_ref`'s slot wiring: each captured field offset
    // becomes a `FieldSlot` over the value span so member properties
    // (`address`, `description`, …) are individually editable.
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
