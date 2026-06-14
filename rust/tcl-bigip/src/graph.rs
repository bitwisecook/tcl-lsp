//! BIG-IP object reference graph — Rust port of
//! `dialects/f5/bigip/link_extract.py`.
//!
//! Builds the node/edge graph that `f5 stats` / `cleanup` / `grep` / `validate`
//! / `graph` / `rename` all consume. Every stanza becomes an [`ObjectNode`]
//! (`_build_objects_for_source`) with a stable `node_id`, its `(module,
//! object_type, identifier)`, resolved registry `kind`, and source [`Range`].
//! The forward reference **edges** ([`build_bigip_object_graph`]) combine three
//! passes per the Python `_build_forward_edges`: the registry-first pilot
//! value-spec dispatch, the legacy token-scan fallback, and — for `ltm`/`gtm`
//! rule bodies — the iRule object-reference walker (`tcl-irules`).

use tcl_irules::extract_irules_object_references;
use tcl_lexer::LineIndex;
use tcl_registry::bigip::default_registry;
use tcl_registry::BigipRegistry;

use crate::model::ModelObject;
use crate::parser::driver::{BigipConfig, Placed};
use crate::parser::header::{parse_generic_header, ObjectTypeIndex};
use crate::parser::helpers::extract_blocks;
use crate::range::Range;

/// One BIG-IP object stanza as a graph node. Mirrors the Python `_BlockObject`
/// (exposed publicly as `BigipObjectNode`).
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
    #[allow(dead_code)]
    registry: BigipRegistry,
    index: ObjectTypeIndex,
    /// Tcl command registry with the iRules dialect loaded, for the iRule edge
    /// walker.
    irules_registry: tcl_registry::CommandRegistry,
}

impl GraphContext {
    /// Build the registry + header index + iRules command registry once.
    #[must_use]
    pub fn new() -> Self {
        let registry = BigipRegistry::build();
        let index = ObjectTypeIndex::build(&registry);
        let mut irules_registry = tcl_registry::CommandRegistry::build_default();
        irules_registry.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        Self {
            registry,
            index,
            irules_registry,
        }
    }
}

impl Default for GraphContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Build every [`ObjectNode`] for one source (mirrors
/// `_build_objects_for_source`): each parseable stanza becomes a node, in
/// source order.
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
        // Walk back to the start of the header line (mirrors the Python loop).
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

// ---------------------------------------------------------------------------
// Name resolution — port of `object_registry.resolve_kind_in_configs` and the
// `BigipConfig` resolvers it relies on (`resolve_name` /
// `resolve_generic_object`). Resolves a `(kind, reference)` to the source span
// of the named object, which the edge builder matches back to a node.
// ---------------------------------------------------------------------------

/// A node's range identity: `(start.line, start.character, end.line,
/// end.character)`, mirroring the Python `_range_key`.
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

/// Resolve a possibly-short `name` to a full-path object in `table` (mirrors
/// `BigipConfig.resolve_name`): exact, then partition-qualified against
/// `default_partition`, then `/Common/`, then a suffix match.
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

/// Resolve a generic-object key by identifier (mirrors
/// `BigipConfig.resolve_generic_object`): exact identifier, or a partition-
/// tolerant suffix match, optionally constrained by module / object-types.
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
        if let Some(m) = module {
            if obj.module != m {
                continue;
            }
        }
        // The Python always passes the spec's (never-None) object-types tuple,
        // so the membership check always applies.
        if !object_types.contains(&obj.object_type.as_str()) {
            continue;
        }
        let ident = &obj.identifier;
        // `ident == clean` (first disjunct) already covers the equality the
        // Python repeats inside its non-absolute branch.
        let matches = ident == clean
            || (clean.starts_with('/') && ident.ends_with(clean))
            || (!clean.starts_with('/') && ident.ends_with(&format!("/{clean}")));
        if matches {
            return Some(key.as_str());
        }
    }
    None
}

