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

//! BIG-IP object reference graph.
//!
//! Builds the node/edge graph that `f5 stats` / `cleanup` / `grep` / `validate`
//! / `graph` / `rename` all consume. Every stanza becomes an [`ObjectNode`]
//! (`_build_objects_for_source`) with a stable `node_id`, its `(module,
//! object_type, identifier)`, resolved registry `kind`, and source [`Range`].
//! The forward reference **edges** ([`build_bigip_object_graph`]) combine three
//! passes: the registry-first pilot
//! value-spec dispatch, the legacy token-scan fallback, and — for `ltm`/`gtm`
//! rule bodies — the iRule object-reference walker (`tcl-irules`).

use tcl_irules::extract_irules_object_references;
use tcl_lexer::LineIndex;
use tcl_registry::BigipRegistry;
use tcl_registry::bigip::default_registry;

use crate::error::BigipError;
use crate::model::ModelObject;
use crate::parser::driver::{BigipConfig, Placed};
use crate::parser::header::{ObjectTypeIndex, parse_generic_header};
use crate::parser::helpers::extract_blocks;
use crate::range::Range;

/// One BIG-IP object stanza as a graph node (exposed publicly as
/// `BigipObjectNode`).
#[derive(Debug, Clone)]
pub struct ObjectNode {
    /// `{uri}:{line}:{char}:{module}:{object_type}:{identifier}` — stable id.
    pub node_id: String,
    /// Source URI this node came from.
    pub uri: String,
    /// tmsh module word (`ltm`, `gtm`, …).
    pub module: String,
    /// tmsh object-type word(s).
    pub object_type: String,
    /// Object identifier / full-path (may be empty).
    pub identifier: String,
    /// Resolved registry kind, or `None` when unregistered.
    pub kind: Option<&'static str>,
    /// Stanza header text (`"ltm pool /Common/p"`).
    pub header: String,
    /// Stanza body (between the outermost braces).
    pub body: String,
    /// Byte offset of the start of the header line.
    pub header_start_offset: usize,
    /// Byte offset of the opening `{`.
    pub start_offset: usize,
    /// Byte offset one past the closing `}`.
    pub end_offset: usize,
    /// Inclusive source span of the stanza.
    pub range: Range,
}

/// A graph-building context: the BIG-IP registry + the object-type header index
/// + the iRules command registry, built once and reused across sources.
pub struct GraphContext {
    index: ObjectTypeIndex,
    /// The profile-stamped iRules command registry (cached, §9 subtractive
    /// rules applied), for the iRule edge walker.
    irules_registry: &'static tcl_registry::CommandRegistry,
}

impl GraphContext {
    /// Build the registry + header index + iRules command registry once.
    #[must_use]
    pub fn new() -> Self {
        let registry = BigipRegistry::build();
        let index = ObjectTypeIndex::build(&registry);
        let irules_registry =
            tcl_registry::registry_for_profile(tcl_dialect::DialectProfile::irules());
        Self {
            index,
            irules_registry,
        }
    }

    /// The iRules-dialect command registry, for callers that walk rule
    /// bodies with [`extract_irules_object_references`] (e.g. the
    /// document-links provider) and want to reuse this context's registry
    /// rather than rebuild one.
    #[must_use]
    pub fn irules_registry(&self) -> &tcl_registry::CommandRegistry {
        self.irules_registry
    }
}

impl Default for GraphContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Build every [`ObjectNode`] for one source: each parseable stanza becomes
/// a node, in source order.
#[must_use]
pub fn build_objects_for_source(uri: &str, source: &str, ctx: &GraphContext) -> Vec<ObjectNode> {
    let line_index = LineIndex::new(source);
    let reg = default_registry();
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    for block in extract_blocks(source) {
        let Some((module, object_type, identifier)) =
            parse_generic_header(&block.header, &ctx.index)
        else {
            continue;
        };
        // Walk back to the start of the header line.
        let mut header_start = block.start_offset;
        while header_start > 0 && bytes[header_start - 1] != b'\n' {
            header_start -= 1;
        }
        let range = Range::from_offsets(source, &line_index, block.start_offset, block.end_offset);
        let node_id = format!(
            "{uri}:{}:{}:{module}:{object_type}:{identifier}",
            range.start.line, range.start.character
        );
        let kind = reg.kind_for_header(&module, &object_type);
        result.push(ObjectNode {
            node_id,
            uri: uri.to_owned(),
            module,
            object_type,
            identifier,
            kind,
            header: block.header,
            body: block.body,
            header_start_offset: header_start,
            start_offset: block.start_offset,
            end_offset: block.end_offset,
            range,
        });
    }
    result
}

// Name resolution: resolve a `(kind, reference)` to the source span of the
// named object, which the edge builder matches back to a node.

/// A node's range identity: `(start.line, start.character, end.line,
/// end.character)`.
pub type RangeKey = (u32, u32, u32, u32);

/// The `"range"` from an object's canonical fields as a [`RangeKey`].
///
/// `canon_fields()` carries `"range": {"r": [sl, sc, so, el, ec, eo]}` for every
/// object that has a span — the model's range surfaced without a bespoke
/// accessor on the generated `ModelObject`.
fn range_key_of(object: &ModelObject) -> Option<RangeKey> {
    let cf = object.canon_fields();
    let r = cf.get("range")?.get("r")?.as_array()?;
    let g = |i: usize| -> Option<u32> { u32::try_from(r.get(i)?.as_u64()?).ok() };
    Some((g(0)?, g(1)?, g(3)?, g(4)?))
}

fn range_key_from(range: Range) -> RangeKey {
    (
        range.start.line,
        range.start.character,
        range.end.line,
        range.end.character,
    )
}

