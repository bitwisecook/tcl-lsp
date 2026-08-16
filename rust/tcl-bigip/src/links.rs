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

//! BIG-IP document links — clickable object references in `.conf` / `.scf`.
//!
//! The BIG-IP sibling of the Tcl `source` / `package require` document links:
//! every object reference inside an `ltm` / `gtm` rule body, and every
//! migrated TMSH property value that resolves to another object
//! (`pool /Common/p`, `monitor /Common/m`,
//! `profiles { /Common/clientssl { } }`), becomes a clickable link pointing
//! at the target object's stanza in the same file.
//!
//! Two engines:
//!
//! - **iRule bodies** ([`extract_irules_object_references`]) — always emit a
//!   link; the target is `None` when the reference doesn't resolve so the
//!   user still sees the range the scanner recognised (tooltip
//!   "… (no definition found)").
//! - **migrated properties** (the registry pilot value-spec dispatch,
//!   [`crate::graph::pilot_references`]) — emit a link **only** when the
//!   reference resolves, pointing at the exact reference token.
//!
//! Single-file resolution: targets resolve within the open document only.
//! Cross-file links await a workspace config index in the LSP.

use tcl_irules::extract_irules_object_references;
use tcl_lexer::LineIndex;
use tcl_registry::BigipRegistry;
use tcl_registry::bigip::default_registry;

use crate::graph::{
    GraphContext, ObjectNode, build_objects_for_source, normalise_reference_for_kind,
    pilot_references, resolve_kind_in_configs,
};
use crate::parser::driver::{BigipConfig, parse_bigip_conf};
use crate::parser::helpers::parse_properties_with_spans;
use crate::range::Range;

/// Default partition used when the config names none (TMSH default).
const DEFAULT_PARTITION: &str = "Common";

/// A BIG-IP document link: a source [`Range`] (inclusive end, UTF-16 columns)
/// over the reference token, the 0-based start line of the resolved target
/// object's stanza (`None` when the reference doesn't resolve), and a hover
/// tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigipLink {
    /// Inclusive source span of the reference token.
    pub range: Range,
    /// 0-based line of the target object's stanza, or `None` if unresolved.
    pub target_line: Option<u32>,
    /// Hover tooltip (`"Go to <path>"` or `"<path> (no definition found)"`).
    pub tooltip: String,
}

/// Return a document link for every object reference in `source`.
///
/// Resolution is single-file: a reference's target is the matching stanza in
/// the same document. The returned ranges carry an **inclusive** end column
/// (the LSP layer makes it exclusive).
#[must_use]
pub fn document_links(source: &str) -> Vec<BigipLink> {
    if source.trim().is_empty() {
        return Vec::new();
    }
    let ctx = GraphContext::new();
    let config = parse_bigip_conf(source, DEFAULT_PARTITION);
    let reg = default_registry();
    let line_index = LineIndex::new(source);
    // Single-file resolution: the open document is the only config; its URI is
    // immaterial here (the caller pairs `target_line` with the document URI).
    let configs: [(String, &BigipConfig); 1] = [(String::new(), &config)];
    let nodes = build_objects_for_source("", source, &ctx);

    let mut out = Vec::new();
    out.extend(irule_links(
        source,
        &nodes,
        &ctx,
        &configs,
        &line_index,
        reg,
    ));
    out.extend(property_links(source, &nodes, &configs, &line_index, reg));
    out
}

/// Engine 1 — object references inside `ltm` / `gtm` rule bodies.
fn irule_links(
    source: &str,
    nodes: &[ObjectNode],
    ctx: &GraphContext,
    configs: &[(String, &BigipConfig)],
    line_index: &LineIndex,
    reg: &BigipRegistry,
) -> Vec<BigipLink> {
    let mut out = Vec::new();
    for node in nodes {
        if !(matches!(node.module.as_str(), "ltm" | "gtm") && node.object_type == "rule") {
            continue;
        }
        // The rule body is the verbatim slice starting one byte past the
        // opening brace, so a body-relative offset + `body_base` is absolute.
        let body_base = node.start_offset + 1;
        for r in extract_irules_object_references(&node.body, None, ctx.irules_registry()) {
            let abs_start = body_base + r.range.start() as usize;
            let abs_end = body_base + r.range.end() as usize;
            // First kind that resolves wins; `None` → unresolved (link still
            // emitted with no target).
            let target_line = r.kinds.iter().find_map(|kind| {
                resolve_kind_in_configs(kind, &r.name, configs, None, reg).map(|(_, rk)| rk.0)
            });
            let tooltip = if target_line.is_some() {
                format!("Go to {}", r.name)
            } else {
                format!("{} (no definition found)", r.name)
            };
            out.push(BigipLink {
                range: Range::from_offsets(
                    source,
                    line_index,
                    abs_start,
                    abs_end.saturating_sub(1),
                ),
                target_line,
                tooltip,
            });
        }
    }
    out
}

