//! IR serialiser — port of the `ir` half of `tooling/cli/serialise.py`
//! (`_serialise_ir` / `_serialise_script`) + the `stmt_*` / `preview`
//! helpers in `tooling/cli/formatters.py`. Drives `tcl diff --show ir`
//! (and, later, `explore`).
//!
//! Status: feasibility spike. Statement kinds / colour classes / nesting
//! and span-derived ranges + condition/subject/pattern text are ported;
//! the `expr`-text of `assign-expr` / `expr-eval` / `return [expr …]` is
//! sliced from source (the IR stores a parsed `ExprNode`, not the raw
//! text), which is approximate for non-braced forms.

use std::collections::BTreeMap;

use tcl_compiler::ir::{Module as IrModule, Script, Statement};
use tcl_lexer::{LineIndex, Span};
use tcl_registry::snapshot::Json;

/// Escape + truncate text for display — port of `formatters.preview`.
fn preview(text: &str, limit: usize) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    if escaped.chars().count() > limit {
        let head: String = escaped.chars().take(limit.saturating_sub(3)).collect();
        format!("{head}...")
    } else {
        escaped
    }
}

/// Source slice for a span (byte range), or `""` if out of bounds.
fn slice(source: &str, span: Span) -> &str {
    let (s, e) = (span.start() as usize, span.end() as usize);
    if s <= e && e <= source.len() && source.is_char_boundary(s) && source.is_char_boundary(e) {
        &source[s..e]
    } else {
        ""
    }
}

/// Matching closer for an opening delimiter (`_RANGE_CLOSERS`).
fn closer_for(open: u8) -> Option<u8> {
    match open {
        b'{' => Some(b'}'),
        b'[' => Some(b']'),
        b'"' => Some(b'"'),
        _ => None,
    }
}

/// Effective exclusive end offset after `widen_range_for_closer`: a
/// braced/quoted/bracketed word's span omits its closer, so when the span
/// opens with a delimiter and the matching closer immediately follows the
/// (exclusive) end, extend by one to cover it. The explorer/diff pipeline
/// sets a highlight source, so `range_dict` widens every IR range.
fn widened_end(span: Span, source: &str) -> u32 {
    let bytes = source.as_bytes();
    let start = span.start() as usize;
    let end = span.end() as usize;
    if start >= bytes.len() {
        return span.end();
    }
    let Some(closer) = closer_for(bytes[start]) else {
        return span.end();
    };
    // Empty `{}` / `[]` / `""` already ends on its closer — don't widen.
    if end == start + 2 && end - 1 < bytes.len() && bytes[end - 1] == closer {
        return span.end();
    }
    if end < bytes.len() && bytes[end] == closer {
        return span.end() + 1;
    }
    span.end()
}

/// `range_dict` for a span, applying `widen_for_highlight` (braced-word
/// ranges extend to cover the closing delimiter).
fn range_json(span: Span, line_index: &LineIndex, source: &str) -> Json {
    let end_off = widened_end(span, source);
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(end_off, source);
    let mut m = BTreeMap::new();
    m.insert("startLine".to_owned(), Json::Int(i64::from(start.line)));
    m.insert("startCol".to_owned(), Json::Int(i64::from(start.character)));
    m.insert("startOffset".to_owned(), Json::Int(i64::from(span.start())));
    m.insert("endLine".to_owned(), Json::Int(i64::from(end.line)));
    m.insert("endCol".to_owned(), Json::Int(i64::from(end.character)));
    m.insert("endOffset".to_owned(), Json::Int(i64::from(end_off)));
    Json::Object(m)
}

/// The brace/quote-stripped inner text of a condition/word slice — the IR
/// stores conditions as already-stripped text, so a leading delimiter on
/// the source slice (the span omits the closer) is dropped.
fn inner_text(slice: &str) -> &str {
    match slice.as_bytes().first() {
        Some(b'{' | b'[' | b'"') => &slice[1..],
        _ => slice,
    }
}

/// Python IR class name (`stmt_kind`).
fn stmt_kind(stmt: &Statement) -> &'static str {
    match stmt {
        Statement::AssignConst { .. } => "IRAssignConst",
        Statement::AssignExpr { .. } => "IRAssignExpr",
        Statement::AssignValue { .. } => "IRAssignValue",
        Statement::Incr { .. } => "IRIncr",
        Statement::ExprEval { .. } => "IRExprEval",
        Statement::Call { .. } => "IRCall",
        Statement::Return { .. } => "IRReturn",
        Statement::Barrier { .. } => "IRBarrier",
        Statement::Block { .. } => "IRBlock",
        Statement::UpFrame { .. } => "IRUpFrame",
        Statement::If { .. } => "IRIf",
        Statement::For { .. } => "IRFor",
        Statement::While { .. } => "IRWhile",
        Statement::Foreach { .. } => "IRForeach",
        Statement::Catch { .. } => "IRCatch",
        Statement::Try { .. } => "IRTry",
        Statement::Switch { .. } => "IRSwitch",
    }
}