/// Resolve a possibly-short `name` to a full-path object in `table`: exact,
/// then partition-qualified against `default_partition`, then `/Common/`,
/// then a suffix match.
fn resolve_name<'a>(
    name: &str,
    table: &[&'a Placed],
    default_partition: &str,
) -> Option<&'a Placed> {
    if let Some(p) = table.iter().copied().find(|p| p.full_path == name) {
        return Some(p);
    }
    if !name.starts_with('/') {
        let trimmed = default_partition.trim_matches('/');
        let partition = if trimmed.is_empty() {
            "Common"
        } else {
            trimmed
        };
        let candidate = format!("/{partition}/{name}");
        if let Some(p) = table.iter().copied().find(|p| p.full_path == candidate) {
            return Some(p);
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if let Some(p) = table.iter().copied().find(|p| p.full_path == candidate) {
                return Some(p);
            }
        }
    }
    let suffix = format!("/{name}");
    table
        .iter()
        .copied()
        .find(|p| p.full_path.ends_with(&suffix))
}

/// Resolve a generic-object key by identifier: exact identifier, or a
/// partition-tolerant suffix match, optionally constrained by module /
/// object-types.
fn resolve_generic_object<'a>(
    cfg: &'a BigipConfig,
    clean: &str,
    module: Option<&str>,
    object_types: &[&str],
) -> Option<&'a str> {
    let clean = clean.trim();
    if clean.is_empty() {
        return None;
    }
    for (key, obj) in &cfg.generic_objects {
        if let Some(m) = module
            && obj.module != m
        {
            continue;
        }
        // The spec's object-types tuple is never empty, so the membership
        // check always applies.
        if !object_types.contains(&obj.object_type.as_str()) {
            continue;
        }
        let ident = &obj.identifier;
        // `ident == clean` (first disjunct) already covers the equality that
        // is otherwise repeated inside the non-absolute branch.
        let matches = ident == clean
            || (clean.starts_with('/') && ident.ends_with(clean))
            || (!clean.starts_with('/') && ident.ends_with(&format!("/{clean}")));
        if matches {
            return Some(key.as_str());
        }
    }
    None
}

/// Resolve `(kind, reference)` across `configs` to `(uri, range_key)`.
/// `configs` is `(uri, config)` in source order.
#[must_use]
pub fn resolve_kind_in_configs(
    kind: &str,
    reference: &str,
    configs: &[(String, &BigipConfig)],
    preferred_module: Option<&str>,
    reg: &BigipRegistry,
) -> Option<(String, RangeKey)> {
    if reference.is_empty() {
        return None;
    }
    let clean = reference.trim_matches(|c| "{}\"'[]".contains(c));
    if clean.is_empty() {
        return None;
    }
    let spec = reg.get(kind)?;
    let ks = spec.kind_spec;

    if let Some(table_name) = ks.table_name {
        for (uri, cfg) in configs {
            let table: Vec<&Placed> = cfg
                .objects
                .iter()
                .filter(|p| p.table_name == table_name)
                .collect();
            let exact = table.iter().copied().find(|p| p.full_path == clean);
            let resolved: Option<&Placed> = if exact.is_some() {
                exact
            } else if matches!(kind, "node" | "monitor" | "virtual") || ks.resolver_name.is_some() {
                resolve_name(clean, &table, &cfg.default_partition)
            } else {
                None
            };
            let Some(placed) = resolved else { continue };
            // For a table-backed kind every object shares the kind's module
            // (`ks.module`), so it stands in for the per-object `obj.module`
            // read here; the `spec.module` self-check never fires.
            if let (Some(pm), Some(om)) = (preferred_module, ks.module)
                && om != pm
            {
                continue;
            }
            if let Some(rk) = range_key_of(&placed.object) {
                return Some(((*uri).clone(), rk));
            }
        }
        return None;
    }

    for (uri, cfg) in configs {
        let mut module = ks.module;
        if preferred_module.is_some() && module.is_none() && kind == "pool" {
            module = preferred_module;
        }
        if let Some(key) = resolve_generic_object(cfg, clean, module, ks.object_types)
            && let Some((_, gobj)) = cfg.generic_objects.iter().find(|(k, _)| k == key)
            && let Some(range) = gobj.range
        {
            return Some(((*uri).clone(), range_key_from(range)));
        }
    }
    None
}

// Forward reference edges (legacy token-scan path). The registry-first
// (pilot value-spec) dispatch is layered on top; this is the always-on
// fallback path that keeps the graph complete.

use std::collections::{HashMap, HashSet};

use crate::parser::helpers::{
    parse_keyed_block_entries, parse_list_block, parse_properties_with_spans,
};

/// A forward reference edge between two object nodes (exposed publicly as
/// `BigipObjectEdge`).
#[derive(Debug, Clone)]
pub struct ObjectEdge {
    /// Referencing node id.
    pub source_id: String,
    /// Referenced node id.
    pub target_id: String,
    /// The property (or `key[]` for a list item, `irule:<cmd>` for iRules).
    pub via_property: String,
    /// The registry kind the reference resolved through.
    pub via_kind: String,
}

/// Tokens that look like references but never are.
const FALSEY_REF_TOKENS: &[&str] = &[
    "none",
    "add",
    "delete",
    "modify",
    "replace-all-with",
    "enabled",
    "disabled",
    "default",
    "all",
    "and",
    "or",
    "context",
    "clientside",
    "serverside",
    "true",
    "false",
];

const REF_STRIP: &[char] = &['{', '}', '"', '\'', '[', ']', '(', ')', ','];

