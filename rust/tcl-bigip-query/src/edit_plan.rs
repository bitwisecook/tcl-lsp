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

//! Edit plan: collect, route, and apply query-driven source edits.
//!
//! Every assignment produced by the evaluator turns into an [`EditOp`];
//! [`apply`] groups them
//! by source URI, routes identity-field writes (`.name = ...`,
//! `.name |= with_partition(...)`) through
//! [`rename_object`](crate::rewrite::rename_object), runs cascading
//! [`PrefixRewrite`]s (`rename_partition` / `rename_folder` /
//! `rename_prefix`), splices each remaining non-identity edit into the source
//! text via its [`FieldSlot`], and returns one [`AppliedSource`] per touched
//! URI — carrying the [`RenameReport`](crate::rewrite::RenameReport)s the CLI
//! surfaces on stderr.
//!
//! The applier is intentionally text-oriented: it never round-trips through
//! the parser, so comments, whitespace, key order, and unknown stanzas all
//! survive.

use std::collections::HashMap;

use regex::Regex;

use crate::errors::QueryError;
use crate::rewrite::{RenameReport, rename_object};
use crate::value::{FieldSlot, MAX_VALUE_WALK_DEPTH, Value};

/// Identity fields whose location is the stanza header — writes to these route
/// through the [`rename_object`] token-rewrite engine.
const IDENTITY_FIELDS: &[&str] = &["name", "full-path"];

/// `(object_kind, field_name)` pairs that `+=` / `=` may materialise as a
/// fresh `<field> { ... }` block — flat list slots whose elements stringify
/// to bare tokens.
const MATERIALISABLE_KIND_FIELDS: &[(&str, &str)] = &[
    ("ltm virtual", "rules"),
    ("ltm virtual", "profiles"),
    ("ltm virtual", "persist"),
    ("ltm virtual", "policies"),
];

/// A single (object, field) → new-value edit recorded by the evaluator — port
/// of `edit_plan.EditOp`.
#[derive(Debug, Clone)]
pub struct EditOp {
    pub source_uri: String,
    pub object_path: String,
    pub object_kind: String,
    pub field_name: String,
    /// `"="`, `"|="`, `"+="`, or `"-="`.
    pub operator: String,
    pub new_value: Value,
    pub field_slot: Option<FieldSlot>,
    pub stanza_slot: Option<FieldSlot>,
    /// When `true` (the default), a zero-occurrence rename raises
    /// [`QueryError::Edit`]. The tolerant `rename()` builtin sets this `false`
    /// so the applier skips a no-match silently and the CLI surfaces it via
    /// the post-apply diff + exit-code 1.
    pub strict: bool,
}

/// What the byte *before* a cascade match must satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Before {
    /// No constraint.
    Any,
    /// The negative look-behind `(?<![A-Za-z0-9_/.\-])` — the preceding
    /// byte must not be a BIG-IP identifier char (start-of-string passes).
    NotIdent,
}

/// What the byte *after* a cascade match must satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum After {
    /// No constraint.
    Any,
    /// The look-ahead `(?=[A-Za-z0-9_])` — the next byte must be a name
    /// char (so an exact-prefix match still hits but trailing whitespace /
    /// punctuation does not).
    RequireNameChar,
    /// The negative look-ahead `(?![A-Za-z0-9_/.\-])` — the next byte
    /// must not be an identifier char (end-of-string passes).
    NotIdent,
}