/// CSS colour class (`stmt_color_class`).
fn stmt_color_class(stmt: &Statement) -> &'static str {
    match stmt {
        Statement::Barrier { .. } => "ir-barrier",
        Statement::Call { .. } => "ir-call",
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. } => "ir-assign",
        Statement::Return { .. } => "ir-return",
        Statement::If { .. } | Statement::For { .. } | Statement::Switch { .. } => "ir-control",
        _ => "ir-other",
    }
}

/// Extract the `expr` text of a `set name [expr …]` / `expr …` statement
/// from its source slice (the IR keeps only the parsed `ExprNode`).
fn expr_text(stmt_src: &str) -> String {
    let Some(idx) = stmt_src.find("[expr") else {
        return String::new();
    };
    let after = &stmt_src[idx + "[expr".len()..];
    let inner = after.strip_suffix(']').unwrap_or(after).trim();
    let inner = inner
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(inner);
    inner.to_owned()
}

/// One-line statement summary (`stmt_summary`).
fn stmt_summary(stmt: &Statement, source: &str) -> String {
    match stmt {
        Statement::AssignConst { name, value, .. } => format!("assign-const {name} = {value}"),
        Statement::AssignExpr { name, span, .. } => {
            format!("assign-expr {name} = [expr {{{}}}]", preview(&expr_text(slice(source, *span)), 48))
        }
        Statement::AssignValue { name, value, .. } => {
            format!("assign-value {name} = {}", preview(value, 48))
        }
        Statement::Incr { name, amount, .. } => {
            let mut s = format!("incr {name}");
            if let Some(a) = amount {
                s.push(' ');
                s.push_str(&preview(a, 32));
            }
            s
        }
        Statement::Call { command, args, .. } => {
            let mut rendered: Vec<String> = args.iter().take(4).map(|a| preview(a, 20)).collect();
            if args.len() > 4 {
                rendered.push("...".to_owned());
            }
            let joined = rendered.join(" ");
            if joined.is_empty() {
                format!("call {command}")
            } else {
                format!("call {command} {joined}")
            }
        }
        Statement::Return { value, .. } => value
            .as_ref()
            .map_or_else(|| "return".to_owned(), |v| format!("return {}", preview(v, 48))),
        Statement::Barrier {
            reason, command, ..
        } => {
            let mut s = format!("barrier {reason}");
            if !command.is_empty() {
                s.push_str(" (");
                s.push_str(command);
                s.push(')');
            }
            s
        }
        Statement::If {
            clauses, else_body, ..
        } => format!(
            "if ({} clause(s){})",
            clauses.len(),
            if else_body.is_some() { ", else" } else { "" }
        ),
        Statement::For {
            condition_span, ..
        } => format!("for ({})", preview(inner_text(slice(source, *condition_span)), 40)),
        Statement::Switch { subject, arms, .. } => {
            format!("switch {} ({} arm(s))", preview(subject, 40), arms.len())
        }
        other => stmt_kind(other).to_owned(),
    }
}

/// A child node `{label, range, body}` for control-flow statements.
fn child(label: String, range: Option<Json>, body: Json) -> Json {
    let mut m = BTreeMap::new();
    m.insert("label".to_owned(), Json::s(label));
    m.insert("range".to_owned(), range.unwrap_or(Json::Null));
    m.insert("body".to_owned(), body);
    Json::Object(m)
}