/// Split a property value into reference tokens: a braced value is parsed as
/// a list block; otherwise maximal non-space, non-brace runs (`[^\s{}]+`).
fn extract_value_tokens(value: &str) -> Vec<String> {
    let stripped = value.trim();
    if stripped.is_empty() {
        return Vec::new();
    }
    if stripped.starts_with('{') && stripped.ends_with('}') {
        return parse_list_block(stripped);
    }
    stripped
        .split(|c: char| c.is_whitespace() || c == '{' || c == '}')
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether `token` could be an object reference.
fn is_candidate_reference(token: &str) -> bool {
    let clean = token.trim_matches(REF_STRIP);
    if clean.is_empty() {
        return false;
    }
    !FALSEY_REF_TOKENS.contains(&clean.to_ascii_lowercase().as_str())
}

/// Normalise a reference token for `kind`: strips delimiters, and for
/// node/virtual-address kinds drops a trailing `:port` suffix.
pub(crate) fn normalise_reference_for_kind(kind: &str, token: &str) -> String {
    let mut reference = token.trim_matches(REF_STRIP).to_owned();
    let is_addr_kind = matches!(
        kind,
        "node" | "virtual_address" | "ltm_node" | "ltm_virtual_address"
    );
    if is_addr_kind
        && reference.matches(':').count() == 1
        && let Some((left, right)) = reference.rsplit_once(':')
        && !right.is_empty()
        && right.bytes().all(|b| b.is_ascii_digit())
    {
        reference = left.to_owned();
    }
    reference
}

/// Resolve a reference to a target node id: resolve the span, match a node by
/// exact range, then by line containment.
fn resolve_target_node_id(
    kind: &str,
    reference: &str,
    source_module: Option<&str>,
    configs: &[(String, &BigipConfig)],
    by_range: &HashMap<&str, HashMap<RangeKey, String>>,
    nodes_by_uri: &[(String, Vec<ObjectNode>)],
    reg: &BigipRegistry,
) -> Option<String> {
    let (target_uri, target_rk) =
        resolve_kind_in_configs(kind, reference, configs, source_module, reg)?;
    if let Some(id) = by_range
        .get(target_uri.as_str())
        .and_then(|m| m.get(&target_rk))
    {
        return Some(id.clone());
    }
    // Containment fallback: a node whose line span encloses the target.
    let (target_start, _, target_end, _) = target_rk;
    let nodes = nodes_by_uri
        .iter()
        .find(|(u, _)| *u == target_uri)
        .map(|(_, n)| n)?;
    for node in nodes {
        let (lo, hi) = (node.range.start.line, node.range.end.line);
        if lo <= target_start && target_start <= hi && lo <= target_end && target_end <= hi {
            return Some(node.node_id.clone());
        }
    }
    None
}

/// Shared resolution context + edge accumulator for [`build_forward_edges`].
struct EdgeBuilder<'a> {
    edges: Vec<ObjectEdge>,
    seen: HashSet<(String, String, String, String)>,
    by_range: HashMap<&'a str, HashMap<RangeKey, String>>,
    nodes_by_uri: &'a [(String, Vec<ObjectNode>)],
    configs: &'a [(String, &'a BigipConfig)],
    reg: &'a BigipRegistry,
}

impl<'a> EdgeBuilder<'a> {
    fn new(
        nodes_by_uri: &'a [(String, Vec<ObjectNode>)],
        configs: &'a [(String, &'a BigipConfig)],
        reg: &'a BigipRegistry,
    ) -> Self {
        let mut by_range: HashMap<&str, HashMap<RangeKey, String>> = HashMap::new();
        for (uri, nodes) in nodes_by_uri {
            let m = by_range.entry(uri.as_str()).or_default();
            for n in nodes {
                m.insert(range_key_from(n.range), n.node_id.clone());
            }
        }
        Self {
            edges: Vec::new(),
            seen: HashSet::new(),
            by_range,
            nodes_by_uri,
            configs,
            reg,
        }
    }

    /// Resolve a single reference to a target node id under the shared context.
    fn resolve(&self, kind: &str, reference: &str, source_module: &str) -> Option<String> {
        resolve_target_node_id(
            kind,
            reference,
            Some(source_module),
            self.configs,
            &self.by_range,
            self.nodes_by_uri,
            self.reg,
        )
    }

    /// Dedup-insert one edge.
    fn push(&mut self, src: &str, target_id: String, via_property: String, kind: &str) {
        let ek = (
            src.to_owned(),
            target_id.clone(),
            via_property.clone(),
            kind.to_owned(),
        );
        if self.seen.insert(ek) {
            self.edges.push(ObjectEdge {
                source_id: src.to_owned(),
                target_id,
                via_property,
                via_kind: kind.to_owned(),
            });
        }
    }

    /// Registry-first (pilot value-spec) dispatch — runs BEFORE the legacy
    /// path, so its edges win the shared dedup and the output order matches.
    /// Migrated properties whose legacy `references` were cleared (e.g.
    /// `policies`/`vlans` on `ltm virtual`) get their edges only from here.
    fn pilot_pass(&mut self, node: &ObjectNode, key: &str, value: &str) {
        let Some(spec_refs) = pilot_references(&node.module, &node.object_type, key, value) else {
            return;
        };
        for (target_kind, target_path) in spec_refs {
            for &kind in &self.reg.candidate_registry_kinds_for_display(&target_kind) {
                let reference = normalise_reference_for_kind(kind, &target_path);
                if let Some(target_id) = self.resolve(kind, &reference, &node.module) {
                    self.push(&node.node_id, target_id, key.to_owned(), kind);
                }
            }
        }
    }

    /// Legacy key path: candidate kinds for the property name.
    fn key_pass(&mut self, node: &ObjectNode, key: &str, value: &str) {
        let key_kinds = self.reg.candidate_kinds_for_key(
            key,
            None,
            Some(&node.module),
            Some(&node.object_type),
        );
        if key_kinds.is_empty() {
            return;
        }
        for token in extract_value_tokens(value) {
            if !is_candidate_reference(&token) {
                continue;
            }
            for &kind in &key_kinds {
                let reference = normalise_reference_for_kind(kind, &token);
                if let Some(target_id) = self.resolve(kind, &reference, &node.module) {
                    self.push(&node.node_id, target_id, key.to_owned(), kind);
                }
            }
        }
    }

    /// Legacy section path: candidate kinds for list items.
    fn section_pass(&mut self, node: &ObjectNode, key: &str, value: &str) {
        let section_kinds = self.reg.candidate_kinds_for_section_item(
            key,
            Some(&node.module),
            Some(&node.object_type),
        );
        if section_kinds.is_empty() {
            return;
        }
        for token in parse_list_block(value) {
            if !is_candidate_reference(&token) {
                continue;
            }
            for &kind in &section_kinds {
                let reference = normalise_reference_for_kind(kind, &token);
                if let Some(target_id) = self.resolve(kind, &reference, &node.module) {
                    self.push(&node.node_id, target_id, format!("{key}[]"), kind);
                }
            }
        }
    }

    /// iRule object references — walk an `ltm`/`gtm` rule body and resolve every
    /// BIG-IP object it names via `extract_irules_object_references`.
    fn irule_pass(&mut self, node: &ObjectNode, irules_registry: &tcl_registry::CommandRegistry) {
        if !(matches!(node.module.as_str(), "ltm" | "gtm") && node.object_type == "rule") {
            return;
        }
        for reference in
            extract_irules_object_references(&node.body, Some(&node.module), irules_registry)
        {
            for &kind in &reference.kinds {
                if let Some(target_id) = self.resolve(kind, &reference.name, &node.module) {
                    self.push(
                        &node.node_id,
                        target_id,
                        format!("irule:{}", reference.command),
                        kind,
                    );
                }
            }
        }
    }
}

