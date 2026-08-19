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

//! F5 BIG-IP `*.conf` handling.
//!
//! A BIG-IP config is **not** Tcl source: it is a tree of
//! `module object-type identifier { ... }` stanzas (with embedded Tcl
//! only inside `ltm rule` bodies). The native server detects a
//! canonical BIG-IP basename on `did_open`, routes the document to
//! this module for the outline, and suppresses the general Tcl
//! analyser so its diagnostics (W123 / E002 / W210, the encrypted
//! `$M$…$` marker mis-read as a `$var`) never leak.
//!
//! Two pieces live here:
//!
//! * [`is_bigip_conf_name`] — the basename → BIG-IP test
//!   (`bigip.conf`, `bigip_base.conf`, …).
//! * [`document_symbols`] — a `module → kind → object` outline built
//!   directly from the stanza headers, using the
//!   [`BigipRegistry`](tcl_registry::bigip::BigipRegistry) object-type
//!   index for longest-prefix `(module, object-type)` resolution.
//!   Nameless global singletons (`auth password-policy`,
//!   `net self-allow`, …) fall back to their kind label so no outline
//!   symbol ever carries an empty `name`.

use std::collections::BTreeMap;

use rustc_hash::{FxHashMap, FxHashSet};

use tcl_lexer::LineIndex;
use tcl_registry::bigip::{BigipPropertySpec, BigipRegistry, ValueKind};

use crate::document_symbols::{DocumentSymbol, LineRange, SymbolKind};
use crate::folding::{FoldKind, FoldingRange};

/// Canonical BIG-IP configuration file names, matched by basename
/// (not extension).
const BIGIP_CONF_NAMES: &[&str] = &[
    "bigip.conf",
    "bigip_base.conf",
    "bigip_gtm.conf",
    "bigip_script.conf",
    "bigip_user.conf",
];

/// Return `true` when `uri`'s basename is a canonical BIG-IP config
/// name. Single source of truth for the basename → BIG-IP test the
/// `did_open` dialect routing relies on.
#[must_use]
pub fn is_bigip_conf_name(uri: &str) -> bool {
    let basename = uri.rsplit(['/', '\\']).next().unwrap_or(uri);
    let lower = basename.to_ascii_lowercase();
    BIGIP_CONF_NAMES.contains(&lower.as_str())
}

/// A top-level config stanza: its header text and byte span.
struct Block {
    /// e.g. `"ltm virtual /Common/my_vs"`.
    header: String,
    /// Byte offset of the first header character.
    start_offset: usize,
    /// Byte offset one past the closing `}` (exclusive end).
    end_offset: usize,
}

