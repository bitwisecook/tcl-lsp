//! BIG-IP object reference graph — Rust port of
//! `dialects/f5/bigip/link_extract.py`.
//!
//! Builds the node/edge graph that `f5 stats` / `cleanup` / `grep` / `validate`
//! / `graph` / `rename` all consume. This module owns the **node** half
//! (`_build_objects_for_source`): every stanza in a source becomes an
//! [`ObjectNode`] with a stable `node_id`, its `(module, object_type,
//! identifier)`, resolved registry `kind`, and source [`Range`]. The forward
//! reference **edges** (the config-resolving + registry/pilot reference walk)
//! land alongside it as the port progresses.

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

/// A graph-building context: the BIG-IP registry + the object-type header index,
/// built once and reused across sources.
pub struct GraphContext {
    #[allow(dead_code)]
    registry: BigipRegistry,
    index: ObjectTypeIndex,
}

impl GraphContext {
    /// Build the registry + header index once.
    #[must_use]
    pub fn new() -> Self {
        let registry = BigipRegistry::build();
        let index = ObjectTypeIndex::build(&registry);
        Self { registry, index }
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