/// Build the forward reference edges across all nodes (legacy token-scan path).
fn build_forward_edges(
    nodes_by_uri: &[(String, Vec<ObjectNode>)],
    configs: &[(String, &BigipConfig)],
    reg: &BigipRegistry,
    irules_registry: &tcl_registry::CommandRegistry,
) -> Vec<ObjectEdge> {
    let mut builder = EdgeBuilder::new(nodes_by_uri, configs, reg);

    for (_uri, nodes) in nodes_by_uri {
        for node in nodes {
            for (key, prop) in parse_properties_with_spans(&node.body) {
                builder.pilot_pass(node, &key, &prop.value);
                builder.key_pass(node, &key, &prop.value);
                builder.section_pass(node, &key, &prop.value);
            }
            builder.irule_pass(node, irules_registry);
        }
    }
    builder.edges
}

/// The full object graph: per-source nodes (in source order) and the flat list
/// of forward edges.
pub struct ObjectGraph {
    /// `(uri, nodes)` in input order; nodes in source order.
    pub nodes_by_uri: Vec<(String, Vec<ObjectNode>)>,
    /// Flat forward-edge list.
    pub edges: Vec<ObjectEdge>,
}

/// Build the object reference graph from `(uri, source)` inputs. `configs`
/// supplies the parsed model per uri for reference resolution; sources without
/// a config are skipped.
#[must_use]
pub fn build_bigip_object_graph(
    sources: &[(String, String)],
    configs: &[(String, &BigipConfig)],
    ctx: &GraphContext,
) -> ObjectGraph {
    let mut nodes_by_uri: Vec<(String, Vec<ObjectNode>)> = Vec::new();
    for (uri, source) in sources {
        if !configs.iter().any(|(u, _)| u == uri) {
            continue;
        }
        nodes_by_uri.push((uri.clone(), build_objects_for_source(uri, source, ctx)));
    }
    let reg = default_registry();
    let edges = build_forward_edges(&nodes_by_uri, configs, reg, ctx.irules_registry);
    ObjectGraph {
        nodes_by_uri,
        edges,
    }
}

// Pilot value-spec reference dispatch — the registry-first edge path
// (`references_via_spec` + the migrated `PILOT_PROPERTY_SPECS`). The graph only
// consumes each `Reference`'s `(target_kind, target_path)`, so each spec is
// reproduced as a slim extractor over the raw property value rather than the
// full `ValueSpec` / `BigipList` materialisation. Specs are added incrementally;
// an unmigrated property returns `None` and falls through to the legacy path.