/// A whole-source token-bounded prefix substitution.
///
/// Used by cascade operations (`rename_partition` / `rename_folder` /
/// `rename_prefix`) that rewrite *every* occurrence of a prefix — including
/// ones embedded in compound values (destination addresses, pool-member
/// names, iRule body literals) that are not standalone object identifiers and
/// so fall outside [`rename_object`]'s token-bounded match.
///
/// The reference compiles a single `re.Pattern` with look-behind / look-ahead token
/// boundaries; the `regex` crate supports neither, so the boundaries are
/// hoisted out of the [`Regex`] into [`Before`] / [`After`] and applied as a
/// manual byte check around each `core` match in [`apply`]. The substitution
/// stays identifier-safe and byte-identical to the reference.
#[derive(Debug, Clone)]
pub struct PrefixRewrite {
    pub source_uri: String,
    /// Human-readable LHS, for stderr summaries.
    pub label: String,
    /// The match core — the reference pattern with its look-behind / look-ahead
    /// assertions stripped (those move to `before` / `after`).
    pub pattern: Regex,
    pub before: Before,
    pub after: After,
    /// Replacement template using `$1` / `${1}` back-refs against `pattern`'s
    /// capture groups (`\g<1>` becomes `$1`).
    pub replacement: String,
    /// Human-readable rendering of the destination for the stderr summary —
    /// `replacement` may carry regex back-refs (`$1`) that confuse users.
    /// Defaults to `replacement` when empty.
    pub human_new: String,
}

/// Whether `b` is a BIG-IP identifier char — `[A-Za-z0-9_/.\-]`.
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'/' | b'.' | b'-')
}

/// Collected edits, applied once at the end of a query run (`ops` +
/// `prefix_rewrites`).
#[derive(Debug, Default)]
pub struct EditPlan {
    pub ops: Vec<EditOp>,
    pub prefix_rewrites: Vec<PrefixRewrite>,
}

impl EditPlan {
    #[must_use]
    pub fn new() -> Self {
        EditPlan {
            ops: Vec::new(),
            prefix_rewrites: Vec::new(),
        }
    }

    pub fn add(&mut self, op: EditOp) {
        self.ops.push(op);
    }

    pub fn add_prefix(&mut self, rewrite: PrefixRewrite) {
        self.prefix_rewrites.push(rewrite);
    }

    #[must_use]
    pub fn has_edits(&self) -> bool {
        !self.ops.is_empty() || !self.prefix_rewrites.is_empty()
    }
}

/// One source file's result after edits land.
#[derive(Debug, Clone)]
pub struct AppliedSource {
    pub uri: String,
    pub original: String,
    pub new_source: String,
    pub rename_reports: Vec<RenameReport>,
    pub field_edits: usize,
}

/// Apply every op in *plan* to *sources*, returning one [`AppliedSource`] per
/// touched URI.
///
/// Cascading prefix rewrites run first, then identity writes route through
/// [`rename_object`], and finally field edits are spliced. Mixing a prefix
/// rewrite with a field edit in the same statement is rejected (the rewrite
/// shifts byte offsets, invalidating field-slot ranges); `+=` / `-=` on an
/// identity field is rejected (arithmetic on a name is nonsensical).
///
/// # Errors
/// Returns [`QueryError::Edit`] for identity `+=` / `-=`, prefix/field mixing,
/// an empty rename target, a strict zero-occurrence rename, overlapping edits,
/// non-writable compound values, or values that cannot be encoded in SCF.
pub fn apply<S: std::hash::BuildHasher>(
    plan: &EditPlan,
    sources: &HashMap<String, String, S>,
) -> Result<HashMap<String, AppliedSource>, QueryError> {
    // Group ops by URI, preserving insertion order of first appearance.
    let mut order: Vec<String> = Vec::new();
    let mut by_uri: HashMap<String, Vec<&EditOp>> = HashMap::new();
    for op in &plan.ops {
        if !by_uri.contains_key(&op.source_uri) {
            order.push(op.source_uri.clone());
        }
        by_uri.entry(op.source_uri.clone()).or_default().push(op);
    }
    let mut prefix_by_uri: HashMap<String, Vec<&PrefixRewrite>> = HashMap::new();
    for pr in &plan.prefix_rewrites {
        if !by_uri.contains_key(&pr.source_uri) && !prefix_by_uri.contains_key(&pr.source_uri) {
            order.push(pr.source_uri.clone());
        }
        prefix_by_uri
            .entry(pr.source_uri.clone())
            .or_default()
            .push(pr);
    }

    let mut out: HashMap<String, AppliedSource> = HashMap::new();
    for uri in order {
        let ops = by_uri.get(&uri).cloned().unwrap_or_default();
        let prefixes = prefix_by_uri.get(&uri).cloned().unwrap_or_default();
        let source = sources
            .get(&uri)
            .ok_or_else(|| QueryError::edit(format!("no source loaded for {uri}")))?;
        let applied = apply_one_uri(&uri, source, &ops, &prefixes)?;
        out.insert(uri, applied);
    }
    Ok(out)
}

