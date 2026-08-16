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
use tcl_compiler::ir::{
    CommandTokens, Procedure, Script, Statement, SwitchArm, TryHandler, when_event_name,
};
use tcl_compiler::registry_invocation::effective_command_arguments;
use tcl_registry::CommandRegistry;
use tcl_registry::InvocationArguments;
use tcl_registry::dialects::DialectSet;
use tcl_registry::events::EventRegistry;
use tcl_registry::registry::{
    ExactInvocationCompletion, InvocationCompletion, InvocationCompletionKnowledge,
};

const MAX_DEPTH: usize = 8;
const MAX_EVENTS: usize = 12;
const MAX_ARG_LEN: usize = 60;

/// A statically-known Tcl completion carried by the diagram JSON contract.
///
/// This is deliberately smaller than Tcl's complete result/options triple:
/// diagrams only need to know whether a path can continue after a `try`'s
/// `finally` body.  `normal` is omitted from the JSON for compatibility;
/// every other value is emitted as a node's `completion` field.  Unknown
/// invocations stay normal in this structural projection rather than being
/// guessed to be an error or an exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagramCompletion {
    Normal,
    Error,
    Return,
    Break,
    Continue,
    Dynamic,
    DynamicReturnOrError,
    ExactCustom(i32),
    ProcessExit,
    Terminal,
}

impl DiagramCompletion {
    const fn as_str(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Error => Some("error"),
            Self::Return => Some("return"),
            Self::Break => Some("break"),
            Self::Continue => Some("continue"),
            Self::Dynamic => Some("dynamic"),
            Self::DynamicReturnOrError => Some("dynamic_return_or_error"),
            Self::ExactCustom(_) => Some("custom"),
            Self::ProcessExit => Some("process_exit"),
            Self::Terminal => Some("terminal"),
        }
    }
}

/// The exact registry completion for a concrete command, if one is known.
///
/// Keep this at the IR → diagram boundary.  Rendering must not infer that a
/// command called `error`, `return`, or `break` has a particular effect: an
/// alias is already recorded in `canonical_command`, and the registry owns
/// the effective completion descriptor and terminating traits.
fn command_completion(
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
    registry: &CommandRegistry,
) -> DiagramCompletion {
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let effective = tokens.map(|tokens| {
        effective_command_arguments(
            tokens,
            tcl_dialect::EscapeSyntax::of_dialect_name(
                registry.profile().map(|profile| profile.name),
            ),
        )
    });
    let knowledge = if let Some(effective) = effective.as_ref() {
        let words: Vec<_> = effective
            .iter()
            .map(|word| word.as_registry_word())
            .collect();
        registry.invocation_completion_knowledge(
            command,
            InvocationArguments::structured(&words),
            DialectSet::empty(),
        )
    } else {
        registry
            .exact_invocation_completion(command, &arg_refs, DialectSet::empty())
            .map(InvocationCompletionKnowledge::Exact)
    };
    if let Some(InvocationCompletionKnowledge::Exact(completion)) = knowledge {
        return match completion {
            ExactInvocationCompletion::Tcl(tcl_registry::CompletionCode::Ok) => {
                DiagramCompletion::Normal
            }
            ExactInvocationCompletion::Tcl(tcl_registry::CompletionCode::Error) => {
                DiagramCompletion::Error
            }
            ExactInvocationCompletion::Tcl(tcl_registry::CompletionCode::Return) => {
                DiagramCompletion::Return
            }
            ExactInvocationCompletion::Tcl(tcl_registry::CompletionCode::Break) => {
                DiagramCompletion::Break
            }
            ExactInvocationCompletion::Tcl(tcl_registry::CompletionCode::Continue) => {
                DiagramCompletion::Continue
            }
            ExactInvocationCompletion::Tcl(tcl_registry::CompletionCode::Other(code)) => {
                DiagramCompletion::ExactCustom(code)
            }
            ExactInvocationCompletion::ProcessExit => DiagramCompletion::ProcessExit,
        };
    }
    if let Some(knowledge) = knowledge {
        return match knowledge {
            InvocationCompletionKnowledge::Exact(_) => unreachable!("handled above"),
            InvocationCompletionKnowledge::DynamicReturnOrError => {
                DiagramCompletion::DynamicReturnOrError
            }
            InvocationCompletionKnowledge::Dynamic => DiagramCompletion::Dynamic,
        };
    }
    match registry.invocation_completion(command, &arg_refs, DialectSet::empty()) {
        InvocationCompletion::ReturnsResult(_) => DiagramCompletion::Return,
        InvocationCompletion::Terminates => DiagramCompletion::Terminal,
        // Dynamic completion options are not enough evidence to draw a
        // terminal path.  Leave such calls as ordinary structural actions.
        InvocationCompletion::FallsThrough | InvocationCompletion::Unknown => {
            DiagramCompletion::Normal
        }
    }
}

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