/// Enumerate `(target_kind, target_path)` references for a migrated property,
/// or `None` when the property isn't in the pilot table.
pub(crate) fn pilot_references(
    module: &str,
    object_type: &str,
    property: &str,
    raw: &str,
) -> Option<Vec<(String, String)>> {
    // `ListSpec(ObjectRefSpec(kind = K))` — a braced-space-separated list of
    // refs; each non-empty token yields one reference to the first kind.
    if let Some(list_ref_kind) = match (module, object_type, property) {
        ("ltm", "virtual", "rules") => Some("ltm rule"),
        ("ltm", "virtual", "policies") => Some("ltm policy"),
        ("ltm", "virtual", "vlans") => Some("net vlan"),
        ("security", "firewall policy", "rule-lists") => Some("security firewall rule-list"),
        ("security", "firewall address-list", "address-lists") => {
            Some("security firewall address-list")
        }
        _ => None,
    } {
        let refs = parse_list_block(raw)
            .into_iter()
            .filter_map(|tok| {
                let t = tok.trim();
                (!t.is_empty()).then(|| (list_ref_kind.to_owned(), t.to_owned()))
            })
            .collect();
        return Some(refs);
    }

    // `ListSpec(ProfileAttachmentSpec | PersistenceAttachmentSpec)` — a
    // keyed-block list where each item's key IS the referenced path.
    if let Some(attach_kind) = match (module, object_type, property) {
        ("ltm", "virtual", "profiles") => Some("ltm profile"),
        ("ltm", "virtual", "persist") => Some("ltm persistence"),
        _ => None,
    } {
        let refs = parse_keyed_block_entries(raw)
            .into_iter()
            .filter_map(|(key, _body)| {
                let k = key.trim();
                (!k.is_empty()).then(|| (attach_kind.to_owned(), k.to_owned()))
            })
            .collect();
        return Some(refs);
    }

    // `MonitorExpressionSpec` — a pool/node/GTM `monitor` expression; each
    // referenced monitor path targets the (family) monitor kind.
    if let Some(monitor_kind) = match (module, object_type, property) {
        ("ltm", "pool" | "node", "monitor") => Some("ltm monitor"),
        ("gtm", "pool" | "server", "monitor") => Some("gtm monitor"),
        _ => None,
    } {
        let refs = monitor_paths(raw)
            .into_iter()
            .map(|p| (monitor_kind.to_owned(), p))
            .collect();
        return Some(refs);
    }

    // `SnatModeSpec` — `source-address-translation`; references the SNAT pool
    // when the mode is `snat`.
    if (module, object_type, property) == ("ltm", "virtual", "source-address-translation") {
        let refs = snat_pool_path(raw)
            .map(|p| vec![("ltm snatpool".to_owned(), p)])
            .unwrap_or_default();
        return Some(refs);
    }

    // `ListSpec(CertKeyChainSpec)` — keyed-block; each entry references its
    // cert / key / chain SSL files.
    if matches!(
        (module, object_type, property),
        (
            "ltm",
            "profile client-ssl" | "profile server-ssl",
            "cert-key-chain"
        )
    ) {
        let refs = parse_keyed_block_entries(raw)
            .into_iter()
            .flat_map(|(_name, body)| cert_key_chain_refs(&body))
            .collect();
        return Some(refs);
    }

    // `ListSpec(FirewallRuleSpec)` — keyed-block; each rule body references its
    // source/destination port-lists + address-lists and any nested rule-list.
    if (module, object_type, property) == ("security", "firewall rule-list", "rules") {
        let refs = parse_keyed_block_entries(raw)
            .into_iter()
            .flat_map(|(_name, body)| firewall_rule_refs(&body))
            .collect();
        return Some(refs);
    }

    // Migrated but reference-free for raw-string input: `DestinationSpec` (no
    // `references()`), `DataGroupRecordSpec` / `GtmRegionMemberSpec` (no refs),
    // and `LtmPolicyRuleSpec` (no `parse()`, so it sees a string with no
    // `actions`). All fall through to the legacy path: an empty pilot result
    // followed by the unconditional legacy passes.
    None
}

/// Monitor paths referenced by a `monitor` expression: `default`/`none` →
/// none;
/// `min N of { … }` → the braced tokens; otherwise an `and`-chain of paths.
fn monitor_paths(text: &str) -> Vec<String> {
    let s = text.trim();
    if s.is_empty() || s == "default" || s == "none" {
        return Vec::new();
    }
    if let Some(rest) = s.strip_prefix("min ") {
        let rest = rest.trim_start();
        let Some(of_idx) = rest.find(" of ") else {
            return Vec::new();
        };
        if rest[..of_idx].trim().parse::<i64>().is_err() {
            return Vec::new();
        }
        let body = rest[of_idx + 4..].trim_start();
        if !body.starts_with('{') || !body.trim_end().ends_with('}') {
            return Vec::new();
        }
        let close = body.rfind('}').unwrap_or(body.len());
        return body[1..close]
            .split_whitespace()
            .map(str::to_owned)
            .collect();
    }
    // `M1 and M2 and …` (or a single bare path). A non-`and` where a separator
    // is expected is a parse failure → no references.
    let mut monitors = Vec::new();
    let mut expect_monitor = true;
    for tok in s.split_whitespace() {
        if expect_monitor {
            monitors.push(tok.to_owned());
            expect_monitor = false;
        } else if tok == "and" {
            expect_monitor = true;
        } else {
            return Vec::new();
        }
    }
    monitors
}

/// The SNAT pool path of a `source-address-translation` body, or `None`
/// unless `type snat pool …`.
fn snat_pool_path(text: &str) -> Option<String> {
    let mut body = text.trim();
    body = body.strip_prefix('{').unwrap_or(body);
    body = body.strip_suffix('}').unwrap_or(body);
    let tokens: Vec<&str> = body.split_whitespace().collect();
    let mut kind = "none";
    let mut pool = "";
    let mut idx = 0;
    while idx < tokens.len() {
        match tokens[idx] {
            "type" if idx + 1 < tokens.len() => {
                kind = match tokens[idx + 1] {
                    "none" => "none",
                    "automap" => "automap",
                    "snat" => "snat",
                    _ => return None,
                };
                idx += 2;
            }
            "pool" if idx + 1 < tokens.len() => {
                pool = tokens[idx + 1];
                idx += 2;
            }
            _ => idx += 1,
        }
    }
    (kind == "snat" && !pool.is_empty()).then(|| pool.to_owned())
}

