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