/// An assignment worth showing captures a command substitution.
fn is_notable_assign(value: &str) -> bool {
    value.contains('[')
}

/// Build an `action` flow node (shared by `Statement::Call` /
/// `Statement::Barrier`). Mirrors the label / args construction in both
/// `walk_statement` action branches.
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

/// Add the optional completion field to an action-like node.
fn with_completion(mut node: Value, completion: DiagramCompletion) -> Value {
    if let Some(completion_str) = completion.as_str()
        && let Some(object) = node.as_object_mut()
    {
        object.insert("completion".to_owned(), json!(completion_str));
        if let DiagramCompletion::ExactCustom(code) = completion {
            object.insert("completion_code".to_owned(), json!(code));
        }
    }
    node
}

/// Build the `switch` flow-node dict from its subject, arms and default body.
fn walk_switch(
    subject: &str,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    proc_names: &HashSet<&str>,
    depth: usize,
    registry: &CommandRegistry,
) -> Value {
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
    json!({
        "kind": "switch",
        "subject": truncate(subject, 80),
        "arms": serialised_arms,
    })
}

/// Build the `try` flow-node dict from its body, handlers and finally body.
fn walk_try(
    body: &Script,
    handlers: &[TryHandler],
    finally_body: Option<&Script>,
    proc_names: &HashSet<&str>,
    depth: usize,
    registry: &CommandRegistry,
) -> Value {
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
                    "completion_code": handler_completion_code(h, registry),
                    "trap_pattern": h.trap_pattern,
                    "fallthrough": h.fallthrough,
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
    Value::Object(result)
}

/// Canonicalise an `on` selector in the shared projection.  Tcl accepts its
/// integer grammar here (including `02`, `+2`, and `0x2`); renderers must not
/// reimplement it. Unknown symbolic selectors cannot be completion codes.
fn handler_completion_code(handler: &TryHandler, registry: &CommandRegistry) -> Option<i32> {
    if handler.kind != "on" {
        return None;
    }
    match handler.match_arg.as_str() {
        "ok" => Some(0),
        "error" => Some(1),
        "return" => Some(2),
        "break" => Some(3),
        "continue" => Some(4),
        value => registry.profile().and_then(|profile| {
            tcl_registry::completion::canonical_completion_code(
                value,
                tcl_syntax::number::Numbers::of_profile(Some(profile)),
            )
        }),
    }
}

/// Build the `if` flow-node dict from its clauses and optional `else` body.
fn walk_if(
    clauses: &[tcl_compiler::ir::IfClause],
    else_body: Option<&Script>,
    proc_names: &HashSet<&str>,
    depth: usize,
    registry: &CommandRegistry,
) -> Value {
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
    json!({ "kind": "if", "branches": branches })
}

/// Build the flow node for a `Call` statement, or `None` when it isn't notable.
fn walk_call(
    command: &str,
    canonical_command: Option<&str>,
    args: &[String],
    tokens: Option<&CommandTokens>,
    proc_names: &HashSet<&str>,
    registry: &CommandRegistry,
) -> Option<Value> {
    // Matching is on `Statement::Call.canonical_command` (the stamped
    // namespace-qualified form). The lowerer only stamps it for
    // alias / namespace resolution, leaving plain calls `None`; for
    // those the source spelling *is* the canonical (modulo a leading
    // `::`, which `is_diagram_action` / the registry strip), so fall
    // back to it to reproduce the resolution.
    let canonical = canonical_command.unwrap_or(command);
    let display = command;
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
    let completion = command_completion(canonical, args, tokens, registry);
    // Keep an exact non-normal completion visible even when it is not a
    // diagram action (for example `error` / `throw`), so a surrounding try
    // can route it through `finally`.  Ordinary unknown calls remain omitted
    // unless the registry marks them as a diagram action.
    if registry.is_diagram_action(canonical) || completion != DiagramCompletion::Normal {
        return Some(with_completion(action_node(display, args), completion));
    }
    None
}