/// SSL `(kind, path)` refs of one `cert-key-chain` entry body: cert + key +
/// chain, in order.
fn cert_key_chain_refs(body: &str) -> Vec<(String, String)> {
    let tokens: Vec<&str> = body.split_whitespace().collect();
    let (mut cert, mut key, mut chain) = ("", "", "");
    for i in 0..tokens.len() {
        let Some(value) = tokens.get(i + 1) else {
            continue;
        };
        match tokens[i] {
            "cert" => cert = value,
            "key" => key = value,
            "chain" => chain = value,
            _ => {}
        }
    }
    let mut out = Vec::new();
    if !cert.is_empty() {
        out.push(("sys file ssl-cert".to_owned(), cert.to_owned()));
    }
    if !key.is_empty() {
        out.push(("sys file ssl-key".to_owned(), key.to_owned()));
    }
    if !chain.is_empty() {
        out.push(("sys file ssl-cert".to_owned(), chain.to_owned()));
    }
    out
}

/// `(kind, path)` refs of one firewall rule body: source then destination
/// port-lists + address-lists,
/// then a nested `rule-list`.
fn firewall_rule_refs(body: &str) -> Vec<(String, String)> {
    let raw: Vec<&str> = body.split_whitespace().collect();
    // Split off the brace-balanced source/destination sub-block bodies.
    let mut top: Vec<&str> = Vec::new();
    let mut source = String::new();
    let mut destination = String::new();
    let mut idx = 0;
    while idx < raw.len() {
        let tok = raw[idx];
        if matches!(tok, "source" | "destination") && raw.get(idx + 1) == Some(&"{") {
            let mut depth = 1;
            idx += 2;
            let start = idx;
            while idx < raw.len() && depth > 0 {
                match raw[idx] {
                    "{" => depth += 1,
                    "}" => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                idx += 1;
            }
            let sub = raw[start..idx].join(" ");
            if tok == "source" {
                source = sub;
            } else {
                destination = sub;
            }
            if idx < raw.len() {
                idx += 1; // skip the closing `}`
            }
            continue;
        }
        top.push(tok);
        idx += 1;
    }
    let mut rule_list = "";
    let mut i = 0;
    while i < top.len() {
        if top[i] == "rule-list" && i + 1 < top.len() {
            rule_list = top[i + 1];
            i += 2;
        } else {
            i += 1;
        }
    }
    let mut out = Vec::new();
    endpoint_refs(&source, &mut out);
    endpoint_refs(&destination, &mut out);
    if !rule_list.is_empty() {
        out.push((
            "security firewall rule-list".to_owned(),
            rule_list.to_owned(),
        ));
    }
    out
}

/// Append one firewall endpoint's `(kind, path)` refs: port-lists then
/// address-lists (port/address literals aren't object references).
fn endpoint_refs(sub_body: &str, out: &mut Vec<(String, String)>) {
    let tokens: Vec<&str> = sub_body.split_whitespace().collect();
    let mut port_lists: Vec<&str> = Vec::new();
    let mut address_lists: Vec<&str> = Vec::new();
    let mut idx = 0;
    while idx < tokens.len() {
        let tok = tokens[idx];
        if matches!(tok, "port-lists" | "ports" | "address-lists" | "addresses")
            && tokens.get(idx + 1) == Some(&"{")
        {
            idx += 2;
            let mut items: Vec<&str> = Vec::new();
            while idx < tokens.len() && tokens[idx] != "}" {
                items.push(tokens[idx]);
                idx += 1;
            }
            if idx < tokens.len() {
                idx += 1;
            }
            match tok {
                "port-lists" => port_lists.extend(items),
                "address-lists" => address_lists.extend(items),
                _ => {}
            }
            continue;
        }
        idx += 1;
    }
    for p in port_lists {
        out.push(("security firewall port-list".to_owned(), p.to_owned()));
    }
    for p in address_lists {
        out.push(("security firewall address-list".to_owned(), p.to_owned()));
    }
}

// Graph serialisation (DOT / JSON / Mermaid) for the `f5 graph` verb.
// Operates on a built [`ObjectGraph`].

/// A serialised graph plus its node/edge counts.
pub struct GraphExport {
    /// The requested format (`"dot"` / `"json"` / `"mermaid"`).
    pub fmt: String,
    /// The serialised graph text.
    pub text: String,
    /// Number of nodes emitted.
    pub node_count: usize,
    /// Number of edges emitted.
    pub edge_count: usize,
}

/// The supported `f5 graph` formats.
pub const GRAPH_FORMATS: [&str; 3] = [
    GraphFormat::Dot.as_str(),
    GraphFormat::Json.as_str(),
    GraphFormat::Mermaid.as_str(),
];

/// Closed vocabulary of `f5 graph` output formats. `export_graph` takes the
/// CLI-typed `&str` (the wire spelling users type as `--format dot|json|mermaid`
/// is preserved unchanged) and parses it to this enum for exhaustive internal
/// dispatch — the previous `match fmt { "dot" => .., "json" => .., _ => .. }`
/// let an unrecognised-but-validated string silently fall through to mermaid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphFormat {
    Dot,
    Json,
    Mermaid,
}

impl GraphFormat {
    /// The CLI spelling for this format — must stay byte-identical to the
    /// pre-enum strings (`--format` value, `GRAPH_FORMATS`, the "unknown
    /// graph format" error list).
    const fn as_str(self) -> &'static str {
        match self {
            GraphFormat::Dot => "dot",
            GraphFormat::Json => "json",
            GraphFormat::Mermaid => "mermaid",
        }
    }
}

impl std::str::FromStr for GraphFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dot" => Ok(GraphFormat::Dot),
            "json" => Ok(GraphFormat::Json),
            "mermaid" => Ok(GraphFormat::Mermaid),
            _ => Err(()),
        }
    }
}