/// Apply the ops + prefix rewrites for a single URI — the per-file body of
/// [`apply`].
fn apply_one_uri(
    uri: &str,
    source: &str,
    ops: &[&EditOp],
    prefixes: &[&PrefixRewrite],
) -> Result<AppliedSource, QueryError> {
    if !prefixes.is_empty()
        && ops
            .iter()
            .any(|op| !IDENTITY_FIELDS.contains(&op.field_name.as_str()))
    {
        return Err(QueryError::edit(
            "cannot mix prefix-cascade rewrites (e.g. rename_partition) with \
             field edits in a single statement; split them with ';' and the \
             runner will apply each statement against the post-rewrite source",
        ));
    }

    let mut current = source.to_owned();
    let mut rename_reports: Vec<RenameReport> = Vec::new();

    // Cascading prefix rewrites first, building a synthetic RenameReport per
    // pattern so the CLI can surface the count.
    for pr in prefixes {
        let (new_text, count) = apply_prefix_rewrite(pr, &current);
        if count == 0 {
            continue;
        }
        current = new_text;
        rename_reports.push(RenameReport {
            old: pr.label.clone(),
            new: if pr.human_new.is_empty() {
                pr.replacement.clone()
            } else {
                pr.human_new.clone()
            },
            occurrences: count,
            new_source: String::new(),
        });
    }

    // Split identity vs field ops.
    let mut identity_ops: Vec<&EditOp> = Vec::new();
    let mut field_ops: Vec<&EditOp> = Vec::new();
    for op in ops {
        if IDENTITY_FIELDS.contains(&op.field_name.as_str()) {
            if op.operator == "+=" || op.operator == "-=" {
                return Err(QueryError::edit(format!(
                    "assignment {} to identity field {} is not supported",
                    op.operator,
                    crate::eval::pyr_pub(&op.field_name)
                )));
            }
            identity_ops.push(op);
        } else {
            field_ops.push(op);
        }
    }

    // Route identity writes through `rename_object`.
    for op in &identity_ops {
        apply_identity_rename(op, &mut current, &mut rename_reports)?;
    }

    let field_edit_count = field_ops.len();
    if !field_ops.is_empty() {
        current = splice_edits(&current, &field_ops, uri)?;
    }

    Ok(AppliedSource {
        uri: uri.to_owned(),
        original: source.to_owned(),
        new_source: current,
        rename_reports,
        field_edits: field_edit_count,
    })
}

/// Route one identity-field write through [`rename_object`], updating
/// `current` and recording a [`RenameReport`].
fn apply_identity_rename(
    op: &EditOp,
    current: &mut String,
    rename_reports: &mut Vec<RenameReport>,
) -> Result<(), QueryError> {
    let mut new_path = stringify(&op.new_value);
    if new_path.is_empty() {
        return Err(QueryError::edit(format!(
            "rename target for {} produced an empty value",
            crate::eval::pyr_pub(&op.object_path)
        )));
    }
    // Bare-leaf new values resolve against the existing object's partition +
    // folder context so `.name = "X"` keeps the object in the same partition.
    if !new_path.contains('/') && op.object_path.contains('/') {
        let folder = op.object_path.rsplit_once('/').map_or("", |(head, _)| head);
        new_path = format!("{folder}/{new_path}");
    }
    let report = rename_object(current, &op.object_path, &new_path, &op.object_kind)
        .map_err(QueryError::edit)?;
    if report.occurrences == 0 {
        if op.strict {
            return Err(QueryError::edit(format!(
                "rename of {} matched no source text",
                crate::eval::pyr_pub(&op.object_path)
            )));
        }
        // Tolerant rename: leave the source unchanged, emit no report.
        return Ok(());
    }
    current.clone_from(&report.new_source);
    rename_reports.push(report);
    Ok(())
}