/// Resolve `(kind, reference)` across `configs` to `(uri, range_key)` (mirrors
/// `resolve_kind_in_configs`). `configs` is `(uri, config)` in source order.
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
            // (`ks.module`), so it stands in for the per-object `obj.module` the
            // Python reads; the `spec.module` self-check never fires.
            if let (Some(pm), Some(om)) = (preferred_module, ks.module) {
                if om != pm {
                    continue;
                }
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
        if let Some(key) = resolve_generic_object(cfg, clean, module, ks.object_types) {
            if let Some((_, gobj)) = cfg.generic_objects.iter().find(|(k, _)| k == key) {
                if let Some(range) = gobj.range {
                    return Some(((*uri).clone(), range_key_from(range)));
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Forward reference edges — port of `_build_forward_edges` (legacy token-scan
// path). The registry-first (pilot value-spec) dispatch is layered on top in a
// later increment; this is the always-on fallback path that keeps the graph
// complete.
// ---------------------------------------------------------------------------

use std::collections::{HashMap, HashSet};

use crate::parser::helpers::{
    parse_keyed_block_entries, parse_list_block, parse_properties_with_spans,
};

/// A forward reference edge between two object nodes. Mirrors the Python
/// `_Edge` (`BigipObjectEdge`).
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

/// Tokens that look like references but never are (mirrors `_FALSEY_REF_TOKENS`).
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

/// Split a property value into reference tokens (mirrors `_extract_value_tokens`):
/// a braced value is parsed as a list block; otherwise maximal non-space,
/// non-brace runs (`[^\s{}]+`).
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

/// Whether `token` could be an object reference (mirrors `_is_candidate_reference`).
fn is_candidate_reference(token: &str) -> bool {
    let clean = token.trim_matches(REF_STRIP);
    if clean.is_empty() {
        return false;
    }
    !FALSEY_REF_TOKENS.contains(&clean.to_ascii_lowercase().as_str())
}

/// Normalise a reference token for `kind` (mirrors `_normalise_reference_for_kind`):
/// strips delimiters, and for node/virtual-address kinds drops a trailing
/// `:port` suffix.
fn normalise_reference_for_kind(kind: &str, token: &str) -> String {
    let mut reference = token.trim_matches(REF_STRIP).to_owned();
    let is_addr_kind = matches!(
        kind,
        "node" | "virtual_address" | "ltm_node" | "ltm_virtual_address"
    );
    if is_addr_kind && reference.matches(':').count() == 1 {
        if let Some((left, right)) = reference.rsplit_once(':') {
            if !right.is_empty() && right.bytes().all(|b| b.is_ascii_digit()) {
                reference = left.to_owned();
            }
        }
    }
    reference
}

/// Resolve a reference to a target node id (mirrors `_resolve_target_node_id`):
/// resolve the span, match a node by exact range, then by line containment.
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

/// Build the forward reference edges across all nodes (legacy token-scan path).
// A faithful port of the (long) `_build_forward_edges` — the pilot + two legacy
// reference passes plus the shared dedup read most clearly as one function.
#[allow(clippy::too_many_lines)]
fn build_forward_edges(
    nodes_by_uri: &[(String, Vec<ObjectNode>)],
    configs: &[(String, &BigipConfig)],
    reg: &BigipRegistry,
    irules_registry: &tcl_registry::CommandRegistry,
) -> Vec<ObjectEdge> {
    let mut edges = Vec::new();
    let mut seen: HashSet<(String, String, String, String)> = HashSet::new();

    let mut by_range: HashMap<&str, HashMap<RangeKey, String>> = HashMap::new();
    for (uri, nodes) in nodes_by_uri {
        let m = by_range.entry(uri.as_str()).or_default();
        for n in nodes {
            m.insert(range_key_from(n.range), n.node_id.clone());
        }
    }

    let mut push_edge = |edges: &mut Vec<ObjectEdge>,
                         src: &str,
                         target_id: String,
                         via_property: String,
                         kind: &str| {
        let ek = (
            src.to_owned(),
            target_id.clone(),
            via_property.clone(),
            kind.to_owned(),
        );
        if seen.insert(ek) {
            edges.push(ObjectEdge {
                source_id: src.to_owned(),
                target_id,
                via_property,
                via_kind: kind.to_owned(),
            });
        }
    };

    for (_uri, nodes) in nodes_by_uri {
        for node in nodes {
            for (key, prop) in parse_properties_with_spans(&node.body) {
                // Registry-first (pilot value-spec) dispatch — runs BEFORE the
                // legacy path, exactly like Python, so its edges win the shared
                // dedup and the output order matches. Migrated properties whose
                // legacy `references` were cleared (e.g. `policies`/`vlans` on
                // `ltm virtual`) get their edges only from here.
                if let Some(spec_refs) =
                    pilot_references(&node.module, &node.object_type, &key, &prop.value)
                {
                    for (target_kind, target_path) in spec_refs {
                        for &kind in &reg.candidate_registry_kinds_for_display(&target_kind) {
                            let reference = normalise_reference_for_kind(kind, &target_path);
                            if let Some(target_id) = resolve_target_node_id(
                                kind,
                                &reference,
                                Some(&node.module),
                                configs,
                                &by_range,
                                nodes_by_uri,
                                reg,
                            ) {
                                push_edge(&mut edges, &node.node_id, target_id, key.clone(), kind);
                            }
                        }
                    }
                }

                // Legacy key path: candidate kinds for the property name.
                let key_kinds = reg.candidate_kinds_for_key(
                    &key,
                    None,
                    Some(&node.module),
                    Some(&node.object_type),
                );
                if !key_kinds.is_empty() {
                    for token in extract_value_tokens(&prop.value) {
                        if !is_candidate_reference(&token) {
                            continue;
                        }
                        for &kind in &key_kinds {
                            let reference = normalise_reference_for_kind(kind, &token);
                            if let Some(target_id) = resolve_target_node_id(
                                kind,
                                &reference,
                                Some(&node.module),
                                configs,
                                &by_range,
                                nodes_by_uri,
                                reg,
                            ) {
                                push_edge(&mut edges, &node.node_id, target_id, key.clone(), kind);
                            }
                        }
                    }
                }

                // Legacy section path: candidate kinds for list items.
                let section_kinds = reg.candidate_kinds_for_section_item(
                    &key,
                    Some(&node.module),
                    Some(&node.object_type),
                );
                if section_kinds.is_empty() {
                    continue;
                }
                for token in parse_list_block(&prop.value) {
                    if !is_candidate_reference(&token) {
                        continue;
                    }
                    for &kind in &section_kinds {
                        let reference = normalise_reference_for_kind(kind, &token);
                        if let Some(target_id) = resolve_target_node_id(
                            kind,
                            &reference,
                            Some(&node.module),
                            configs,
                            &by_range,
                            nodes_by_uri,
                            reg,
                        ) {
                            push_edge(
                                &mut edges,
                                &node.node_id,
                                target_id,
                                format!("{key}[]"),
                                kind,
                            );
                        }
                    }
                }
            }

            // iRule object references — walk an `ltm`/`gtm` rule body and
            // resolve every BIG-IP object it names (mirrors the trailing
            // `extract_irules_object_references` block in `_build_forward_edges`).
            if matches!(node.module.as_str(), "ltm" | "gtm") && node.object_type == "rule" {
                for reference in extract_irules_object_references(
                    &node.body,
                    Some(&node.module),
                    irules_registry,
                ) {
                    for &kind in &reference.kinds {
                        if let Some(target_id) = resolve_target_node_id(
                            kind,
                            &reference.name,
                            Some(&node.module),
                            configs,
                            &by_range,
                            nodes_by_uri,
                            reg,
                        ) {
                            push_edge(
                                &mut edges,
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
    }
    edges
}

/// The full object graph: per-source nodes (in source order) and the flat list
/// of forward edges. Mirrors `build_bigip_object_graph`'s return shape.
pub struct ObjectGraph {
    /// `(uri, nodes)` in input order; nodes in source order.
    pub nodes_by_uri: Vec<(String, Vec<ObjectNode>)>,
    /// Flat forward-edge list.
    pub edges: Vec<ObjectEdge>,
}

/// Build the object reference graph from `(uri, source)` inputs (mirrors
/// `build_bigip_object_graph`). `configs` supplies the parsed model per uri for
/// reference resolution; sources without a config are skipped.
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
    let edges = build_forward_edges(&nodes_by_uri, configs, reg, &ctx.irules_registry);
    ObjectGraph {
        nodes_by_uri,
        edges,
    }
}

// ---------------------------------------------------------------------------
// Pilot value-spec reference dispatch — the registry-first edge path
// (`references_via_spec` + the migrated `PILOT_PROPERTY_SPECS`). The graph only
// consumes each `Reference`'s `(target_kind, target_path)`, so each spec is
// reproduced as a slim extractor over the raw property value rather than the
// full `ValueSpec` / `BigipList` materialisation. Specs are added incrementally;
// an unmigrated property returns `None` and falls through to the legacy path.
// ---------------------------------------------------------------------------

/// Enumerate `(target_kind, target_path)` references for a migrated property,
/// or `None` when the property isn't in the pilot table.
fn pilot_references(
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
    // `actions`). All fall through to the legacy path, exactly like Python's
    // empty pilot result followed by the unconditional legacy passes.
    None
}

/// Monitor paths referenced by a `monitor` expression (port of
/// `MonitorExpression.try_parse` → `.monitors`): `default`/`none` → none;
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

/// The SNAT pool path of a `source-address-translation` body (port of
/// `SnatMode.try_parse` + `references`), or `None` unless `type snat pool …`.
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

/// SSL `(kind, path)` refs of one `cert-key-chain` entry body (port of
/// `CertKeyChain.from_raw` + `references`): cert + key + chain, in order.
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

/// `(kind, path)` refs of one firewall rule body (port of `FirewallRule` +
/// `FirewallEndpoint`): source then destination port-lists + address-lists,
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

// ---------------------------------------------------------------------------
// Graph serialisation — port of `graph_export.py` (DOT / JSON / Mermaid) for
// the `f5 graph` verb. Operates on a built [`ObjectGraph`].
// ---------------------------------------------------------------------------

/// A serialised graph plus its node/edge counts (mirrors `GraphExport`).
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
pub const GRAPH_FORMATS: [&str; 3] = ["dot", "json", "mermaid"];

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
/// from `seeds` (mirrors `export_graph`). `reverse` walks incoming references;
/// `max_depth` bounds the BFS.
///
/// # Errors
/// Returns `Err` when `fmt` is not one of [`GRAPH_FORMATS`].
pub fn export_graph(
    graph: &ObjectGraph,
    fmt: &str,
    seeds: &[String],
    reverse: bool,
    max_depth: Option<usize>,
) -> Result<GraphExport, String> {
    if !GRAPH_FORMATS.contains(&fmt) {
        return Err(format!(
            "unknown graph format {fmt:?} (expected one of {GRAPH_FORMATS:?})"
        ));
    }

    // Flatten nodes in source order (across uris), keyed by node_id.
    let all_nodes: Vec<&ObjectNode> = graph
        .nodes_by_uri
        .iter()
        .flat_map(|(_uri, nodes)| nodes.iter())
        .collect();
    let (kept, edges) = filter_to_subgraph(&all_nodes, &graph.edges, seeds, reverse, max_depth);

    let kept_ids: HashSet<&str> = kept.iter().map(|n| n.node_id.as_str()).collect();
    let text = match fmt {
        "dot" => to_dot(&kept, &edges, &kept_ids),
        "json" => to_json(&kept, &edges, &kept_ids),
        _ => to_mermaid(&kept, &edges, &kept_ids),
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
/// nodes (in original order) and the edges among them. With no seeds the whole
/// graph passes through. Mirrors `_filter_to_subgraph`.
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

    let mut visited: HashSet<&str> = matched_seeds.iter().copied().collect();
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
                    queue.push_back(neighbour);
                }
            }
        }
    }

    let kept: Vec<&ObjectNode> = all_nodes
        .iter()
        .copied()
        .filter(|n| visited.contains(n.node_id.as_str()))
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

/// Serialise to `json.dumps(indent=2)`-compatible JSON, built by hand to match
/// Python's key order (which `serde_json`'s sorted maps wouldn't preserve).
fn to_json(nodes: &[&ObjectNode], edges: &[&ObjectEdge], kept: &HashSet<&str>) -> String {
    use std::fmt::Write as _;
    // Escape a string value the way `serde_json` (and `json.dumps`) does.
    let q = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_owned());
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