/// A DOT-safe node id: `n_` + each non-alphanumeric char replaced by `_`.
fn safe_id(node_id: &str) -> String {
    let mut s = String::with_capacity(node_id.len() + 2);
    s.push_str("n_");
    for c in node_id.chars() {
        s.push(if c.is_alphanumeric() { c } else { '_' });
    }
    s
}

fn node_label(node: &ObjectNode) -> String {
    let head = node.kind.unwrap_or(node.object_type.as_str());
    format!("{head}\\n{}", node.identifier)
}

/// Serialise `graph` to `fmt`, optionally filtered to the subgraph reachable
/// from `seeds`. `reverse` walks incoming references; `max_depth` bounds the
/// BFS.
///
/// # Errors
/// Returns [`BigipError::Graph`] when `fmt` is not one of [`GRAPH_FORMATS`].
pub fn export_graph(
    graph: &ObjectGraph,
    fmt: &str,
    seeds: &[String],
    reverse: bool,
    max_depth: Option<usize>,
) -> Result<GraphExport, BigipError> {
    let Ok(format) = fmt.parse::<GraphFormat>() else {
        return Err(BigipError::graph(format!(
            "unknown graph format {fmt:?} (expected one of {GRAPH_FORMATS:?})"
        )));
    };

    // Flatten nodes in source order (across uris), keyed by node_id.
    let all_nodes: Vec<&ObjectNode> = graph
        .nodes_by_uri
        .iter()
        .flat_map(|(_uri, nodes)| nodes.iter())
        .collect();
    let (kept, edges) = filter_to_subgraph(&all_nodes, &graph.edges, seeds, reverse, max_depth);

    let kept_ids: HashSet<&str> = kept.iter().map(|n| n.node_id.as_str()).collect();
    let text = match format {
        GraphFormat::Dot => to_dot(&kept, &edges, &kept_ids),
        GraphFormat::Json => to_json(&kept, &edges, &kept_ids),
        GraphFormat::Mermaid => to_mermaid(&kept, &edges, &kept_ids),
    };
    Ok(GraphExport {
        fmt: fmt.to_owned(),
        text,
        node_count: kept.len(),
        edge_count: edges
            .iter()
            .filter(|e| {
                kept_ids.contains(e.source_id.as_str()) && kept_ids.contains(e.target_id.as_str())
            })
            .count(),
    })
}

/// BFS from the seed objects (matched by identifier), returning the reached
/// nodes (in BFS-visit order) and the edges among them. With no seeds the whole
/// graph passes through unchanged (original order).
fn filter_to_subgraph<'a>(
    all_nodes: &[&'a ObjectNode],
    edges: &'a [ObjectEdge],
    seeds: &[String],
    reverse: bool,
    max_depth: Option<usize>,
) -> (Vec<&'a ObjectNode>, Vec<&'a ObjectEdge>) {
    if seeds.is_empty() {
        return (all_nodes.to_vec(), edges.iter().collect());
    }

    let mut matched_seeds: Vec<&str> = Vec::new();
    for path in seeds {
        for node in all_nodes {
            if &node.identifier == path && !matched_seeds.contains(&node.node_id.as_str()) {
                matched_seeds.push(node.node_id.as_str());
            }
        }
    }
    if matched_seeds.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        if reverse {
            adjacency
                .entry(edge.target_id.as_str())
                .or_default()
                .push(edge.source_id.as_str());
        } else {
            adjacency
                .entry(edge.source_id.as_str())
                .or_default()
                .push(edge.target_id.as_str());
        }
    }

    // `order` is the BFS visit order (seeds first, then discovered neighbours),
    // which is the order `visited` dict yields — and the order the kept
    // nodes are emitted in. (Distinct from the original source order.)
    let mut order: Vec<&str> = Vec::new();
    let mut visited: HashSet<&str> = HashSet::new();
    for &seed in &matched_seeds {
        if visited.insert(seed) {
            order.push(seed);
        }
    }
    let mut depth: HashMap<&str, usize> = matched_seeds.iter().map(|s| (*s, 0usize)).collect();
    let mut queue: std::collections::VecDeque<&str> = matched_seeds.iter().copied().collect();
    while let Some(current) = queue.pop_front() {
        let d = depth[current];
        if max_depth.is_some_and(|m| d >= m) {
            continue;
        }
        if let Some(neighbours) = adjacency.get(current) {
            for &neighbour in neighbours {
                if visited.insert(neighbour) {
                    depth.insert(neighbour, d + 1);
                    order.push(neighbour);
                    queue.push_back(neighbour);
                }
            }
        }
    }

    let by_id: HashMap<&str, &ObjectNode> =
        all_nodes.iter().map(|n| (n.node_id.as_str(), *n)).collect();
    let kept: Vec<&ObjectNode> = order
        .iter()
        .filter_map(|id| by_id.get(id).copied())
        .collect();
    let kept_ids: HashSet<&str> = kept.iter().map(|n| n.node_id.as_str()).collect();
    let kept_edges: Vec<&ObjectEdge> = edges
        .iter()
        .filter(|e| {
            kept_ids.contains(e.source_id.as_str()) && kept_ids.contains(e.target_id.as_str())
        })
        .collect();
    (kept, kept_edges)
}

fn to_dot(nodes: &[&ObjectNode], edges: &[&ObjectEdge], kept: &HashSet<&str>) -> String {
    let mut lines = vec![
        "digraph bigip {".to_owned(),
        "  rankdir=LR;".to_owned(),
        "  node [shape=box, fontname=\"monospace\"];".to_owned(),
    ];
    for node in nodes {
        lines.push(format!(
            "  {} [label=\"{}\"];",
            safe_id(&node.node_id),
            node_label(node)
        ));
    }
    for edge in edges {
        if !kept.contains(edge.source_id.as_str()) || !kept.contains(edge.target_id.as_str()) {
            continue;
        }
        lines.push(format!(
            "  {} -> {} [label=\"{}\"];",
            safe_id(&edge.source_id),
            safe_id(&edge.target_id),
            edge.via_kind
        ));
    }
    lines.push("}".to_owned());
    lines.join("\n") + "\n"
}