/// Build a `loop` flow node with the given rendered label and exit reason.
fn loop_node(
    label: &str,
    exit: &str,
    body: &Script,
    proc_names: &HashSet<&str>,
    depth: usize,
    registry: &CommandRegistry,
) -> Value {
    let child = walk_script(body, proc_names, depth + 1, registry);
    json!({ "kind": "loop", "label": label, "exit": exit, "body": child })
}

/// Project the compiler's structured loop variants into one loop flow node.
fn walk_loop_statement(
    stmt: &Statement,
    proc_names: &HashSet<&str>,
    depth: usize,
    registry: &CommandRegistry,
) -> Option<Value> {
    match stmt {
        Statement::For { body, .. } => {
            Some(loop_node("for", "false", body, proc_names, depth, registry))
        }
        Statement::While {
            condition, body, ..
        } => {
            let label = format!("while {}", condition_text(condition));
            Some(loop_node(
                &label, "false", body, proc_names, depth, registry,
            ))
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            let vars_part = iterators
                .iter()
                .map(|it| it.vars.join(" "))
                .collect::<Vec<_>>()
                .join(", ");
            let label = format!("foreach {}", truncate(&vars_part, MAX_ARG_LEN));
            Some(loop_node(
                &label,
                "exhausted",
                body,
                proc_names,
                depth,
                registry,
            ))
        }
        _ => None,
    }
}

/// Flow node for a notable assignment, or `None` when the value isn't notable.
fn assign_node(name: &str, value: &str) -> Option<Value> {
    is_notable_assign(value)
        .then(|| json!({ "kind": "assign", "var": name, "value": truncate(value, 80) }))
}

/// Build the `return` flow node, appending the returned value when present.
fn return_node(value: Option<&String>) -> Value {
    let mut label = "return".to_owned();
    if let Some(v) = value
        && !v.is_empty()
    {
        label.push(' ');
        label.push_str(&truncate(v, MAX_ARG_LEN));
    }
    json!({ "kind": "return", "label": label, "completion": "return" })
}

/// Convert one IR statement to a flow-node dict, or `None` to skip it.
/// One arm per IR statement kind, so the length tracks the statement set.
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
        } => Some(walk_switch(
            subject,
            arms,
            default_body.as_ref(),
            proc_names,
            depth,
            registry,
        )),

        Statement::If {
            clauses, else_body, ..
        } => Some(walk_if(
            clauses,
            else_body.as_ref(),
            proc_names,
            depth,
            registry,
        )),

        Statement::For { .. } | Statement::While { .. } | Statement::Foreach { .. } => {
            walk_loop_statement(stmt, proc_names, depth, registry)
        }

        Statement::Call {
            command,
            canonical_command,
            args,
            tokens,
            ..
        } => walk_call(
            command,
            canonical_command.as_deref(),
            args,
            tokens.as_ref(),
            proc_names,
            registry,
        ),

        Statement::Barrier {
            command,
            canonical_command,
            args,
            tokens,
            ..
        } => {
            let canonical = canonical_command.as_deref().unwrap_or(command.as_str());
            let completion = command_completion(canonical, args, tokens.as_ref(), registry);
            (registry.is_diagram_action(canonical) || completion != DiagramCompletion::Normal)
                .then(|| with_completion(action_node(command, args), completion))
        }

        Statement::Return { value, .. } => Some(return_node(value.as_ref())),

        Statement::AssignConst { name, value, .. } | Statement::AssignValue { name, value, .. } => {
            assign_node(name, value)
        }

        Statement::AssignExpr { name, expr, .. } => assign_node(name, &render_expr(expr)),

        Statement::Catch { body, .. } => {
            let child = walk_script(body, proc_names, depth + 1, registry);
            Some(json!({ "kind": "catch", "body": child }))
        }

        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => Some(walk_try(
            body,
            handlers,
            finally_body.as_ref(),
            proc_names,
            depth,
            registry,
        )),

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
/// the caller-supplied registry to classify diagram-action commands. Parses as
/// plain Tcl; use [`diagram_data_for_dialect`] for a dialect (e.g. `f5-irules`,
/// so an iRule's `}{` control flow is parsed correctly).
#[must_use]
pub fn diagram_data(source: &str, registry: &CommandRegistry) -> Value {
    diagram_data_for_dialect(source, registry, "")
}