/// Apply one [`PrefixRewrite`] to `source`, honouring its [`Before`] /
/// [`After`] token boundaries manually (the `regex` crate lacks look-around).
///
/// Returns the rewritten text and the replacement count. The scan is
/// non-overlapping and left-to-right: each accepted match advances the cursor
/// past the matched core, so the count is stable for a given input.
fn apply_prefix_rewrite(pr: &PrefixRewrite, source: &str) -> (String, usize) {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut count = 0usize;
    let mut cursor = 0usize;
    while cursor <= source.len() {
        let Some(caps) = pr.pattern.captures_at(source, cursor) else {
            break;
        };
        let m = caps.get(0).unwrap();
        let (start, end) = (m.start(), m.end());
        // Token-boundary checks around the matched core. A boundary failure is
        // not a match (the look-around fails silently and the engine advances
        // one position), so re-search from `start + 1` rather than consuming.
        let before_ok = match pr.before {
            Before::Any => true,
            Before::NotIdent => start == 0 || !is_ident_byte(bytes[start - 1]),
        };
        let after_ok = match pr.after {
            After::Any => true,
            After::RequireNameChar => {
                end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
            }
            After::NotIdent => end == bytes.len() || !is_ident_byte(bytes[end]),
        };
        if !(before_ok && after_ok) {
            // Advance one byte and retry; nothing committed yet.
            let next = if start >= cursor {
                start + 1
            } else {
                cursor + 1
            };
            // Emit up to `next` so the cursor stays a valid char boundary.
            let next = floor_char_boundary(source, next.min(source.len()));
            out.push_str(&source[cursor..next]);
            if next == cursor {
                break;
            }
            cursor = next;
            continue;
        }
        // Accepted match: emit the gap then the expanded replacement.
        out.push_str(&source[cursor..start]);
        let mut dst = String::new();
        caps.expand(&pr.replacement, &mut dst);
        out.push_str(&dst);
        count += 1;
        cursor = if end > start {
            end
        } else {
            // Zero-width match: emit one byte to make progress.
            if end < source.len() {
                let next = floor_char_boundary(source, end + 1);
                out.push_str(&source[end..next]);
                next
            } else {
                end + 1
            }
        };
    }
    if cursor < source.len() {
        out.push_str(&source[cursor..]);
    }
    (out, count)
}