fn to_mermaid(nodes: &[&ObjectNode], edges: &[&ObjectEdge], kept: &HashSet<&str>) -> String {
    let mut lines = vec!["graph LR".to_owned()];
    for node in nodes {
        let label = node.identifier.replace('"', "'");
        lines.push(format!("  {}[\"{label}\"]", safe_id(&node.node_id)));
    }
    for edge in edges {
        if !kept.contains(edge.source_id.as_str()) || !kept.contains(edge.target_id.as_str()) {
            continue;
        }
        lines.push(format!(
            "  {} -->|{}| {}",
            safe_id(&edge.source_id),
            edge.via_kind,
            safe_id(&edge.target_id)
        ));
    }
    lines.join("\n") + "\n"
}

/// Serialise to 2-space-indented JSON, built by hand to match
/// the intended key order (which `serde_json`'s sorted maps wouldn't preserve).
fn to_json(nodes: &[&ObjectNode], edges: &[&ObjectEdge], kept: &HashSet<&str>) -> String {
    use std::fmt::Write as _;

    use crate::jsonfmt::json_string as q;
    let mut out = String::from("{\n  \"nodes\": [");
    for (i, node) in nodes.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        let kind = node.kind.map_or_else(|| "null".to_owned(), q);
        let _ = write!(
            out,
            "    {{\n      \"id\": {},\n      \"module\": {},\n      \"objectType\": {},\n      \"identifier\": {},\n      \"kind\": {kind},\n      \"uri\": {}\n    }}",
            q(&node.node_id),
            q(&node.module),
            q(&node.object_type),
            q(&node.identifier),
            q(&node.uri),
        );
    }
    out.push_str(if nodes.is_empty() { "],\n" } else { "\n  ],\n" });

    let visible: Vec<&&ObjectEdge> = edges
        .iter()
        .filter(|e| kept.contains(e.source_id.as_str()) && kept.contains(e.target_id.as_str()))
        .collect();
    out.push_str("  \"edges\": [");
    for (i, edge) in visible.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        let _ = write!(
            out,
            "    {{\n      \"source\": {},\n      \"target\": {},\n      \"viaProperty\": {},\n      \"viaKind\": {}\n    }}",
            q(&edge.source_id),
            q(&edge.target_id),
            q(&edge.via_property),
            q(&edge.via_kind),
        );
    }
    out.push_str(if visible.is_empty() { "]\n" } else { "\n  ]\n" });
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::{GraphContext, build_bigip_object_graph};
    use crate::parser::driver::parse_bigip_conf;

    #[test]
    fn build_bigip_object_graph_links_virtual_to_its_references() {
        // A virtual that references a pool (single ObjectRef, legacy path) and
        // an iRule (a `rules { … }` list, pilot path) — both defined.
        let src = concat!(
            "ltm pool /Common/web {\n",
            "  members {\n    /Common/n:80 { address 10.0.0.1 }\n  }\n}\n",
            "ltm rule /Common/r {\n  when HTTP_REQUEST { }\n}\n",
            "ltm virtual /Common/vs {\n  pool /Common/web\n  rules {\n    /Common/r\n  }\n}\n",
        );
        let cfg = parse_bigip_conf(src, "Common");
        let ctx = GraphContext::new();
        let uri = "file:///c.conf".to_string();
        let graph = build_bigip_object_graph(
            &[(uri.clone(), src.to_string())],
            &[(uri.clone(), &cfg)],
            &ctx,
        );

        // One input source → one node bucket; nodes for the pool, rule, virtual.
        assert_eq!(graph.nodes_by_uri.len(), 1);
        let nodes = &graph.nodes_by_uri[0].1;
        let find = |object_type: &str| {
            nodes
                .iter()
                .find(|n| n.object_type == object_type)
                .unwrap_or_else(|| panic!("missing {object_type} node"))
        };
        let vs = find("virtual");
        let pool = find("pool");
        let rule = find("rule");

        let edge = |from: &str, to: &str| {
            graph
                .edges
                .iter()
                .any(|e| e.source_id == from && e.target_id == to)
        };
        // Forward edges from the virtual to both referenced objects.
        assert!(edge(&vs.node_id, &pool.node_id), "virtual → pool edge");
        assert!(edge(&vs.node_id, &rule.node_id), "virtual → rule edge");
        // The rules-list edge records the property it originated from.
        assert!(
            graph.edges.iter().any(|e| e.source_id == vs.node_id
                && e.target_id == rule.node_id
                && e.via_property.contains("rules")),
            "rule edge is attributed to the `rules` property"
        );
    }

    #[test]
    fn sources_without_a_config_are_skipped() {
        let src = "ltm pool /Common/p {\n}\n";
        let cfg = parse_bigip_conf(src, "Common");
        let ctx = GraphContext::new();
        // Two sources, but only one has a matching config entry.
        let graph = build_bigip_object_graph(
            &[
                ("file:///has.conf".to_string(), src.to_string()),
                ("file:///orphan.conf".to_string(), src.to_string()),
            ],
            &[("file:///has.conf".to_string(), &cfg)],
            &ctx,
        );
        assert_eq!(graph.nodes_by_uri.len(), 1);
        assert_eq!(graph.nodes_by_uri[0].0, "file:///has.conf");
    }
}