/// The `children` array for a control-flow statement (`if`/`for`/`switch`),
/// or `None` for flat statements (port of the `_serialise_script` branches).
fn children_json(stmt: &Statement, line_index: &LineIndex, source: &str) -> Option<Json> {
    let mut children = Vec::new();
    match stmt {
        Statement::If {
            clauses,
            else_body,
            else_span,
            ..
        } => {
            for (i, clause) in clauses.iter().enumerate() {
                children.push(child(
                    format!(
                        "clause {}: {}",
                        i + 1,
                        preview(inner_text(slice(source, clause.condition_span)), 60)
                    ),
                    Some(range_json(clause.condition_span, line_index, source)),
                    serialise_script(&clause.body, line_index, source),
                ));
            }
            if let Some(body) = else_body {
                children.push(child(
                    "else".to_owned(),
                    else_span.map(|s| range_json(s, line_index, source)),
                    serialise_script(body, line_index, source),
                ));
            }
        }
        Statement::For {
            init,
            init_span,
            condition_span,
            next,
            next_span,
            body,
            body_span,
            ..
        } => {
            children.push(child(
                "init".to_owned(),
                Some(range_json(*init_span, line_index, source)),
                serialise_script(init, line_index, source),
            ));
            children.push(child(
                format!(
                    "condition: {}",
                    preview(inner_text(slice(source, *condition_span)), 60)
                ),
                Some(range_json(*condition_span, line_index, source)),
                Json::Array(Vec::new()),
            ));
            children.push(child(
                "next".to_owned(),
                Some(range_json(*next_span, line_index, source)),
                serialise_script(next, line_index, source),
            ));
            children.push(child(
                "body".to_owned(),
                Some(range_json(*body_span, line_index, source)),
                serialise_script(body, line_index, source),
            ));
        }
        Statement::Switch {
            arms,
            default_body,
            default_span,
            ..
        } => {
            for arm in arms {
                let label = format!(
                    "{}: {}",
                    if arm.fallthrough { "fallthrough" } else { "arm" },
                    preview(&arm.pattern, 48)
                );
                let body = arm.body.as_ref().map_or_else(
                    || Json::Array(Vec::new()),
                    |b| serialise_script(b, line_index, source),
                );
                children.push(child(
                    label,
                    Some(range_json(arm.pattern_span, line_index, source)),
                    body,
                ));
            }
            if let Some(body) = default_body {
                children.push(child(
                    "default".to_owned(),
                    default_span.map(|s| range_json(s, line_index, source)),
                    serialise_script(body, line_index, source),
                ));
            }
        }
        _ => return None,
    }
    Some(Json::Array(children))
}

/// Serialise one statement to its node dict (port of the `_serialise_script`
/// body), recursing into control-flow children.
fn statement_json(stmt: &Statement, line_index: &LineIndex, source: &str) -> Json {
    let mut m = BTreeMap::new();
    m.insert("kind".to_owned(), Json::s(stmt_kind(stmt)));
    m.insert("summary".to_owned(), Json::s(stmt_summary(stmt, source)));
    m.insert("colorClass".to_owned(), Json::s(stmt_color_class(stmt)));
    m.insert("range".to_owned(), range_json(*stmt_span(stmt), line_index, source));
    if let Some(children) = children_json(stmt, line_index, source) {
        m.insert("children".to_owned(), children);
    }
    Json::Object(m)
}

/// Whole-statement span accessor (every variant carries `span`).
fn stmt_span(stmt: &Statement) -> &Span {
    match stmt {
        Statement::AssignConst { span, .. }
        | Statement::AssignExpr { span, .. }
        | Statement::AssignValue { span, .. }
        | Statement::Incr { span, .. }
        | Statement::ExprEval { span, .. }
        | Statement::Call { span, .. }
        | Statement::Return { span, .. }
        | Statement::Barrier { span, .. }
        | Statement::Block { span, .. }
        | Statement::UpFrame { span, .. }
        | Statement::If { span, .. }
        | Statement::For { span, .. }
        | Statement::While { span, .. }
        | Statement::Foreach { span, .. }
        | Statement::Catch { span, .. }
        | Statement::Try { span, .. }
        | Statement::Switch { span, .. } => span,
    }
}

/// Serialise a script (list of statements).
fn serialise_script(script: &Script, line_index: &LineIndex, source: &str) -> Json {
    Json::Array(
        script
            .statements
            .iter()
            .map(|s| statement_json(s, line_index, source))
            .collect(),
    )
}

/// The `ir` payload (`{topLevel, procedures}`) — port of `_serialise_ir`.
#[must_use]
pub fn serialise_ir(ir_module: &IrModule, line_index: &LineIndex, source: &str) -> Json {
    let mut procs = BTreeMap::new();
    let mut names: Vec<&String> = ir_module.procedures.keys().collect();
    names.sort();
    for qname in names {
        let proc = &ir_module.procedures[qname];
        let mut entry = BTreeMap::new();
        entry.insert(
            "params".to_owned(),
            Json::Array(proc.params.iter().map(|p| Json::s(p.clone())).collect()),
        );
        entry.insert("range".to_owned(), range_json(proc.span, line_index, source));
        entry.insert(
            "body".to_owned(),
            serialise_script(&proc.body, line_index, source),
        );
        procs.insert(qname.clone(), Json::Object(entry));
    }
    let mut root = BTreeMap::new();
    root.insert(
        "topLevel".to_owned(),
        serialise_script(&ir_module.top_level, line_index, source),
    );
    root.insert("procedures".to_owned(), Json::Object(procs));
    Json::Object(root)
}
