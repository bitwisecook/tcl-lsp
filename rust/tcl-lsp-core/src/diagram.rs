//! Control-flow diagram extraction — walk the lowered IR and build the
//! `{events, procedures}` flow tree used to render iRule diagrams.
//!
//! This is the shared, consumer-agnostic home for the diagram shape: the
//! `tcl diagram` CLI verb and the `tcl_lsp_py` `PyO3` facade both build the
//! *same* tree from this one implementation. Callers supply a resolved
//! [`CommandRegistry`]; the only registry dependency is the `DIAGRAM_ACTION`
//! trait (`CommandRegistry::is_diagram_action`).

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value, json};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::expr_ast::{ExprNode, render_expr};
use tcl_compiler::ir::{Procedure, Script, Statement, when_event_name};
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventRegistry;

const MAX_DEPTH: usize = 8;
const MAX_EVENTS: usize = 12;
const MAX_ARG_LEN: usize = 60;

/// Truncate to `limit` characters with a trailing `...`
/// (slices by code point, so this counts/takes `char`s).
fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let head: String = text.chars().take(limit.saturating_sub(3)).collect();
    format!("{head}...")
}

/// Replace `token` with `repl` wherever it is bounded by whitespace on both
/// sides — the consume-free equivalent of a regex substitution that matches the
/// token between whitespace lookarounds (the `regex` crate has no lookbehind, so
/// the surrounding whitespace is asserted by hand and left in place). Scans
/// left-to-right, non-overlapping.
fn replace_ws_bounded(text: &str, token: &str, repl: &str) -> String {
    let bytes = text.as_bytes();
    let tok = token.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let prev_ws = i > 0 && bytes[i - 1].is_ascii_whitespace();
        let matches_tok = bytes[i..].starts_with(tok);
        let next_ws = matches_tok
            && i + tok.len() < bytes.len()
            && bytes[i + tok.len()].is_ascii_whitespace();
        if matches_tok && prev_ws && next_ws {
            out.push_str(repl);
            i += tok.len();
        } else {
            // Preserve UTF-8 by copying whole chars.
            let ch = text[i..].chars().next().expect("char boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Replace a prefix logical `!` (at start or after whitespace, not `!=`) with
/// `not ` — equivalent to a regex substitution that rewrites a `!` at the start
/// or after whitespace (but not before `=`). The captured preceding whitespace is
/// preserved (re-emitted), so this is equivalent to an in-place token rewrite
/// keyed on the original surrounding characters.
fn replace_prefix_not(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'!' {
            let at_start_or_ws = i == 0 || bytes[i - 1].is_ascii_whitespace();
            let not_eq = i + 1 >= bytes.len() || bytes[i + 1] != b'=';
            if at_start_or_ws && not_eq {
                out.push_str("not ");
                i += 1;
                continue;
            }
        }
        let ch = text[i..].chars().next().expect("char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Replace symbolic logical operators with words for Mermaid compatibility:
/// `&&` → `and`, `||` → `or` (both
/// whitespace-bounded), prefix `!` → `not `.
fn diagram_safe_operators(text: &str) -> String {
    let text = replace_ws_bounded(text, "&&", "and");
    let text = replace_ws_bounded(&text, "||", "or");
    replace_prefix_not(&text)
}

/// `_condition_text`: rendered expression text, truncated to 80 and
/// Mermaid-safe.
fn condition_text(node: &ExprNode) -> String {
    diagram_safe_operators(&truncate(&render_expr(node), 80))
}

/// `_is_notable_assign`: an assignment worth showing captures a command
/// substitution.
fn is_notable_assign(value: &str) -> bool {
    value.contains('[')
}

/// Build an `action` flow node (shared by `IRCall` / `IRBarrier`). Mirrors the
/// label / args construction in both `_walk_statement` action branches.
fn action_node(display: &str, args: &[String]) -> Value {
    let arg_strs: Vec<String> = args
        .iter()
        .take(4)
        .map(|a| truncate(a, MAX_ARG_LEN))
        .collect();
    let arg_str = arg_strs.join(" ");
    let label = if arg_str.is_empty() {
        display.to_owned()
    } else {
        format!("{display} {arg_str}").trim().to_owned()
    };
    json!({
        "kind": "action",
        "label": truncate(&label, 80),
        "command": display,
        "args": arg_strs,
    })
}

/// Convert one IR statement to a flow-node dict, or `None` to skip it.
/// One arm per IR statement kind, so the length tracks the statement set.
#[allow(clippy::too_many_lines)]
fn walk_statement(
    stmt: &Statement,
    proc_names: &HashSet<&str>,
    depth: usize,
    registry: &CommandRegistry,
) -> Option<Value> {
    if depth > MAX_DEPTH {
        return Some(json!({ "kind": "truncated", "label": "... (nested logic)" }));
    }

    match stmt {
        Statement::Switch {
            subject,
            arms,
            default_body,
            ..
        } => {
            let mut serialised_arms: Vec<Value> = Vec::new();
            let mut fallthrough_patterns: Vec<String> = Vec::new();
            for arm in arms {
                if arm.fallthrough {
                    fallthrough_patterns.push(arm.pattern.clone());
                    continue;
                }
                let mut patterns = std::mem::take(&mut fallthrough_patterns);
                patterns.push(arm.pattern.clone());
                let body = arm.body.as_ref().map_or_else(Vec::new, |b| {
                    walk_script(b, proc_names, depth + 1, registry)
                });
                let pattern = if patterns.len() > 1 {
                    patterns.join(" | ")
                } else {
                    patterns[0].clone()
                };
                serialised_arms.push(json!({ "pattern": pattern, "body": body }));
            }
            if let Some(body) = default_body {
                let body = walk_script(body, proc_names, depth + 1, registry);
                serialised_arms.push(json!({ "pattern": "default", "body": body }));
            }
            Some(json!({
                "kind": "switch",
                "subject": truncate(subject, 80),
                "arms": serialised_arms,
            }))
        }

        Statement::If {
            clauses, else_body, ..
        } => {
            let mut branches: Vec<Value> = Vec::new();
            for clause in clauses {
                let body = walk_script(&clause.body, proc_names, depth + 1, registry);
                branches.push(json!({
                    "condition": condition_text(&clause.condition),
                    "body": body,
                }));
            }
            if let Some(else_body) = else_body {
                let body = walk_script(else_body, proc_names, depth + 1, registry);
                branches.push(json!({ "condition": "else", "body": body }));
            }
            Some(json!({ "kind": "if", "branches": branches }))
        }

        Statement::For { body, .. } => {
            let child = walk_script(body, proc_names, depth + 1, registry);
            Some(json!({ "kind": "loop", "label": "for", "body": child }))
        }

        Statement::While {
            condition, body, ..
        } => {
            let child = walk_script(body, proc_names, depth + 1, registry);
            Some(json!({
                "kind": "loop",
                "label": format!("while {}", condition_text(condition)),
                "body": child,
            }))
        }

        Statement::Foreach {
            iterators, body, ..
        } => {
            let vars_part = iterators
                .iter()
                .map(|it| it.vars.join(" "))
                .collect::<Vec<_>>()
                .join(", ");
            let child = walk_script(body, proc_names, depth + 1, registry);
            Some(json!({
                "kind": "loop",
                "label": format!("foreach {}", truncate(&vars_part, MAX_ARG_LEN)),
                "body": child,
            }))
        }

        Statement::Call {
            command,
            canonical_command,
            args,
            ..
        } => {
            // Matching is on `IRCall.canonical_command` (the stamped
            // namespace-qualified form). The Rust lowerer only stamps it for
            // alias / namespace resolution, leaving plain calls `None`; for
            // those the source spelling *is* the canonical (modulo a leading
            // `::`, which `is_diagram_action` / the registry strip), so fall
            // back to it to reproduce the resolution.
            let canonical = canonical_command.as_deref().unwrap_or(command.as_str());
            let display = command.as_str();
            // Skip the top-level `when` calls — their bodies are in procedures.
            if canonical == "::when" {
                return None;
            }
            // Procedure calls.
            if proc_names.contains(display) || proc_names.contains(canonical) {
                return Some(json!({
                    "kind": "proc_call",
                    "label": format!("call {display}"),
                    "command": display,
                }));
            }
            // Notable action commands.
            if registry.is_diagram_action(canonical) {
                return Some(action_node(display, args));
            }
            None
        }

        Statement::Barrier {
            command,
            canonical_command,
            args,
            ..
        } => {
            let canonical = canonical_command.as_deref().unwrap_or(command.as_str());
            if registry.is_diagram_action(canonical) {
                return Some(action_node(command, args));
            }
            None
        }

        Statement::Return { value, .. } => {
            let mut label = "return".to_owned();
            if let Some(v) = value
                && !v.is_empty()
            {
                label.push(' ');
                label.push_str(&truncate(v, MAX_ARG_LEN));
            }
            Some(json!({ "kind": "return", "label": label }))
        }

        Statement::AssignConst { name, value, .. } | Statement::AssignValue { name, value, .. }
            if is_notable_assign(value) =>
        {
            Some(json!({ "kind": "assign", "var": name, "value": truncate(value, 80) }))
        }

        Statement::AssignExpr { name, expr, .. } => {
            let text = render_expr(expr);
            if is_notable_assign(&text) {
                Some(json!({ "kind": "assign", "var": name, "value": truncate(&text, 80) }))
            } else {
                None
            }
        }

        Statement::Catch { body, .. } => {
            let child = walk_script(body, proc_names, depth + 1, registry);
            Some(json!({ "kind": "catch", "body": child }))
        }

        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            let child = walk_script(body, proc_names, depth + 1, registry);
            let mut result = Map::new();
            result.insert("kind".to_owned(), json!("try"));
            result.insert("body".to_owned(), json!(child));
            if !handlers.is_empty() {
                let handler_nodes: Vec<Value> = handlers
                    .iter()
                    .map(|h| {
                        json!({
                            "kind_handler": h.kind,
                            "match": h.match_arg,
                            "body": walk_script(&h.body, proc_names, depth + 1, registry),
                        })
                    })
                    .collect();
                result.insert("handlers".to_owned(), Value::Array(handler_nodes));
            }
            if let Some(finally_body) = finally_body {
                result.insert(
                    "finally".to_owned(),
                    json!(walk_script(finally_body, proc_names, depth + 1, registry)),
                );
            }
            Some(Value::Object(result))
        }

        _ => None,
    }
}

/// Walk all statements in a script, dropping the skipped (`None`) nodes.
fn walk_script(
    script: &Script,
    proc_names: &HashSet<&str>,
    depth: usize,
    registry: &CommandRegistry,
) -> Vec<Value> {
    script
        .statements
        .iter()
        .filter_map(|stmt| walk_statement(stmt, proc_names, depth, registry))
        .collect()
}

/// Extract structured `{events, procedures}` flow data from a source, using
/// the caller-supplied registry to classify diagram-action commands.
#[must_use]
pub fn diagram_data(source: &str, registry: &CommandRegistry) -> Value {
    let cu = CompilationUnit::build_for(source, registry, false);
    let module = &cu.ir_module;

    // Recover the source-order dict iteration (the procedures map is a
    // `HashMap`) by sorting on the defining-token offset.
    let mut items: Vec<(&String, &Procedure)> = module.procedures.iter().collect();
    items.sort_by_key(|(_, proc)| proc.span.start());

    let event_procs: Vec<(&String, &Procedure)> = items
        .iter()
        .copied()
        .filter(|(key, _)| key.starts_with("::when::"))
        .collect();
    let regular_procs: Vec<(&String, &Procedure)> = items
        .iter()
        .copied()
        .filter(|(key, _)| !key.starts_with("::when::"))
        .collect();

    // User-defined procedure names for call detection.
    let proc_names: HashSet<&str> = regular_procs
        .iter()
        .map(|(_, proc)| proc.name.as_str())
        .collect();

    let event_registry = EventRegistry::build();

    // Group handlers per event (source order), then stable-sort by priority.
    let mut handlers_by_event: HashMap<String, Vec<&Procedure>> = HashMap::new();
    let mut unique_events: Vec<String> = Vec::new();
    for (_, proc) in &event_procs {
        let event = when_event_name(&proc.qualified_name).to_owned();
        let entry = handlers_by_event.entry(event.clone()).or_default();
        if entry.is_empty() {
            unique_events.push(event);
        }
        entry.push(proc);
    }
    for handlers in handlers_by_event.values_mut() {
        handlers.sort_by_key(|p| p.base_priority);
    }

    // Order events by canonical firing order.
    let ordered = event_registry.order_events(&unique_events);

    let mut events: Vec<Value> = Vec::new();
    'outer: for event_name in ordered {
        if let Some(handlers) = handlers_by_event.get(&event_name) {
            for proc in handlers {
                let flow = walk_script(&proc.body, &proc_names, 0, registry);
                let priority = if proc.base_priority == 500 {
                    Value::Null
                } else {
                    json!(proc.base_priority)
                };
                events.push(json!({
                    "name": event_name,
                    "priority": priority,
                    "multiplicity": event_registry.event_multiplicity(&event_name),
                    "flow": flow,
                }));
                if events.len() >= MAX_EVENTS {
                    break 'outer;
                }
            }
        }
    }

    let procedures: Vec<Value> = regular_procs
        .iter()
        .map(|(_, proc)| {
            let flow = walk_script(&proc.body, &proc_names, 0, registry);
            json!({ "name": proc.name, "params": proc.params, "flow": flow })
        })
        .collect();

    json!({ "events": events, "procedures": procedures })
}