/// Round `idx` down to the nearest UTF-8 char boundary (stable equivalent of
/// `str::floor_char_boundary`).
fn floor_char_boundary(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Port of `edit_plan._stringify`.
fn stringify(value: &Value) -> String {
    match value {
        Value::PathRef(p) => p.full_path.clone(),
        other => describe_str(other),
    }
}

/// `str(value)` for a scalar — `str()` on the assignment value types a
/// rename target produces (string / int / float / bool / None).
fn describe_str(value: &Value) -> String {
    match value {
        Value::Str(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::jsonfmt::py_float_repr(*f),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        Value::Null => "None".to_owned(),
        other => other.describe(),
    }
}

/// Apply non-identity edits to *source*.
///
/// Each op carries either a [`FieldSlot`] (an existing property to overwrite)
/// or targets a missing list field we can materialise into a fresh
/// `<field> { ... }` block. Slots are checked for overlap before application.
fn splice_edits(source: &str, ops: &[&EditOp], uri: &str) -> Result<String, QueryError> {
    let mut placed: Vec<(usize, usize, String, &EditOp)> = Vec::new();
    for op in ops {
        if let Some(slot) = &op.field_slot {
            let new_text = format_value(&op.new_value, &slot.raw_text, &op.field_name, 0)?;
            placed.push((slot.start, slot.end, new_text, op));
            continue;
        }
        if let Some(insert) = materialise_compound_block(source, op)? {
            placed.push(insert);
            continue;
        }
        return Err(QueryError::edit(format!(
            "cannot edit {} on {}: this field has no single-line slot in the \
             source (compound values are not writable in v1)",
            crate::eval::pyr_pub(&op.field_name),
            crate::eval::pyr_pub(&op.object_path)
        )));
    }

    placed.sort_by_key(|p| (p.0, p.1));
    for i in 1..placed.len() {
        let (_, prev_end, _, prev_op) = &placed[i - 1];
        let (start, _, _, op) = &placed[i];
        if start < prev_end {
            return Err(QueryError::edit(format!(
                "overlapping edits at {uri}: {}.{} and {}.{}",
                prev_op.object_path, prev_op.field_name, op.object_path, op.field_name
            )));
        }
    }

    let mut out_parts = String::new();
    let mut cursor = 0usize;
    for (start, end, new_text, _) in &placed {
        out_parts.push_str(&source[cursor..*start]);
        out_parts.push_str(new_text);
        cursor = *end;
    }
    out_parts.push_str(&source[cursor..]);
    Ok(out_parts)
}

/// Render *value* for splicing back into source text.
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_VALUE_WALK_DEPTH`] this returns [`QueryError::Edit`] instead of
/// descending into another nested list (issue #996) — silently truncating
/// the rendered SCF output here would corrupt the spliced source, so an
/// error is the only sound fallback.
///
/// # Errors
/// Returns [`QueryError::Edit`] when a string value contains a character with
/// no safe SCF representation (newlines / braces / control chars), or when
/// `value` nests deeper than [`MAX_VALUE_WALK_DEPTH`].
fn format_value(
    value: &Value,
    original_raw: &str,
    field_name: &str,
    depth: u32,
) -> Result<String, QueryError> {
    if MAX_VALUE_WALK_DEPTH.exceeded(depth) {
        return Err(QueryError::edit(format!(
            "cannot render value for field {}: nests too deeply for SCF output",
            crate::eval::pyr_pub(field_name)
        )));
    }
    match value {
        Value::PathRef(p) => Ok(p.full_path.clone()),
        Value::List(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for v in items {
                parts.push(format_value(v, "", field_name, depth + 1)?);
            }
            let joined = parts.join(" ");
            if original_raw.starts_with('{') {
                Ok(format!("{{ {joined} }}"))
            } else {
                Ok(joined)
            }
        }
        Value::Null => Ok("none".to_owned()),
        Value::Bool(b) => Ok(if *b { "enabled" } else { "disabled" }.to_owned()),
        Value::Str(s) => encode_tmsh_scalar(s, field_name),
        Value::Int(i) => Ok(i.to_string()),
        Value::Float(f) => Ok(crate::jsonfmt::py_float_repr(*f)),
        // `str(value)` fall-through for any other shape.
        other => Ok(other.describe()),
    }
}

/// Encode *value* as an SCF scalar token.
fn encode_tmsh_scalar(value: &str, field_name: &str) -> Result<String, QueryError> {
    if let Some(bad) = forbidden_char(value) {
        let ctx = if field_name.is_empty() {
            String::new()
        } else {
            format!(" for field {}", crate::eval::pyr_pub(field_name))
        };
        return Err(QueryError::edit(format!(
            "value contains character {} that cannot be safely represented in \
             SCF / TMSH{ctx}: newlines, braces, and control characters break \
             the brace-balanced format and would corrupt the surrounding stanza",
            crate::eval::pyr_pub(&bad.to_string())
        )));
    }
    if value.is_empty() {
        return Ok("\"\"".to_owned());
    }
    if requires_quoting(value) {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        return Ok(format!("\"{escaped}\""));
    }
    Ok(value.to_owned())
}

/// Characters with no safe textual representation in an SCF scalar.
fn forbidden_char(value: &str) -> Option<char> {
    value.chars().find(|&c| {
        matches!(c, '\n' | '\r' | '{' | '}')
            || ('\u{00}'..='\u{08}').contains(&c)
            || c == '\u{0b}'
            || c == '\u{0c}'
            || ('\u{0e}'..='\u{1f}').contains(&c)
    })
}

/// Whether *value* must be double-quoted (`[\s"\[\];#]`).
fn requires_quoting(value: &str) -> bool {
    value
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '[' | ']' | ';' | '#'))
}

/// Whether *value* renders cleanly as a bare SCF token.
fn is_flat_list_element(value: &Value) -> bool {
    matches!(
        value,
        Value::Str(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Bool(_)
            | Value::PathRef(_)
            | Value::Null
    )
}

/// Return an edit that overwrites or materialises a `<field> { ... }` block.
fn materialise_compound_block<'a>(
    source: &str,
    op: &'a EditOp,
) -> Result<Option<(usize, usize, String, &'a EditOp)>, QueryError> {
    if !MATERIALISABLE_KIND_FIELDS.contains(&(op.object_kind.as_str(), op.field_name.as_str())) {
        return Ok(None);
    }
    let Some(stanza_slot) = &op.stanza_slot else {
        return Ok(None);
    };
    if op.operator != "+=" && op.operator != "=" {
        return Ok(None);
    }
    let Value::List(new_value) = &op.new_value else {
        return Ok(None);
    };
    if new_value.is_empty() {
        return Ok(None);
    }
    if let Some(bad) = new_value.iter().find(|v| !is_flat_list_element(v)) {
        return Err(QueryError::edit(format!(
            "cannot materialise {} on {}: list element of type {} is not a \
             flat SCF token (materialisation is restricted to scalars and PathRefs)",
            crate::eval::pyr_pub(&op.field_name),
            crate::eval::pyr_pub(&op.object_path),
            bad.type_name()
        )));
    }

    let stanza_start = stanza_slot.start;
    let stanza_end = stanza_slot.end;
    let Some(rel_close) = source[stanza_start..stanza_end].rfind('}') else {
        return Ok(None);
    };
    let closing_brace = stanza_start + rel_close;

    // Sniff indent from the line immediately before the closing brace.
    let line_before_close_start = source[stanza_start..closing_brace]
        .rfind('\n')
        .map_or(stanza_start, |i| stanza_start + i + 1);
    let indent = if line_before_close_start > 0 {
        let prev_line_end = line_before_close_start - 1;
        let prev_line_start = source[stanza_start..prev_line_end]
            .rfind('\n')
            .map_or(stanza_start, |i| stanza_start + i + 1);
        let prev_line = &source[prev_line_start..prev_line_end];
        let trimmed = prev_line.trim_start_matches([' ', '\t']);
        let ind = &prev_line[..prev_line.len() - trimmed.len()];
        if ind.is_empty() {
            "    ".to_owned()
        } else {
            ind.to_owned()
        }
    } else {
        "    ".to_owned()
    };

    let mut items = Vec::with_capacity(new_value.len());
    for v in new_value {
        items.push(format_value(v, "", &op.field_name, 0)?);
    }
    let items_text = items.join(" ");

    // Existing `<field> { ... }` block to overwrite?
    let body_text = &source[stanza_start..stanza_end];
    if let Some((start_in_body, end_in_body)) = find_top_level_block(body_text, &op.field_name) {
        let abs_start = stanza_start + start_in_body;
        let abs_end = stanza_start + end_in_body;
        let new_text = format!("{} {{ {items_text} }}", op.field_name);
        return Ok(Some((abs_start, abs_end, new_text, op)));
    }

    // No existing block — insert before the closing brace.
    let block = format!("{indent}{} {{ {items_text} }}\n", op.field_name);
    Ok(Some((closing_brace, closing_brace, block, op)))
}

/// Locate `<name> { ... }` at the top level of *body*. Returns the byte span
/// covering `<name> { ... }` exactly.
fn find_top_level_block(body: &str, name: &str) -> Option<(usize, usize)> {
    let bytes = body.as_bytes();
    let n = bytes.len();
    let name_bytes = name.as_bytes();
    let first = *name_bytes.first()?;
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < n {
        let ch = bytes[i];
        if ch == b'{' {
            depth += 1;
            i += 1;
            continue;
        }
        if ch == b'}' {
            depth -= 1;
            i += 1;
            continue;
        }
        if ch == b'"' {
            i += 1;
            while i < n && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if depth == 1 && ch == first {
            // Match `\b<name>\s*{` at position i.
            if let Some(end) = match_name_open_brace(body, i, name) {
                // Walk to the matching closing brace.
                let mut inner_depth = 1i32;
                let mut j = end;
                while j < n && inner_depth > 0 {
                    match bytes[j] {
                        b'{' => inner_depth += 1,
                        b'}' => inner_depth -= 1,
                        b'"' => {
                            j += 1;
                            while j < n && bytes[j] != b'"' {
                                if bytes[j] == b'\\' && j + 1 < n {
                                    j += 2;
                                    continue;
                                }
                                j += 1;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if inner_depth == 0 {
                    return Some((i, j));
                }
                return None;
            }
        }
        i += 1;
    }
    None
}

/// Match `\b<name>\s*{` starting at byte *pos* in *body*; return the offset
/// just past the opening brace, or `None`. The `\b` word boundary requires
/// the preceding byte not to be a word character.
fn match_name_open_brace(body: &str, pos: usize, name: &str) -> Option<usize> {
    let bytes = body.as_bytes();
    // `\b` boundary: previous char must not be a word char.
    if pos > 0 {
        let prev = bytes[pos - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return None;
        }
    }
    let name_bytes = name.as_bytes();
    if pos + name_bytes.len() > bytes.len() {
        return None;
    }
    if &bytes[pos..pos + name_bytes.len()] != name_bytes {
        return None;
    }
    let mut j = pos + name_bytes.len();
    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    if j < bytes.len() && bytes[j] == b'{' {
        return Some(j + 1);
    }
    None
}

#[cfg(test)]
mod recursion_tests {
    use super::*;

    /// A list nested `depth` levels deep, wrapping a single `Int(1)` leaf.
    fn deep_nested_list(depth: usize) -> Value {
        let mut v = Value::Int(1);
        for _ in 0..depth {
            v = Value::List(vec![v]);
        }
        v
    }

    /// Run *f* on a worker thread with a 256 MiB stack (mirroring
    /// `tests/hardening.rs`'s `with_big_stack`). Building — and, at scope
    /// end, dropping — a several-thousand-level-deep `Value` costs one
    /// native stack frame per level, since `Value`'s derived `Clone`/`Drop`
    /// have no depth cap of their own (unlike `format_value` under test
    /// here), so the fixture itself needs more room than the harness's
    /// default ~2 MiB per-test thread provides — independently of whether
    /// the guard is correct.
    fn with_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn(f)
            .expect("spawn worker thread")
            .join()
            .expect("worker thread did not panic / overflow")
    }

    /// Regression coverage for issue #996: `format_value` recurses once per
    /// nested `List` level when rendering an assignment value for SCF
    /// splicing, with no depth cap before this fix — reachable via a
    /// `setpath`/`=`-style edit whose right-hand side is a deeply nested
    /// list value. 5000 is comfortably past `MAX_VALUE_WALK_DEPTH` (64);
    /// silently truncating rendered SCF output would corrupt the spliced
    /// source, so the assertion is specifically that the cap surfaces as a
    /// clean [`QueryError::Edit`], not a crash or malformed output.
    #[test]
    fn deeply_nested_list_value_returns_error_not_crash() {
        with_big_stack(|| {
            let value = deep_nested_list(5000);
            let result = format_value(&value, "", "test-field", 0);
            assert!(
                matches!(result, Err(QueryError::Edit(_))),
                "expected a clean Edit error past the depth cap, got {result:?}"
            );
        });
    }

    /// A list nested well under `MAX_VALUE_WALK_DEPTH` still renders exactly
    /// as before this fix.
    #[test]
    fn moderately_nested_list_value_is_unchanged() {
        let value = Value::List(vec![Value::List(vec![Value::Int(1), Value::Int(2)])]);
        let rendered = format_value(&value, "", "test-field", 0).expect("well under the cap");
        assert_eq!(rendered, "1 2");
    }
}