/// [`diagram_data`] for an explicit dialect. `registry` must be that
/// dialect's registry.
#[must_use]
pub fn diagram_data_for_dialect(source: &str, registry: &CommandRegistry, dialect: &str) -> Value {
    let cu = CompilationUnit::build_for_dialect(source, registry, false, dialect);
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

#[cfg(test)]
mod tests {
    use super::diagram_data_for_dialect;
    use serde_json::{Value, json};
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[test]
    fn loop_nodes_carry_their_compiler_exit_reason() {
        let source = r"
            proc loops {} {
                while {$ready} { set seen [clock seconds] }
                for {set i 0} {$i < 1} {incr i} { set seen [clock seconds] }
                foreach item $items { set seen [clock seconds] }
            }
        ";
        let data = diagram_data_for_dialect(
            source,
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        let flow = data
            .pointer("/procedures/0/flow")
            .and_then(serde_json::Value::as_array)
            .expect("procedure flow");
        let exits: Vec<_> = flow
            .iter()
            .map(|node| node.get("exit").and_then(serde_json::Value::as_str))
            .collect();
        assert_eq!(exits, vec![Some("false"), Some("false"), Some("exhausted")]);
    }

    #[test]
    fn try_projection_carries_exact_completions_and_handler_fallthrough() {
        let data = diagram_data_for_dialect(
            r"
                proc paths {} {
                    try { error boom } on error message - on return value {
                        set recovered [clock seconds]
                    } finally {
                        set cleaned [clock seconds]
                    }
                    set after [clock seconds]
                }
            ",
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        let try_node = data.pointer("/procedures/0/flow/0").expect("try node");
        assert_eq!(try_node["kind"], "try");
        assert_eq!(try_node["body"][0]["completion"], "error");
        assert_eq!(try_node["handlers"][0]["fallthrough"], true);
        assert_eq!(try_node["handlers"][1]["fallthrough"], false);
        assert_eq!(try_node["finally"][0]["kind"], "assign");
        assert_eq!(try_node["finally"][0].get("completion"), None);

        // The following action remains in the projection; it is the renderer
        // that selects it only for the normal/handled completion paths.
        let flow = data
            .pointer("/procedures/0/flow")
            .and_then(Value::as_array)
            .unwrap();
        assert_eq!(flow[1]["value"], "[clock seconds]");
    }

    #[test]
    fn return_projection_is_an_explicit_completion_not_a_renderer_guess() {
        let data = diagram_data_for_dialect(
            "proc paths {} { try { return stop } finally { set cleaned [clock seconds] } }",
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/0/body/0/completion"),
            Some(&Value::String("return".to_owned()))
        );

        let overridden = diagram_data_for_dialect(
            "proc paths {} { try { set done [clock seconds] } finally { return stop } }",
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        assert_eq!(
            overridden.pointer("/procedures/0/flow/0/finally/0/completion"),
            Some(&Value::String("return".to_owned()))
        );
    }

    #[test]
    fn literal_return_options_and_exit_keep_distinct_try_outcomes() {
        let data = diagram_data_for_dialect(
            r"
                proc paths {} {
                    try { return -code error payload } finally { set cleaned [clock seconds] }
                    try { return -level 0 -code error payload } finally { set cleaned [clock seconds] }
                    try { exit 0 } finally { set never [clock seconds] }
                }
            ",
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        // Tcl's default `return -level 1` is itself TCL_RETURN inside the
        // enclosing try; only level 0 exposes the configured error code.
        assert_eq!(
            data.pointer("/procedures/0/flow/0/body/0/completion"),
            Some(&Value::String("return".to_owned()))
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/1/body/0/completion"),
            Some(&Value::String("error".to_owned()))
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/2/body/0/completion"),
            Some(&Value::String("process_exit".to_owned()))
        );
    }

    #[test]
    fn dynamic_return_forms_keep_their_distinct_possible_completions() {
        let data = diagram_data_for_dialect(
            r"
                proc paths {} {
                    try { return -code $code payload } finally { set cleaned yes }
                    try { return -level 0 -code $code payload } finally { set cleaned yes }
                    try { return -options $options payload } finally { set cleaned yes }
                }
            ",
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/0/body/0/completion"),
            Some(&Value::String("dynamic_return_or_error".to_owned()))
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/1/body/0/completion"),
            Some(&Value::String("dynamic".to_owned()))
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/2/body/0/completion"),
            Some(&Value::String("dynamic".to_owned()))
        );
    }

    #[test]
    fn literal_invalid_return_forms_project_as_catchable_errors() {
        let source = r"
            proc paths {} {
                try { return -code bogus payload } on error {m o} {}
                try { return -level -1 payload } on error {m o} {}
            }
        ";
        for dialect in ["tcl8.6", "tcl9.0"] {
            let data = diagram_data_for_dialect(
                source,
                tcl_registry::registry_for_dialect(dialect),
                dialect,
            );
            for index in 0..2 {
                assert_eq!(
                    data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                    Some(&Value::String("error".to_owned())),
                    "{dialect}: {data}"
                );
            }
        }
    }

    #[test]
    fn trailing_return_option_words_are_results_not_missing_values() {
        let source = r"
            proc paths {} {
                try { return -code } on return {m o} {}
                try { return -level } on return {m o} {}
                try { return -options } on return {m o} {}
            }
        ";
        for dialect in ["tcl8.6", "tcl9.0"] {
            let data = diagram_data_for_dialect(
                source,
                tcl_registry::registry_for_dialect(dialect),
                dialect,
            );
            for index in 0..3 {
                assert_eq!(
                    data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                    Some(&Value::String("return".to_owned())),
                    "{dialect}: {data}"
                );
            }
        }
    }

    #[test]
    fn invalid_known_terminator_arity_projects_as_catchable_error() {
        let source = r"
            proc paths {} {
                try { error } on error {m o} { set caught_error yes }
                try { throw TYPE } on error {m o} { set caught_throw yes }
                try { break extra } on error {m o} { set caught_break yes }
                try { exit 0 extra } on error {m o} { set caught_exit yes }
            }
        ";
        for dialect in ["tcl8.6", "tcl9.0"] {
            let data = diagram_data_for_dialect(
                source,
                tcl_registry::registry_for_dialect(dialect),
                dialect,
            );
            for index in 0..4 {
                assert_eq!(
                    data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                    Some(&Value::String("error".to_owned())),
                    "{dialect}: {data}"
                );
            }
        }
    }

    #[test]
    fn return_completion_preserves_source_word_shape() {
        let source = r#"
            proc paths {} {
                try { return -code {$code} } on error {m o} {}
                try { return -code $code } on error {m o} {}
                try { return -code "$code" } on error {m o} {}
                try { return -code \x32 } on return {m o} {}
                try { return -code {\x32} } on error {m o} {}
            }
        "#;
        for dialect in ["tcl8.6", "tcl9.0"] {
            let data = diagram_data_for_dialect(
                source,
                tcl_registry::registry_for_dialect(dialect),
                dialect,
            );
            let completions = [
                "error",
                "dynamic_return_or_error",
                "dynamic_return_or_error",
                "return",
                "error",
            ];
            for (index, expected) in completions.into_iter().enumerate() {
                assert_eq!(
                    data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                    Some(&Value::String(expected.to_owned())),
                    "{dialect}: {data}"
                );
            }
        }
    }

    #[test]
    fn numeric_return_codes_project_as_their_canonical_completions() {
        let data = diagram_data_for_dialect(
            "proc paths {} { try { return -level 0 -code 00 x } finally {} ; try { return -level 0 -code +1 x } finally {} ; try { return -level 0 -code 02 x } finally {} ; try { return -level 0 -code 0x3 x } finally {} ; try { return -level 0 -code 04 x } finally {} }",
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        let expected = [
            None,
            Some("error"),
            Some("return"),
            Some("break"),
            Some("continue"),
        ];
        for (index, completion) in expected.into_iter().enumerate() {
            assert_eq!(
                data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                completion
                    .map(|value| Value::String(value.to_owned()))
                    .as_ref(),
                "flow {index}"
            );
        }
    }

    #[test]
    fn exact_custom_return_codes_keep_their_integer_payload() {
        let data = diagram_data_for_dialect(
            "proc paths {} { try { return -level 0 -code 42 x } finally {} ; try { return -level 0 -code 4294967295 x } finally {} }",
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/0/body/0/completion"),
            Some(&Value::String("custom".to_owned()))
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/0/body/0/completion_code"),
            Some(&Value::from(42))
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/1/body/0/completion_code"),
            Some(&Value::from(-1))
        );
    }

    #[test]
    fn handler_completion_codes_use_the_dialect_numeric_grammar() {
        let source = "proc paths {} { try { return -options $options payload } on 02 {message options} { set two yes } on +2 {message options} { set duplicate yes } on 0x2 {message options} { set duplicate_hex yes } on return {message options} { set symbolic yes } on 2147483648 {message options} { set wrapped_min yes } on 4294967295 {message options} { set wrapped_minus_one yes } on -2147483649 {message options} { set invalid_low yes } on nonsense {message options} { set invalid yes } }";
        // `try` itself begins in Tcl 8.6, so diagram handlers can only be
        // projected for releases that provide the command.
        for dialect in ["tcl8.6", "tcl9.0"] {
            let data = diagram_data_for_dialect(
                source,
                tcl_registry::registry_for_dialect(dialect),
                dialect,
            );
            let handlers = data
                .pointer("/procedures/0/flow/0/handlers")
                .and_then(Value::as_array)
                .expect("try handlers");
            let codes: Vec<_> = handlers
                .iter()
                .map(|handler| {
                    handler
                        .get("completion_code")
                        .cloned()
                        .unwrap_or(Value::Null)
                })
                .collect();
            assert_eq!(
                codes,
                vec![
                    Value::from(2),
                    Value::from(2),
                    Value::from(2),
                    Value::from(2),
                    Value::from(i32::MIN),
                    Value::from(-1),
                    Value::Null,
                    Value::Null
                ],
                "{dialect}"
            );
        }
    }

    #[test]
    fn trap_patterns_are_projected_as_tcl_list_elements() {
        let data = diagram_data_for_dialect(
            r#"proc paths {} { try { error x } trap {A \$B} {message options} {} trap {A {$B}} {message options} {} trap {A \[B\]} {message options} {} trap {A {[B]}} {message options} {} trap {A C:\\tmp} {message options} {} trap {A {C:\tmp}} {message options} {} trap "A \{" {message options} {} trap $pattern {message options} {} trap "A $pattern" {message options} {} trap [list A B] {message options} {} }"#,
            tcl_registry::registry_for_dialect("tcl8.6"),
            "tcl8.6",
        );
        let expected = [
            json!(["A", "$B"]),
            json!(["A", "$B"]),
            json!(["A", "[B]"]),
            json!(["A", "[B]"]),
            json!(["A", r"C:\tmp"]),
            json!(["A", r"C:\tmp"]),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ];
        for (index, expected) in expected.iter().enumerate() {
            assert_eq!(
                data.pointer(&format!(
                    "/procedures/0/flow/0/handlers/{index}/trap_pattern"
                )),
                Some(expected),
                "handler {index}: {data}"
            );
        }
    }

    #[test]
    fn trap_pattern_projection_preserves_the_dialect_escape_value() {
        let source =
            r#"proc paths {} { try { error x } trap "A \U0001F600" {message options} {} }"#;
        for (dialect, expected) in [("tcl8.6", "\u{fffd}"), ("tcl9.0", "😀")] {
            let data = diagram_data_for_dialect(
                source,
                tcl_registry::registry_for_dialect(dialect),
                dialect,
            );
            assert_eq!(
                data.pointer("/procedures/0/flow/0/handlers/0/trap_pattern"),
                Some(&json!(["A", expected])),
                "{dialect}: {data}",
            );
        }
    }

    #[test]
    fn handler_number_release_vectors_share_tcl_numeric_owner() {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let profile = tcl_registry::registry_for_dialect(dialect)
                .profile()
                .expect("dialect profile");
            let numbers = tcl_syntax::number::Numbers::of_profile(Some(profile));
            assert_eq!(numbers.parse_wide("02"), Some(2), "{dialect}");
            assert_eq!(numbers.parse_wide("+2"), Some(2), "{dialect}");
            assert_eq!(numbers.parse_wide("0x2"), Some(2), "{dialect}");
            assert_eq!(
                tcl_registry::completion::canonical_completion_code("2147483648", numbers),
                Some(i32::MIN),
                "{dialect}"
            );
            assert_eq!(
                tcl_registry::completion::canonical_completion_code("4294967295", numbers),
                Some(-1),
                "{dialect}"
            );
            assert_eq!(
                tcl_registry::completion::canonical_completion_code("-2147483649", numbers),
                None,
                "{dialect}"
            );
            assert_eq!(
                tcl_registry::completion::canonical_completion_code("9223372036854775808", numbers),
                None,
                "{dialect}"
            );
        }
    }

    #[test]
    fn real_tcl_oracle_distinguishes_default_and_level_zero_return_codes() {
        let script = r"
            proc probe {spec} {
                set log {}
                try { return {*}$spec payload } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_dynamic_code {code} {
                set log {}
                try { return -code $code payload } \
                    on ok {message options} { lappend log ok } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    on break {message options} { lappend log break } \
                    on continue {message options} { lappend log continue } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_dynamic_options {options} {
                set log {}
                try { return -options $options payload } \
                    on ok {message options} { lappend log ok } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    on break {message options} { lappend log break } \
                    on continue {message options} { lappend log continue } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_source_order {} {
                set log {}
                try { return -options {-code error -level 0 -errorcode {X Y}} payload } \
                    trap {X} {message options} { lappend log broad-trap } \
                    trap {X Y} {message options} { lappend log overlapping-trap } \
                    on error {message options} { lappend log first-error } \
                    on error {message options} { lappend log duplicate-error } \
                    finally { lappend log finally }
                return $log
            }
            proc probe_custom_code {code} {
                set log {}
                try { return -options [list -code $code -level 0] payload } \
                    on 42 {message options} { lappend log first-42 } \
                    on 42 {message options} { lappend log duplicate-42 } \
                    on 43 {message options} { lappend log code-43 } \
                    on -1 {message options} { lappend log minus-one } \
                    on error {message options} { lappend log symbolic-error } \
                    finally { lappend log finally }
                return $log
            }
            puts [probe {-code error}]
            puts [probe {-level 0 -code error}]
            puts [probe {-foo bar}]
            puts [probe {-c error}]
            puts [probe {-level 0x0 -code error}]
            puts [probe_dynamic_code error]
            puts [probe_dynamic_code bogus]
            foreach options {
                {-code ok -level 0}
                {-code error -level 0}
                {-code return -level 0}
                {-code break -level 0}
                {-code continue -level 0}
            } { puts [probe_dynamic_options $options] }
            puts [probe_source_order]
            puts [probe_custom_code 42]
            puts [probe_custom_code 43]
            puts [probe_custom_code 4294967295]
            puts [probe_custom_code error]
        ";
        let mut child = Command::new("tclsh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("tclsh oracle available");
        child
            .stdin
            .as_mut()
            .expect("oracle stdin")
            .write_all(script.as_bytes())
            .expect("write oracle script");
        let output = child.wait_with_output().expect("wait for tclsh oracle");
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).expect("oracle stdout"),
            concat!(
                "return finally after\nerror finally after\nreturn finally after\n",
                "return finally after\nerror finally after\n",
                "return finally after\nerror finally after\n",
                "ok finally after\nerror finally after\nreturn finally after\n",
                "break finally after\ncontinue finally after\n",
                "broad-trap finally\n",
                "first-42 finally\ncode-43 finally\nminus-one finally\nsymbolic-error finally\n",
            )
        );
    }
}
