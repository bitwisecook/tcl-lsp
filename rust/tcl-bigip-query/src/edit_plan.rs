//! Edit plan: collect and apply query-driven source edits.
//!
//! Faithful port of the **field-slot rewrite path** of
//! `dialects/f5/query/edit_plan.py`. Every assignment produced by the
//! evaluator turns into an [`EditOp`]; [`apply`] groups them by source URI,
//! splices each non-identity edit into the source text via its
//! [`FieldSlot`], and returns one [`AppliedSource`] per touched URI.
//!
//! Out of scope (deferred — they route through the rename token-rewrite
//! engine which is not ported): identity-field writes (`.name = ...`) and
//! the `rename*` builtins. The evaluator/applier surface those as a clear
//! [`QueryError::Edit`]; the prefix-cascade (`PrefixRewrite`) branch is
//! likewise omitted.
//!
//! The applier is intentionally text-oriented: it never round-trips through
//! the parser, so comments, whitespace, key order, and unknown stanzas all
//! survive.

use std::collections::HashMap;

use crate::errors::QueryError;
use crate::value::{FieldSlot, Value};

/// Identity fields whose location is the stanza header — writes to these
/// route through the (unported) rename engine and are rejected.
const IDENTITY_FIELDS: &[&str] = &["name", "full-path"];

/// `(object_kind, field_name)` pairs that `+=` / `=` may materialise as a
/// fresh `<field> { ... }` block — flat list slots whose elements stringify
/// to bare tokens. Mirrors `edit_plan._MATERIALISABLE_KIND_FIELDS`.
const MATERIALISABLE_KIND_FIELDS: &[(&str, &str)] = &[
    ("ltm virtual", "rules"),
    ("ltm virtual", "profiles"),
    ("ltm virtual", "persist"),
    ("ltm virtual", "policies"),
];

/// A single (object, field) → new-value edit recorded by the evaluator.
///
/// Port of `edit_plan.EditOp` (field-slot fields only — the `strict` /
/// prefix-rewrite machinery is rename-specific and out of scope).
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
}

/// Collected edits, applied once at the end of a query run — port of
/// `edit_plan.EditPlan` (the `ops` list; prefix rewrites are out of scope).
#[derive(Debug, Default)]
pub struct EditPlan {
    pub ops: Vec<EditOp>,
}

impl EditPlan {
    #[must_use]
    pub fn new() -> Self {
        EditPlan { ops: Vec::new() }
    }

    pub fn add(&mut self, op: EditOp) {
        self.ops.push(op);
    }

    #[must_use]
    pub fn has_edits(&self) -> bool {
        !self.ops.is_empty()
    }
}

/// One source file's result after edits land — port of
/// `edit_plan.AppliedSource` (`rename_reports` always empty in this port).
#[derive(Debug, Clone)]
pub struct AppliedSource {
    pub uri: String,
    pub original: String,
    pub new_source: String,
    pub field_edits: usize,
}

/// Apply every op in *plan* to *sources*, returning one [`AppliedSource`] per
/// touched URI — port of `edit_plan.apply` (field-slot path).
///
/// Identity-field writes are rejected with a clear error (deferred to the
/// rename engine); `+=` / `-=` on an identity field is likewise rejected.
///
/// # Errors
/// Returns [`QueryError::Edit`] for identity-field writes, overlapping edits,
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

    let mut out: HashMap<String, AppliedSource> = HashMap::new();
    for uri in order {
        let ops = &by_uri[&uri];

        // Split identity vs field ops; reject identity-field writes.
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
                return Err(QueryError::edit(
                    "identity-field rewrites / rename are not yet supported in the Rust port",
                ));
            }
            field_ops.push(op);
        }

        let source = sources
            .get(&uri)
            .ok_or_else(|| QueryError::edit(format!("no source loaded for {uri}")))?;
        let field_edit_count = field_ops.len();
        let new_source = splice_edits(source, &field_ops, &uri)?;
        out.insert(
            uri.clone(),
            AppliedSource {
                uri: uri.clone(),
                original: source.clone(),
                new_source,
                field_edits: field_edit_count,
            },
        );
    }
    Ok(out)
}

/// Apply non-identity edits to *source* — port of `edit_plan._splice_edits`.
///
/// Each op carries either a [`FieldSlot`] (an existing property to overwrite)
/// or targets a missing list field we can materialise into a fresh
/// `<field> { ... }` block. Slots are checked for overlap before application.
fn splice_edits(source: &str, ops: &[&EditOp], uri: &str) -> Result<String, QueryError> {
    let mut placed: Vec<(usize, usize, String, &EditOp)> = Vec::new();
    for op in ops {
        if let Some(slot) = &op.field_slot {
            let new_text = format_value(&op.new_value, &slot.raw_text, &op.field_name)?;
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

/// Render *value* for splicing back into source text — port of
/// `edit_plan._format_value`.
///
/// # Errors
/// Returns [`QueryError::Edit`] when a string value contains a character with
/// no safe SCF representation (newlines / braces / control chars).
fn format_value(value: &Value, original_raw: &str, field_name: &str) -> Result<String, QueryError> {
    match value {
        Value::PathRef(p) => Ok(p.full_path.clone()),
        Value::List(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for v in items {
                parts.push(format_value(v, "", field_name)?);
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

/// Encode *value* as an SCF scalar token — port of
/// `edit_plan._encode_tmsh_scalar`.
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

/// Characters with no safe textual representation in an SCF scalar — port of
/// `edit_plan._TMSH_FORBIDDEN_IN_VALUE`.
fn forbidden_char(value: &str) -> Option<char> {
    value.chars().find(|&c| {
        matches!(c, '\n' | '\r' | '{' | '}')
            || ('\u{00}'..='\u{08}').contains(&c)
            || c == '\u{0b}'
            || c == '\u{0c}'
            || ('\u{0e}'..='\u{1f}').contains(&c)
    })
}

/// Whether *value* must be double-quoted — port of
/// `edit_plan._TMSH_REQUIRES_QUOTING` (`[\s"\[\];#]`).
fn requires_quoting(value: &str) -> bool {
    value
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '[' | ']' | ';' | '#'))
}

/// Whether *value* renders cleanly as a bare SCF token — port of
/// `edit_plan._is_flat_list_element`.
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

/// Return an edit that overwrites or materialises a `<field> { ... }` block —
/// port of `edit_plan._materialise_compound_block`.
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
        items.push(format_value(v, "", &op.field_name)?);
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

/// Locate `<name> { ... }` at the top level of *body* — port of
/// `edit_plan._find_top_level_block`. Returns the byte span covering
/// `<name> { ... }` exactly.
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