/// Engine 2 — references on migrated TMSH properties, via the registry pilot
/// value-spec dispatch. A link is emitted only when the reference resolves.
fn property_links(
    source: &str,
    nodes: &[ObjectNode],
    configs: &[(String, &BigipConfig)],
    line_index: &LineIndex,
    reg: &BigipRegistry,
) -> Vec<BigipLink> {
    let mut out = Vec::new();
    for node in nodes {
        let body_base = node.start_offset + 1;
        for (key, prop) in parse_properties_with_spans(&node.body) {
            let Some(value_start) = prop.value_start else {
                continue;
            };
            let Some(spec_refs) =
                pilot_references(&node.module, &node.object_type, &key, &prop.value)
            else {
                continue;
            };
            let value_base = body_base + value_start;
            // The pilot dispatch returns refs in source order, so locate each
            // path token from where the previous one ended — this keeps
            // repeated paths from all snapping to the first occurrence.
            let mut cursor = 0usize;
            for (display_kind, path) in spec_refs {
                let Some(rel) = prop.value.get(cursor..).and_then(|s| s.find(&path)) else {
                    continue;
                };
                let tok_start = cursor + rel;
                let tok_end = tok_start + path.len();
                cursor = tok_end;

                let target_line = reg
                    .candidate_registry_kinds_for_display(&display_kind)
                    .into_iter()
                    .find_map(|kind| {
                        let reference = normalise_reference_for_kind(kind, &path);
                        resolve_kind_in_configs(kind, &reference, configs, Some(&node.module), reg)
                            .map(|(_, rk)| rk.0)
                    });
                // Property links are emitted only when the target resolves
                // (an unresolved property value is more likely a literal /
                // false positive than a recognised-but-missing object).
                let Some(target_line) = target_line else {
                    continue;
                };
                let abs_start = value_base + tok_start;
                let abs_end = value_base + tok_end;
                out.push(BigipLink {
                    range: Range::from_offsets(
                        source,
                        line_index,
                        abs_start,
                        abs_end.saturating_sub(1),
                    ),
                    target_line: Some(target_line),
                    tooltip: format!("Go to {path}"),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irule_body_pool_ref_emits_resolved_link() {
        let source = "ltm pool /Common/web_pool { }\nltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/web_pool }\n}\n";
        let links = document_links(source);
        assert!(!links.is_empty(), "expected an iRule pool-ref link");
        let link = links
            .iter()
            .find(|l| l.tooltip.contains("/Common/web_pool"))
            .expect("pool ref link");
        // Resolves to the pool stanza on line 0.
        assert_eq!(link.target_line, Some(0), "{link:?}");
        assert_eq!(link.tooltip, "Go to /Common/web_pool");
    }

    #[test]
    fn irule_body_unresolved_ref_emits_link_without_target() {
        let source = "ltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/missing }\n}\n";
        let links = document_links(source);
        assert!(!links.is_empty());
        let link = links
            .iter()
            .find(|l| l.tooltip.contains("/Common/missing"))
            .expect("missing ref link");
        assert_eq!(link.target_line, None, "{link:?}");
        assert!(
            link.tooltip.contains("no definition found"),
            "{}",
            link.tooltip,
        );
    }

    #[test]
    fn irule_links_follow_reached_helpers_but_not_dormant_procedures() {
        let source = concat!(
            "ltm pool /Common/helper_pool { }\n",
            "ltm pool /Common/dormant_pool { }\n",
            "ltm rule /Common/r {\n",
            "proc helper {} { pool /Common/helper_pool }\n",
            "proc dormant {} { pool /Common/dormant_pool }\n",
            "when HTTP_REQUEST { call helper; dormant }\n",
            "}\n",
        );
        let links = document_links(source);
        assert!(
            links
                .iter()
                .any(|link| link.tooltip == "Go to /Common/helper_pool"),
            "reached helper reference must participate in BIG-IP links: {links:?}"
        );
        assert!(
            !links
                .iter()
                .any(|link| link.tooltip == "Go to /Common/dormant_pool"),
            "dormant procedure data must not create a graph edge: {links:?}"
        );
    }

    #[test]
    fn property_value_emits_bounded_link_to_target() {
        let source = "ltm monitor http /Common/http_probe { }\nltm pool /Common/web_pool {\n    monitor /Common/http_probe\n}\n";
        let links = document_links(source);
        let link = links
            .iter()
            .find(|l| l.tooltip == "Go to /Common/http_probe")
            .expect("monitor property link");
        // Resolves to the monitor stanza on line 0.
        assert_eq!(link.target_line, Some(0), "{link:?}");
        // The link range is the exact path token — bounded, not the whole
        // line, and (inclusive end) spans exactly the path length minus one.
        assert!(link.range.start.character > 0, "{link:?}");
        let span = link.range.end.character - link.range.start.character + 1;
        assert_eq!(
            span as usize,
            "/Common/http_probe".len(),
            "link range should cover exactly the path token: {link:?}",
        );
        // The reference sits on line 2 (the `monitor …` property line).
        assert_eq!(link.range.start.line, 2, "{link:?}");
    }

    #[test]
    fn keyed_block_profiles_emit_per_item_links() {
        let source = "ltm virtual /Common/v {\n    profiles {\n        /Common/clientssl { context clientside }\n        /Common/http { }\n    }\n}\nltm profile client-ssl /Common/clientssl { }\nltm profile http /Common/http { }\n";
        let links = document_links(source);
        let tooltips: Vec<&str> = links.iter().map(|l| l.tooltip.as_str()).collect();
        assert!(
            tooltips.contains(&"Go to /Common/clientssl"),
            "{tooltips:?}",
        );
        assert!(tooltips.contains(&"Go to /Common/http"), "{tooltips:?}");
    }

    #[test]
    fn no_irule_no_property_refs_yields_no_links() {
        // A lone pool with no references contributes nothing.
        assert!(document_links("ltm pool /Common/p { }\n").is_empty());
    }

    #[test]
    fn empty_source_is_safe() {
        assert!(document_links("").is_empty());
        assert!(document_links("   \n  ").is_empty());
    }
}