/// Extract all top-level `keyword ... { ... }` blocks from `source`.
///
/// Brace-balanced scan that respects quoted strings and backslash
/// escapes. Only ASCII structural bytes (`{`, `}`, `"`, `\`,
/// whitespace, `#`) are inspected, so every recorded offset lands on a
/// UTF-8 char boundary.
fn extract_blocks(source: &str) -> Vec<Block> {
    let bytes = source.as_bytes();
    let length = bytes.len();
    let mut blocks = Vec::new();
    let mut pos = 0;

    while pos < length {
        // Skip whitespace.
        while pos < length && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= length {
            break;
        }
        // Skip a comment line.
        if bytes[pos] == b'#' {
            while pos < length && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        // Read the header up to the opening brace. A newline before any
        // brace means this line is not a block header — skip it.
        let header_start = pos;
        let mut hit_newline = false;
        while pos < length && bytes[pos] != b'{' {
            if bytes[pos] == b'\n' {
                pos += 1;
                hit_newline = true;
                break;
            }
            pos += 1;
        }
        if hit_newline {
            continue;
        }
        if pos >= length || bytes[pos] != b'{' {
            // Ran off the end without an opening brace.
            break;
        }

        let header = source[header_start..pos].trim().to_owned();
        pos += 1; // consume opening brace
        let mut depth = 1;
        while pos < length && depth > 0 {
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b'\\' if pos + 1 < length => pos += 1,
                b'"' => {
                    pos += 1;
                    while pos < length && bytes[pos] != b'"' {
                        if bytes[pos] == b'\\' && pos + 1 < length {
                            pos += 1;
                        }
                        pos += 1;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        blocks.push(Block {
            header,
            start_offset: header_start,
            end_offset: pos,
        });
    }

    blocks
}

/// Split `header` on whitespace, honouring `"..."` quoted spans and
/// backslash escapes. BIG-IP allows
/// quoted names with embedded spaces (`security bot-defense signature
/// "/Common/Microsoft Access"`).
fn tokenise_header(header: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut buf = String::new();
    let mut in_quote = false;
    let mut escape = false;
    for ch in header.chars() {
        if escape {
            buf.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => in_quote = !in_quote,
            c if c.is_whitespace() && !in_quote => {
                if !buf.is_empty() {
                    tokens.push(std::mem::take(&mut buf));
                }
            }
            c => buf.push(c),
        }
    }
    if !buf.is_empty() {
        tokens.push(buf);
    }
    tokens
}

/// Parse a stanza header into `(module, object_type, identifier)`.
///
/// When the registry knows multi-word object types for the module
/// (`sys diags ihealth`, `ltm message-routing diameter route`), the
/// longest matching `(module, object-type)` prefix wins so the
/// remaining tail is the identifier. Otherwise it falls back to the
/// heuristic "everything between the module word and the last token is
/// the object type". An empty identifier denotes a global singleton.
fn parse_header(
    header: &str,
    object_types_by_module: &FxHashMap<&str, FxHashSet<&str>>,
) -> Option<(String, String, String)> {
    let parts = tokenise_header(header);
    if parts.len() < 2 {
        return None;
    }
    let module = parts[0].clone();
    if parts.len() == 2 {
        return Some((module, parts[1].clone(), String::new()));
    }

    // Longest-prefix match against the registry's known object types.
    if let Some(known) = object_types_by_module.get(module.as_str()) {
        for prefix_len in (2..parts.len()).rev() {
            let candidate = parts[1..prefix_len].join(" ");
            if known.contains(candidate.as_str()) {
                let identifier = parts[prefix_len..].join(" ");
                return Some((module, candidate, identifier));
            }
        }
        // The whole tail is a known type with no identifier (singleton).
        let whole = parts[1..].join(" ");
        if known.contains(whole.as_str()) {
            return Some((module, whole, String::new()));
        }
    }

    // Heuristic fallback: last token is the identifier.
    let object_type = parts[1..parts.len() - 1].join(" ");
    if object_type.is_empty() {
        let id = parts.get(2).cloned().unwrap_or_default();
        return Some((module, parts[1].clone(), id));
    }
    let identifier = parts[parts.len() - 1].clone();
    Some((module, object_type, identifier))
}

/// Build `module -> {object-type}` from the BIG-IP registry's header
/// index, so `parse_header` can do longest-prefix object-type matching.
fn object_types_by_module(
    registry: &BigipRegistry,
) -> FxHashMap<&'static str, FxHashSet<&'static str>> {
    let mut map: FxHashMap<&'static str, FxHashSet<&'static str>> = FxHashMap::default();
    for spec in registry.specs() {
        for &(module, object_type) in spec.header_types {
            map.entry(module).or_default().insert(object_type);
        }
    }
    map
}

/// One grouped outline leaf: object name, its range, and the header's
/// start byte (used only to sort entries within a kind group).
type Entry = (String, LineRange, usize);

/// Compute folding ranges for a BIG-IP config.
///
/// A `.conf` is not Tcl source, so the Tcl folding walk finds nothing in it
/// beyond comment blocks: every stanza — `ltm virtual … { … }` and each of
/// its nested `profiles { … }` / `records { … }` sub-blocks — stays
/// unfoldable.  This is the config-shaped equivalent: a brace-balanced scan
/// that folds every multi-line `{ … }` region at any depth, plus the same
/// comment-block folds the Tcl provider emits.
///
/// The embedded Tcl inside an `ltm rule` body folds through the same scan —
/// its `when` / `if` / `foreach` blocks are brace regions like any other.
///
/// Regions are properly nested by construction (they come off a brace
/// stack), so no overlap normalisation is needed.  The end line is the line
/// *before* the closing brace, matching the Tcl provider so the `}` stays
/// visible when a region is collapsed.
#[must_use]
pub fn folding_ranges(source: &str) -> Vec<FoldingRange> {
    if source.is_empty() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    let mut ranges: Vec<FoldingRange> = Vec::new();

    for (open, close) in brace_regions(source) {
        let (Ok(open), Ok(close)) = (u32::try_from(open), u32::try_from(close)) else {
            continue;
        };
        let start_line = line_index.line_at(open);
        // The closing brace's own line stays visible, so the fold ends on
        // the line above it — a single-line `{ … }` folds to nothing.
        let Some(end_line) = line_index.line_at(close).checked_sub(1) else {
            continue;
        };
        if end_line <= start_line || !seen.insert((start_line, end_line)) {
            continue;
        }
        ranges.push(FoldingRange {
            start_line,
            end_line,
            kind: FoldKind::Region,
        });
    }

    collect_extracted_comment_folds(source, &line_index, &mut seen, &mut ranges);
    ranges.sort_by_key(|r| (r.start_line, std::cmp::Reverse(r.end_line)));
    ranges
}

/// Collect comment facts from the real tmsh and iRule regions instead of
/// lexing a whole `.conf` as one Tcl script.  At file scope every stanza is a
/// braced word, so that view necessarily hides its interior comments.
///
/// The tmsh extractor gives us only actual stanza bodies, and the iRule
/// extractor gives us the exact Tcl body of an `ltm`/`gtm rule`; this keeps a
/// `#` in quoted or braced data inert while preserving absolute source lines.
fn collect_extracted_comment_folds(
    source: &str,
    line_index: &LineIndex,
    seen: &mut FxHashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
) {
    let tmsh_config = tcl_lexer::LexerConfig::for_file_dialect("f5-tmsh");
    crate::folding::collect_comment_folds(source, tmsh_config, seen, ranges);
    let bigip_registry = BigipRegistry::build();
    let header_types = object_types_by_module(&bigip_registry);
    for block in tcl_bigip::parser::helpers::extract_blocks(source) {
        let body_start = block.start_offset.saturating_add(1);
        let body_end = block.end_offset.saturating_sub(1);
        let Some(body) = source.get(body_start..body_end) else {
            continue;
        };
        let properties = parse_header(&block.header, &header_types)
            .and_then(|(module, object_type, _)| {
                bigip_registry
                    .get_by_header(&module, &object_type)
                    .map(|spec| spec.properties)
            })
            .unwrap_or_default();
        collect_tmsh_comment_folds(
            body,
            body_start,
            line_index,
            properties,
            tmsh_config,
            seen,
            ranges,
        );
    }

    let irules_profile = tcl_dialect::DialectProfile::irules();
    let irules_registry = crate::registry_for_dialect_profile(irules_profile);
    for rule in tcl_bigip::rule_extract::find_embedded_rules(source) {
        let Some(body) = source.get(rule.body_start_offset..rule.body_end_offset) else {
            continue;
        };
        let facts: FxHashSet<u32> = tcl_compiler::analyser::utils::script_comment_facts(
            body,
            tcl_lexer::LexerConfig::for_file_dialect("f5-irules"),
            irules_registry,
        )
        .into_iter()
        .filter_map(|fact| u32::try_from(fact.line).ok())
        .collect();
        crate::folding::collect_comment_folds_from_facts(
            body,
            line_index.line_at(u32::try_from(rule.body_start_offset).unwrap_or(u32::MAX)),
            &facts,
            seen,
            ranges,
        );
    }
}

/// Recurse only into tmsh values the BIG-IP schema declares structured.  The
/// parser supplies exact value spans, so a braced string property (such as a
/// description) cannot be mistaken for a nested config region.
fn collect_tmsh_comment_folds(
    body: &str,
    body_offset: usize,
    document_lines: &LineIndex,
    properties: &'static [BigipPropertySpec],
    config: tcl_lexer::LexerConfig,
    seen: &mut FxHashSet<(u32, u32)>,
    ranges: &mut Vec<FoldingRange>,
) {
    let facts = tcl_lexer::comment_line_starts(body, config)
        .into_iter()
        .collect();
    crate::folding::collect_comment_folds_from_facts(
        body,
        document_lines.line_at(u32::try_from(body_offset).unwrap_or(u32::MAX)),
        &facts,
        seen,
        ranges,
    );

    for (key, property) in tcl_bigip::parser::helpers::parse_properties_with_spans(body) {
        let Some(schema) = properties.iter().find(|schema| schema.name == key) else {
            continue;
        };
        if !matches!(
            schema.value_type,
            ValueKind::List | ValueKind::Block | ValueKind::Object
        ) && !schema.is_block()
        {
            continue;
        }
        let (Some(value_start), Some(value_end)) = (property.value_start, property.value_end)
        else {
            continue;
        };
        let Some(raw_value) = body.get(value_start..value_end) else {
            continue;
        };
        let leading = raw_value.len().saturating_sub(raw_value.trim_start().len());
        let trailing = raw_value.len().saturating_sub(raw_value.trim_end().len());
        let start = value_start.saturating_add(leading);
        let end = value_end.saturating_sub(trailing);
        if end <= start + 1 || body.as_bytes().get(start) != Some(&b'{') {
            continue;
        }
        let Some(nested) = body.get(start + 1..end - 1) else {
            continue;
        };
        collect_tmsh_comment_folds(
            nested,
            body_offset.saturating_add(start + 1),
            document_lines,
            schema.block,
            config,
            seen,
            ranges,
        );
    }
}

/// Every `{ … }` region in `source`, at any nesting depth, as
/// `(open-brace offset, close-brace offset)` pairs.
///
/// Iterative (a brace stack, not recursion), so a pathologically nested
/// config cannot overflow the native stack.  Quoted spans, backslash
/// escapes and whole-line `#` comments are skipped so a brace inside one
/// does not unbalance the scan — the same structural bytes
/// [`extract_blocks`] respects, applied at every depth rather than only at
/// the top level.  Unbalanced input degrades quietly: an unmatched `{` is
/// dropped with the stack, an unmatched `}` is ignored.
fn brace_regions(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let length = bytes.len();
    let mut stack: Vec<usize> = Vec::new();
    let mut regions: Vec<(usize, usize)> = Vec::new();
    let mut pos = 0;
    // Whether only whitespace has been seen so far on the current line —
    // a `#` there opens a comment, a `#` anywhere else is ordinary text
    // (BIG-IP descriptions and iRule string literals both contain them).
    let mut line_blank_so_far = true;

    while pos < length {
        match bytes[pos] {
            b'\n' => {
                line_blank_so_far = true;
                pos += 1;
            }
            b' ' | b'\t' | b'\r' => pos += 1,
            b'#' if line_blank_so_far => {
                while pos < length && bytes[pos] != b'\n' {
                    pos += 1;
                }
            }
            b'\\' => {
                // A backslash-newline continues the logical line, but the
                // *physical* next line still starts fresh for `#` purposes.
                line_blank_so_far = bytes.get(pos + 1) == Some(&b'\n');
                pos += 2;
            }
            b'"' => {
                line_blank_so_far = false;
                pos += 1;
                while pos < length && bytes[pos] != b'"' {
                    if bytes[pos] == b'\\' {
                        pos += 1;
                    }
                    pos += 1;
                }
                pos += 1;
            }
            b'{' => {
                line_blank_so_far = false;
                stack.push(pos);
                pos += 1;
            }
            b'}' => {
                line_blank_so_far = false;
                if let Some(open) = stack.pop() {
                    regions.push((open, pos));
                }
                pos += 1;
            }
            _ => {
                line_blank_so_far = false;
                pos += 1;
            }
        }
    }

    regions
}

/// Build a `module → kind → object` outline for a BIG-IP config.
///
/// A three-level tree
/// where the leaf is each object's full path, falling back to the kind
/// label for nameless global singletons so no symbol carries an empty
/// `name`. A non-BIG-IP / empty document yields an empty list.
#[must_use]
pub fn document_symbols(source: &str) -> Vec<DocumentSymbol> {
    if source.is_empty() {
        return Vec::new();
    }
    let registry = BigipRegistry::build();
    let otypes = object_types_by_module(&registry);
    let line_index = LineIndex::new(source);

    // Group by module → object-type → [(name, range, start byte)].
    // `BTreeMap` keeps modules and kinds in stable sorted order.
    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<Entry>>> = BTreeMap::new();

    for block in extract_blocks(source) {
        let Some((module, object_type, identifier)) = parse_header(&block.header, &otypes) else {
            continue;
        };
        // A nameless global singleton (empty identifier) would make an
        // empty `DocumentSymbol.name`, which VS Code rejects for the
        // whole outline — fall back to the kind label.
        let name = if identifier.is_empty() {
            object_type.clone()
        } else {
            identifier
        };
        let range = offsets_to_range(source, &line_index, block.start_offset, block.end_offset);
        grouped
            .entry(module)
            .or_default()
            .entry(object_type)
            .or_default()
            .push((name, range, block.start_offset));
    }

    let mut out = Vec::new();
    for (module, kinds) in grouped {
        let mut kind_symbols = Vec::new();
        for (kind, mut entries) in kinds {
            entries.sort_by_key(|(_, _, off)| *off);
            let object_symbols: Vec<DocumentSymbol> = entries
                .iter()
                .map(|(name, range, _)| DocumentSymbol {
                    name: name.clone(),
                    detail: None,
                    kind: SymbolKind::Variable,
                    range: *range,
                    selection_range: *range,
                    children: Vec::new(),
                })
                .collect();
            let kind_range = span_of(&entries);
            kind_symbols.push(DocumentSymbol {
                name: kind,
                detail: None,
                kind: SymbolKind::Class,
                range: kind_range,
                selection_range: kind_range,
                children: object_symbols,
            });
        }
        let module_range = span_over_children(&kind_symbols);
        out.push(DocumentSymbol {
            name: module,
            detail: None,
            kind: SymbolKind::Module,
            range: module_range,
            selection_range: module_range,
            children: kind_symbols,
        });
    }
    out
}

/// One parsed top-level stanza: its `(module, object_type, identifier)`
/// header decomposition and the [`LineRange`] of the whole `… { … }`
/// block.  `identifier` is empty for nameless global singletons.
pub(crate) struct Stanza {
    /// tmsh module word (`"ltm"`, `"auth"`, …).
    pub module: String,
    /// tmsh object-type (`"pool"`, `"partition"`, `"profile http"`, …).
    pub object_type: String,
    /// Object identifier / full path (`"/Common/web_pool"`, `"Tenant_A"`),
    /// or empty for a nameless singleton.
    pub identifier: String,
    /// Source range of the entire stanza.
    pub range: LineRange,
}

/// Parse every top-level stanza of a BIG-IP config into [`Stanza`]s, in
/// source order.  Shares the same `extract_blocks` / `parse_header`
/// machinery the outline ([`document_symbols`]) uses, so the
/// code-action provider sees exactly the same object decomposition the
/// outline does.  A non-BIG-IP / empty document yields an empty list.
pub(crate) fn parse_stanzas(source: &str) -> Vec<Stanza> {
    if source.is_empty() {
        return Vec::new();
    }
    let registry = BigipRegistry::build();
    let otypes = object_types_by_module(&registry);
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();
    for block in extract_blocks(source) {
        let Some((module, object_type, identifier)) = parse_header(&block.header, &otypes) else {
            continue;
        };
        let range = offsets_to_range(source, &line_index, block.start_offset, block.end_offset);
        out.push(Stanza {
            module,
            object_type,
            identifier,
            range,
        });
    }
    out
}

/// Convert a half-open byte span `[start, end)` to a [`LineRange`].
fn offsets_to_range(source: &str, line_index: &LineIndex, start: usize, end: usize) -> LineRange {
    let start_pos = line_index.position_at_utf16(u32::try_from(start).unwrap_or(u32::MAX), source);
    let end_pos = line_index.position_at_utf16(u32::try_from(end).unwrap_or(u32::MAX), source);
    LineRange {
        start_line: start_pos.line,
        start_character: start_pos.character.get(),
        end_line: end_pos.line,
        end_character: end_pos.character.get(),
    }
}

/// Range spanning every entry in a kind group (earliest start, latest
/// end).
fn span_of(entries: &[(String, LineRange, usize)]) -> LineRange {
    let mut iter = entries.iter().map(|(_, r, _)| *r);
    let first = iter.next().unwrap_or(LineRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    });
    iter.fold(first, merge)
}

/// Range spanning every child symbol's range.
fn span_over_children(children: &[DocumentSymbol]) -> LineRange {
    let mut iter = children.iter().map(|c| c.range);
    let first = iter.next().unwrap_or(LineRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    });
    iter.fold(first, merge)
}

/// Merge two ranges into one covering both (earlier start, later end).
fn merge(a: LineRange, b: LineRange) -> LineRange {
    let start_before = (a.start_line, a.start_character) <= (b.start_line, b.start_character);
    let end_after = (a.end_line, a.end_character) >= (b.end_line, b.end_character);
    LineRange {
        start_line: if start_before {
            a.start_line
        } else {
            b.start_line
        },
        start_character: if start_before {
            a.start_character
        } else {
            b.start_character
        },
        end_line: if end_after { a.end_line } else { b.end_line },
        end_character: if end_after {
            a.end_character
        } else {
            b.end_character
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
auth password-policy {
    lockout-duration 10
}
net self-allow {
    defaults {
        tcp:443
    }
}
sys diags ihealth {
    user admin
}
ltm pool /Common/p1 {
    members {
        1.2.3.4:80 { }
    }
}
ltm rule /Common/r {
    when HTTP_REQUEST {
        log local0. \"$M$mn$9uYx+bTjLD8YSGZA=\"
    }
}
";

    #[test]
    fn recognises_canonical_basenames() {
        assert!(is_bigip_conf_name("file:///x/bigip.conf"));
        assert!(is_bigip_conf_name("file:///etc/BIGIP_BASE.CONF"));
        assert!(is_bigip_conf_name("bigip_gtm.conf"));
        assert!(!is_bigip_conf_name("file:///x/app.tcl"));
        assert!(!is_bigip_conf_name("file:///x/notbigip.conf"));
    }

    fn flatten(syms: &[DocumentSymbol], out: &mut Vec<String>) {
        for s in syms {
            out.push(s.name.clone());
            flatten(&s.children, out);
        }
    }

    #[test]
    fn outline_has_only_non_empty_names() {
        let syms = document_symbols(SAMPLE);
        assert!(!syms.is_empty(), "expected a non-empty BIG-IP outline");
        let mut names = Vec::new();
        flatten(&syms, &mut names);
        assert!(
            names.iter().all(|n| !n.is_empty()),
            "empty symbol name in outline: {names:?}"
        );
    }

    #[test]
    fn nameless_singleton_falls_back_to_kind_label() {
        let syms = document_symbols(SAMPLE);
        let mut names = Vec::new();
        flatten(&syms, &mut names);
        // The module folder is named after the module word.
        assert!(names.contains(&"auth".to_owned()), "{names:?}");
        // The nameless `auth password-policy` singleton surfaces by its
        // kind label rather than an empty string.
        assert!(names.contains(&"password-policy".to_owned()), "{names:?}");
    }

    #[test]
    fn named_objects_keep_their_full_path() {
        let syms = document_symbols(SAMPLE);
        let mut names = Vec::new();
        flatten(&syms, &mut names);
        assert!(names.contains(&"/Common/p1".to_owned()), "{names:?}");
        assert!(names.contains(&"/Common/r".to_owned()), "{names:?}");
    }

    #[test]
    fn singleton_header_resolves_multi_word_object_type() {
        let registry = BigipRegistry::build();
        let otypes = object_types_by_module(&registry);
        // `sys diags ihealth` is a multi-word object type with no name.
        let parsed = parse_header("sys diags ihealth", &otypes).unwrap();
        assert_eq!(
            parsed,
            ("sys".into(), "diags ihealth".into(), String::new())
        );
    }

    #[test]
    fn empty_source_yields_no_symbols() {
        assert!(document_symbols("").is_empty());
    }

    /// `(start_line, end_line)` pairs for every region fold, in order.
    fn regions(source: &str) -> Vec<(u32, u32)> {
        folding_ranges(source)
            .into_iter()
            .filter(|r| r.kind == FoldKind::Region)
            .map(|r| (r.start_line, r.end_line))
            .collect()
    }

    #[test]
    fn folds_stanzas_and_their_nested_blocks() {
        // `ltm pool` (11..15) wraps `members` (12..14) wraps the member
        // stanza — which is single-line and so folds to nothing.
        let folds = regions(SAMPLE);
        assert!(folds.contains(&(11, 14)), "pool stanza missing: {folds:?}");
        assert!(
            folds.contains(&(12, 13)),
            "members block missing: {folds:?}"
        );
        // Every region nests properly: no partial overlaps.
        for (i, a) in folds.iter().enumerate() {
            for b in &folds[i + 1..] {
                let disjoint = b.0 > a.1 || a.0 > b.1;
                let nested = (a.0 <= b.0 && b.1 <= a.1) || (b.0 <= a.0 && a.1 <= b.1);
                assert!(disjoint || nested, "partial overlap {a:?} / {b:?}");
            }
        }
    }

    #[test]
    fn folds_tcl_inside_an_embedded_rule() {
        // The `ltm rule` body is Tcl; its `when` block is a brace region
        // like any other, so it folds through the same scan.
        let folds = regions(SAMPLE);
        assert!(folds.contains(&(16, 19)), "rule stanza missing: {folds:?}");
        assert!(folds.contains(&(17, 18)), "when block missing: {folds:?}");
    }

    #[test]
    fn single_line_block_does_not_fold() {
        // `1.2.3.4:80 { }` opens and closes on one line — nothing to hide.
        assert!(regions("ltm pool /Common/p {\n    1.2.3.4:80 { }\n}\n").contains(&(0, 1)));
        assert!(regions("x { }\n").is_empty());
    }

    #[test]
    fn braces_in_comments_and_strings_do_not_unbalance_the_scan() {
        let source = "\
# a stray { in a comment
ltm virtual /Common/v {
    description \"unbalanced } in a string\"
    destination /Common/1.2.3.4:80
}
";
        assert_eq!(regions(source), vec![(1, 3)]);
    }

    #[test]
    fn unbalanced_input_degrades_quietly() {
        // An unclosed stanza yields no fold rather than a bogus one, and a
        // stray closer is ignored.
        assert!(regions("ltm pool /Common/p {\n    members {\n").is_empty());
        assert_eq!(
            regions("}\nltm pool /Common/p {\n    a 1\n}\n"),
            vec![(1, 2)]
        );
    }

    #[test]
    fn comment_blocks_fold_like_the_tcl_provider() {
        let source = "# one\n# two\n# three\nltm node /Common/n {\n    address 1.2.3.4\n}\n";
        let folds = folding_ranges(source);
        assert!(
            folds
                .iter()
                .any(|r| r.kind == FoldKind::Comment && (r.start_line, r.end_line) == (0, 2)),
            "{folds:?}"
        );
    }

    #[test]
    fn comment_blocks_in_extracted_tmsh_stanzas_fold_at_absolute_lines() {
        let source = "ltm pool /Common/p {\n    members {\n        # pool note one\n        # pool note two\n        /Common/n:80 { address 10.0.0.1 }\n    }\n}\n";
        let folds = folding_ranges(source);
        assert!(
            folds.iter().any(|range| {
                range.kind == FoldKind::Comment && (range.start_line, range.end_line) == (2, 3)
            }),
            "{folds:?}"
        );
    }

    #[test]
    fn comment_blocks_in_embedded_irules_fold_but_data_does_not() {
        let source = "ltm rule /Common/r {\n    when HTTP_REQUEST {\n        set result [switch $kind {\n            alpha {\n                # rule note one\n                # rule note two\n                log local0. \"# data\"\n            }\n        }]\n    }\n}\nltm pool /Common/p {\n    description {\n        # braced data\n        # remains data\n    }\n}\n";
        let folds = folding_ranges(source);
        assert!(
            folds.iter().any(|range| {
                range.kind == FoldKind::Comment && (range.start_line, range.end_line) == (4, 5)
            }),
            "embedded iRule comments missing: {folds:?}"
        );
        assert!(
            !folds.iter().any(|range| {
                range.kind == FoldKind::Comment && (range.start_line, range.end_line) == (14, 15)
            }),
            "braced tmsh data became comments: {folds:?}"
        );
    }

    #[test]
    fn empty_source_yields_no_folds() {
        assert!(folding_ranges("").is_empty());
    }

    /// A stanza whose closing `}` is the file's last line must not fold past
    /// it — with or without a trailing newline.  A fold end at or beyond the
    /// client's `lineCount` is discarded wholesale by VS Code's sticky-scroll
    /// filter, taking the candidate's whole subtree with it (issue #1122).
    #[test]
    fn stanza_ending_at_eof_stays_in_bounds() {
        let base = "ltm pool /Common/p1 {\n    members {\n        1.2.3.4:80 { }\n    }\n}";
        for source in [base.to_owned(), format!("{base}\n")] {
            let line_count = u32::try_from(tcl_lexer::LineIndex::new_lsp(&source).line_count())
                .expect("line count fits u32");
            let folds = folding_ranges(&source);
            assert!(!folds.is_empty(), "expected stanza folds for {source:?}");
            for r in &folds {
                assert!(
                    r.end_line < line_count,
                    "fold {}..{} ends past the last line (line_count {line_count})",
                    r.start_line,
                    r.end_line,
                );
                assert!(r.start_line < r.end_line, "degenerate fold {r:?}");
            }
        }
    }
}
