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
//! This is the shared, consumer-agnostic home for the diagram shape. The
//! `tcl diagram` CLI verb, the LSP server, `tcl-mcp` and the BIG-IP report
//! (through its `PyO3` facade) all build the *same* tree from this one
//! implementation. Callers supply the actual [`CommandRegistry`] and source
//! profile. Original declaration allocations and typed event descriptors select
//! source cards; labels supply no lookup, event or completion authority.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value, json};
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::expr_ast::{ExprNode, render_expr};
use tcl_compiler::interprocedural::{namespace_parts_from_proc, resolve_internal_call};
use tcl_compiler::ir::{
    CommandTokens, HandlerMatch, Module, Procedure, Script, Statement, SwitchArm, TryHandler,
    when_event_name,
};
use tcl_compiler::realm::{CommandBindingRealm, RealmBinding};
use tcl_compiler::registry_invocation::effective_command_arguments;
use tcl_lexer::SourceImage;
use tcl_registry::CommandRegistry;
use tcl_registry::InvocationArguments;
use tcl_registry::events::EventRegistry;
use tcl_registry::registry::{
    ExactInvocationCompletion, InvocationCompletion, InvocationCompletionKnowledge,
};

const MAX_DEPTH: usize = 8;
const MAX_EVENTS: usize = 12;
const MAX_ARG_LEN: usize = 60;

/// The semantic context shared by every recursive diagram projection walk.
///
/// Original procedure declarations and call allocations come from the shared
/// source graph owner. The explicitly selected lexical compatibility view
/// retains its independent resolver; printed names are labels in all views.
struct DiagramContext<'a> {
    procedure_names: &'a HashSet<String>,
    identities: &'a CommandBindingRealm,
    registry: &'a CommandRegistry,
    source: &'a str,
    analysis: &'a AnalysisResult,
    availability: &'a tcl_registry::model::ContextRegistry,
}

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
    ExitOrError,
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
            Self::ExitOrError => Some("exit_or_error"),
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
    original: bool,
) -> DiagramCompletion {
    if original {
        let Some(tokens) = tokens else {
            return DiagramCompletion::Normal;
        };
        let Some(selected) =
            tcl_compiler::registry_invocation::resolved_tokens_invocation(registry, None, tokens)
        else {
            return DiagramCompletion::Normal;
        };
        let (knowledge, coarse) = selected.with_argument_words(|words| {
            (
                registry.invocation_completion_knowledge(
                    &selected.facts.canonical_command,
                    words.arguments(),
                    None,
                ),
                registry.invocation_completion_words(
                    &selected.facts.canonical_command,
                    words.arguments(),
                    None,
                ),
            )
        });
        return completion_from_metadata(knowledge, coarse);
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let effective = tokens.map(|tokens| {
        effective_command_arguments(
            tokens,
            tcl_dialect::EscapeSyntax::of_dialect_name(
                registry.profile().map(|profile| profile.name),
            ),
            tcl_syntax::word_rules::WordValueRules::of_profile(registry.profile()),
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
            None,
        )
    } else {
        registry
            .exact_invocation_completion(command, &arg_refs, None)
            .map(InvocationCompletionKnowledge::Exact)
    };
    completion_from_metadata(
        knowledge,
        registry.invocation_completion(command, &arg_refs, None),
    )
}

fn completion_from_metadata(
    knowledge: Option<InvocationCompletionKnowledge>,
    coarse: InvocationCompletion,
) -> DiagramCompletion {
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
            InvocationCompletionKnowledge::ExitOrError => DiagramCompletion::ExitOrError,
            InvocationCompletionKnowledge::Dynamic
            | InvocationCompletionKnowledge::CatchableExitOrError => DiagramCompletion::Dynamic,
        };
    }
    match coarse {
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

// Source-card metadata is selected independently of the printed action label.
fn original_action_source(mut node: Value, at: u32, context: &DiagramContext<'_>) -> Value {
    let Some(input) = context.analysis.resolved_input.as_ref() else {
        return node;
    };
    let segment = if context.analysis.has_original_vendor_source_names() {
        let Some((_, segment)) =
            tcl_compiler::registry_invocation::source_structure::selected_vendor_registry_words_at(
                context.source,
                context.analysis,
                at,
            )
        else {
            return node;
        };
        segment
    } else {
        let Some(tail) = context.source.get(at as usize..) else {
            return node;
        };
        let Some(segment) = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            tail,
            at,
            input.lexer_config(),
        )
        .into_iter()
        .next() else {
            return node;
        };
        segment
    };
    if segment.span.start() != at {
        return node;
    }
    let Some(words) = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        context.source,
        context.analysis,
        &segment,
    ) else {
        return node;
    };
    if !words.matches_registry(context.registry)
        || words.context() != Some(context.availability.context())
    {
        return node;
    }
    if let Some(object) = node.as_object_mut() {
        object.insert(
            "source_span".to_owned(),
            json!([segment.span.start(), segment.span.end()]),
        );
        object.insert(
            "source_schema".to_owned(),
            json!({"command": words.command(), "applicability": "conditional-source",
            "obligations": ["runtime-path-reachability", "observed-terminal-outcome"]}),
        );
    }
    node
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

/// The registry command a source head denotes at one exact source position.
///
/// Original consumers use authentic retained tokens and the shared Registry
/// assistance owner; hosted source retains its separate guarded schema. The
/// explicitly selected lexical branch uses its position-sensitive resolver.
/// A presentation command cannot restore a missing original lookup.
fn registry_command(
    command: &str,
    canonical_command: Option<&str>,
    caller_qname: &str,
    at: u32,
    tokens: Option<&CommandTokens>,
    context: &DiagramContext<'_>,
) -> Option<String> {
    if !context.analysis.allows_lexical_declaration_advice() {
        if context.analysis.has_original_vendor_source_names() {
            // Implementation contract: naming.consumer.original-irules-source-context
            // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
            // A readonly source card uses the same sealed original vector as
            // its source geometry. IR execution tokens retain their own purpose.
            let (selected, segment) = tcl_compiler::registry_invocation::source_structure::selected_vendor_registry_words_at(
                context.source, context.analysis, at,
            )?;
            let config = context.analysis.body_lexer_config?;
            if segment.span.start() != at
                || !selected.matches_source(&SourceImage::document(context.source), config)
                || !selected.matches_registry(context.registry)
                || selected.shape().context() != context.availability.context()
            {
                return None;
            }
            return Some(selected.shape().command().to_owned());
        }
        let tokens = tokens?;
        let assistance =
            tcl_compiler::registry_invocation::original_registry_invocation_assistance(
                context.registry,
                None,
                tokens,
            )?;
        return Some(assistance.unanimous_command_words()?.command().to_owned());
    }
    let command = match context.identities.resolve(command, at) {
        RealmBinding::Command(resolved) if resolved != command => resolved,
        RealmBinding::Rebound => return None,
        RealmBinding::Command(_) => canonical_command.unwrap_or(command),
    };
    let ns_parts = namespace_parts_from_proc(caller_qname);
    let namespace = if ns_parts.is_empty() {
        "::".to_owned()
    } else {
        format!("::{}", ns_parts.join("::"))
    };
    tcl_compiler::naming::resolve_command_with::<&str, _>(&namespace, &[], command, |qname| {
        context
            .registry
            .get(qname.strip_prefix("::").unwrap_or(qname))
            .is_some()
    })
    .map(|qname| qname.trim_start_matches("::").to_owned())
}

/// Whether `command` is a call to a user procedure in `caller_qname`'s
/// namespace at this call site.
///
/// The compiler owns Tcl command-name lookup, including rooted names and the
/// caller-namespace/global candidate sequence. A proven source binding to a
/// registry command is intentionally not reinterpreted as a same-spelled
/// procedure: `rename error saved` must keep `saved` as the original command
/// even if a later `proc error` exists.
fn is_procedure_call(
    command: &str,
    canonical_command: Option<&str>,
    caller_qname: &str,
    at: u32,
    context: &DiagramContext<'_>,
) -> bool {
    if !context.analysis.allows_lexical_declaration_advice() {
        return original_procedure_target_at(at, context).is_some();
    }
    if matches!(context.identities.resolve(command, at), RealmBinding::Command(resolved) if resolved != command)
    {
        return false;
    }
    resolve_internal_call(
        canonical_command.unwrap_or(command),
        caller_qname,
        context.procedure_names,
    )
    .is_some()
}

/// Unique source declaration allocation at this authentic current call site.
/// The returned ordinal indexes readonly diagram cards, not native dispatch.
fn original_procedure_target_at(at: u32, context: &DiagramContext<'_>) -> Option<usize> {
    let mut targets = context
        .analysis
        .original_procedure_declarations()
        .enumerate()
        .filter_map(|(ordinal, declaration)| {
            context
                .analysis
                .command_invocations
                .iter()
                .any(|invocation| {
                    invocation.range.start() == at
                        && invocation.lookup.is_execution_site()
                        && tcl_compiler::source_graph::invocation_targets_declaration_in(
                            context.source,
                            context.analysis,
                            context.source,
                            context.analysis,
                            invocation,
                            declaration,
                            true,
                        )
                })
                .then_some(ordinal)
        });
    let first = targets.next()?;
    targets.next().is_none().then_some(first)
}

/// Build the `switch` flow-node dict from its subject, arms and default body.
fn walk_switch(
    subject: &str,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    caller_qname: &str,
    context: &DiagramContext<'_>,
    depth: usize,
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
            walk_script(b, caller_qname, context, depth + 1)
        });
        let pattern = if patterns.len() > 1 {
            patterns.join(" | ")
        } else {
            patterns[0].clone()
        };
        serialised_arms.push(json!({ "pattern": pattern, "body": body }));
    }
    if let Some(body) = default_body {
        let body = walk_script(body, caller_qname, context, depth + 1);
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
    caller_qname: &str,
    context: &DiagramContext<'_>,
    depth: usize,
) -> Value {
    let child = walk_script(body, caller_qname, context, depth + 1);
    let mut result = Map::new();
    result.insert("kind".to_owned(), json!("try"));
    result.insert("body".to_owned(), json!(child));
    if !handlers.is_empty() {
        let handler_nodes: Vec<Value> = handlers
            .iter()
            .map(|h| {
                json!({
                    "kind_handler": handler_kind_spelling(h.kind),
                    "match": h.match_arg,
                    "completion_code": handler_completion_code(h, context.registry),
                    "trap_pattern": h.trap_pattern,
                    "fallthrough": h.fallthrough,
                    "body": walk_script(&h.body, caller_qname, context, depth + 1),
                })
            })
            .collect();
        result.insert("handlers".to_owned(), Value::Array(handler_nodes));
    }
    if let Some(finally_body) = finally_body {
        result.insert(
            "finally".to_owned(),
            json!(walk_script(finally_body, caller_qname, context, depth + 1)),
        );
    }
    Value::Object(result)
}

/// The diagram protocol's `kind_handler` value — the wire vocabulary the
/// editors render, one spelling per [`HandlerMatch`], independent of the
/// keyword a command's clause grammar introduces the handler with.
fn handler_kind_spelling(kind: HandlerMatch) -> &'static str {
    match kind {
        HandlerMatch::CompletionCode => "on",
        HandlerMatch::ErrorCodePrefix => "trap",
    }
}

/// Canonicalise a completion-code selector in the shared projection.  Tcl
/// accepts its integer grammar here (including `02`, `+2`, and `0x2`);
/// renderers must not reimplement it. Unknown symbolic selectors cannot be
/// completion codes.
fn handler_completion_code(handler: &TryHandler, registry: &CommandRegistry) -> Option<i32> {
    if handler.kind != HandlerMatch::CompletionCode {
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
    caller_qname: &str,
    context: &DiagramContext<'_>,
    depth: usize,
) -> Value {
    let mut branches: Vec<Value> = Vec::new();
    for clause in clauses {
        let body = walk_script(&clause.body, caller_qname, context, depth + 1);
        branches.push(json!({
            "condition": condition_text(&clause.condition),
            "body": body,
        }));
    }
    if let Some(else_body) = else_body {
        let body = walk_script(else_body, caller_qname, context, depth + 1);
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
    caller_qname: &str,
    at: u32,
    context: &DiagramContext<'_>,
) -> Option<Value> {
    // The lowerer records the target of a statically-known alias. Plain calls
    // deliberately retain their source spelling; `resolve_internal_call`
    // below gives those Tcl's caller-namespace lookup rather than treating
    // equal short tails as interchangeable identities.
    let canonical = canonical_command.unwrap_or(command);
    let display = command;
    // Skip the top-level `when` calls — their bodies are in procedures.
    if context.analysis.allows_lexical_declaration_advice() && canonical == "::when" {
        return None;
    }
    // Procedure calls.
    if is_procedure_call(command, canonical_command, caller_qname, at, context) {
        return Some(json!({
            "kind": "proc_call",
            "label": format!("call {display}"),
            "command": display,
            "declaration_id": (!context.analysis.allows_lexical_declaration_advice())
                .then(|| original_procedure_target_at(at, context))
                .flatten().map(|ordinal| format!("declaration-{ordinal}")),
        }));
    }
    let registry_command = registry_command(
        command,
        canonical_command,
        caller_qname,
        at,
        tokens,
        context,
    );
    let completion = registry_command
        .as_deref()
        .map_or(DiagramCompletion::Normal, |command| {
            command_completion(
                command,
                args,
                tokens,
                context.registry,
                !context.analysis.allows_lexical_declaration_advice(),
            )
        });
    // Keep an exact non-normal completion visible even when it is not a
    // diagram action (for example `error` / `throw`), so a surrounding try
    // can route it through `finally`.  Ordinary unknown calls remain omitted
    // unless the registry marks them as a diagram action.
    if registry_command
        .as_deref()
        .is_some_and(|command| context.registry.is_diagram_action(command))
        || completion != DiagramCompletion::Normal
    {
        return Some(with_completion(
            original_action_source(action_node(display, args), at, context),
            completion,
        ));
    }
    None
}

/// Build a `loop` flow node with the given rendered label and exit reason.
fn loop_node(
    label: &str,
    exit: &str,
    body: &Script,
    caller_qname: &str,
    context: &DiagramContext<'_>,
    depth: usize,
) -> Value {
    let child = walk_script(body, caller_qname, context, depth + 1);
    json!({ "kind": "loop", "label": label, "exit": exit, "body": child })
}

/// Project the compiler's structured loop variants into one loop flow node.
fn walk_loop_statement(
    stmt: &Statement,
    caller_qname: &str,
    context: &DiagramContext<'_>,
    depth: usize,
) -> Option<Value> {
    match stmt {
        Statement::For { body, .. } => Some(loop_node(
            "for",
            "false",
            body,
            caller_qname,
            context,
            depth,
        )),
        Statement::While {
            condition, body, ..
        } => {
            let label = format!("while {}", condition_text(condition));
            Some(loop_node(
                &label,
                "false",
                body,
                caller_qname,
                context,
                depth,
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
                caller_qname,
                context,
                depth,
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

/// Project a barrier's retained Registry action and exact completion metadata.
fn walk_barrier(
    statement: &Statement,
    caller_qname: &str,
    context: &DiagramContext<'_>,
) -> Option<Value> {
    let Statement::Barrier {
        span,
        command,
        canonical_command,
        args,
        tokens,
        ..
    } = statement
    else {
        return None;
    };
    let registry_command = registry_command(
        command,
        canonical_command.as_deref(),
        caller_qname,
        span.start(),
        tokens.as_ref(),
        context,
    );
    let completion = registry_command
        .as_deref()
        .map_or(DiagramCompletion::Normal, |command| {
            command_completion(
                command,
                args,
                tokens.as_ref(),
                context.registry,
                !context.analysis.allows_lexical_declaration_advice(),
            )
        });
    (registry_command
        .as_deref()
        .is_some_and(|command| context.registry.is_diagram_action(command))
        || completion != DiagramCompletion::Normal)
        .then(|| {
            with_completion(
                original_action_source(action_node(command, args), span.start(), context),
                completion,
            )
        })
}

/// Convert one IR statement to a flow-node dict, or `None` to skip it.
/// One arm per IR statement kind, so the length tracks the statement set.
fn walk_statement(
    stmt: &Statement,
    caller_qname: &str,
    context: &DiagramContext<'_>,
    depth: usize,
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
            caller_qname,
            context,
            depth,
        )),

        Statement::If {
            clauses, else_body, ..
        } => Some(walk_if(
            clauses,
            else_body.as_ref(),
            caller_qname,
            context,
            depth,
        )),

        Statement::For { .. } | Statement::While { .. } | Statement::Foreach { .. } => {
            walk_loop_statement(stmt, caller_qname, context, depth)
        }

        Statement::Call {
            span,
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
            caller_qname,
            span.start(),
            context,
        ),

        Statement::Barrier { .. } => walk_barrier(stmt, caller_qname, context),

        Statement::Return { value, .. } => Some(return_node(value.as_ref())),

        Statement::AssignConst { name, value, .. } | Statement::AssignValue { name, value, .. } => {
            assign_node(name, value)
        }

        Statement::AssignExpr { name, expr, .. } => assign_node(name, &render_expr(expr)),

        Statement::Catch { body, .. } => {
            let child = walk_script(body, caller_qname, context, depth + 1);
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
            caller_qname,
            context,
            depth,
        )),

        _ => None,
    }
}

/// Walk all statements in a script, dropping the skipped (`None`) nodes.
fn walk_script(
    script: &Script,
    caller_qname: &str,
    context: &DiagramContext<'_>,
    depth: usize,
) -> Vec<Value> {
    script
        .statements
        .iter()
        .filter_map(|stmt| walk_statement(stmt, caller_qname, context, depth))
        .collect()
}

/// Extract structured `{events, procedures}` flow data from a source, using
/// the caller-supplied registry to classify diagram-action commands. Parses as
/// plain Tcl; use [`diagram_data_for_dialect`] for a dialect (e.g. `f5-irules`,
/// so an iRule's `}{` control flow is parsed correctly).
#[must_use]
pub fn diagram_data(source: &str, registry: &CommandRegistry) -> Value {
    diagram_data_for_dialect(
        source,
        registry,
        tcl_registry::model::ingress::resolve_environment("").analyser_profile(),
    )
}

/// [`diagram_data`] for an explicit dialect. `registry` must be that
/// dialect's registry.
#[must_use]
pub fn diagram_data_for_dialect(
    source: &str,
    registry: &CommandRegistry,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Value {
    let availability = std::sync::Arc::new(
        tcl_registry::model::ingress::context_for_profile(dialect)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = ResolvedAnalysisInput::new(
        dialect,
        dialect,
        availability.clone(),
        tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
    );
    let cu = CompilationUnit::build_for_profile(source, registry, false, dialect);
    let mut analyser = Analyser::new().with_resolved_input(input);
    analyser.set_cu_override(std::sync::Arc::new(cu.clone()));
    let analysis = analyser.analyse(source, dialect.name);
    diagram_data_for_analysis(source, &analysis, &cu.ir_module, &availability)
}

/// Procedure cards from the selected original metadata and lowering geometry.
fn source_procedure_nodes(
    regular_procs: &[(&String, &Procedure)],
    module: &Module,
    context: &DiagramContext<'_>,
) -> Vec<Value> {
    if context.analysis.allows_lexical_declaration_advice() {
        regular_procs.iter().map(|(_, procedure)| json!({ "name": procedure.name,
            "params": procedure.params, "flow": walk_script(&procedure.body, &procedure.qualified_name, context, 0) }))
            .collect::<Vec<_>>()
    } else if context.analysis.has_original_vendor_source_names() {
        context.analysis.original_vendor_procedure_declarations().enumerate().map(|(ordinal, declaration)| {
            let metadata = declaration.metadata();
            let input = declaration.name_input();
            let occurrence = declaration.original_occurrence();
            let original_body = occurrence.original_words().last().filter(|word|
                word.group().kind == tcl_lexer::WordKind::Braced
                    && word.content_span().ok() == Some(metadata.body_span));
            let body = regular_procs.iter().filter(|(_, procedure)|
                original_body.is_some()
                    && procedure.span.start() == occurrence.site().offset
                    && procedure.body_offset == metadata.body_span.start()
                    && procedure.body_source.as_deref() == context.source.get(
                        metadata.body_span.start() as usize..metadata.body_span.end() as usize))
                .collect::<Vec<_>>();
            let flow = match body.as_slice() {
                [(label, procedure)] => Some(walk_script(&procedure.body, label, context, 0)),
                _ => None,
            };
            let name = tcl_syntax::native_string::resident_name_label(input.original_word().try_text().unwrap_or("").as_bytes());
            json!({ "id": format!("declaration-{ordinal}"), "name": name,
                "name_span": {"start": input.span().start(), "end": input.span().end()},
                "params": metadata.params.iter().map(|parameter| &parameter.name).collect::<Vec<_>>(),
                "flow": flow, "source_policy": "hosted" })
        }).collect::<Vec<_>>()
    } else {
        context.analysis.original_procedure_declarations().enumerate().map(|(ordinal, declaration)| {
            let metadata = declaration.metadata();
            let flow = tcl_compiler::source_graph::procedure_body_for_declaration(context.source, context.analysis, module, declaration)
                .map(|(label, procedure)| walk_script(&procedure.body, label, context, 0));
            let name = declaration.name().source_spelling().unwrap_or_else(||
                tcl_syntax::native_string::resident_name_label(declaration.name_input().bytes()));
            let span = declaration.name_input().span();
            json!({ "id": format!("declaration-{ordinal}"), "name": name,
                "name_span": {"start": span.start(), "end": span.end()},
                "params": metadata.params.iter().map(|parameter| &parameter.name).collect::<Vec<_>>(), "flow": flow })
        }).collect::<Vec<_>>()
    }
}

/// Structural source projection over independently retained current analysis
/// and lowering. Original declaration IDs preserve equal display names; event
/// descriptors select source handlers without granting worker or TMM entry.
/// Unavailable source/configuration/Registry correspondence returns no tree.
#[must_use]
pub fn diagram_data_for_analysis(
    source: &str,
    analysis: &AnalysisResult,
    module: &Module,
    availability: &tcl_registry::model::ContextRegistry,
) -> Value {
    // Implementation contract: naming.consumer.original-structural-diagrams
    // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
    let registry = availability.commands();
    let Some((image, config)) = tcl_compiler::source_graph::current_analysis(source, analysis)
    else {
        return json!({ "events": [], "procedures": [], "unavailable": "current-source" });
    };
    if module.source != image
        || module.lexer_config != config
        || module
            .registry_snapshot
            .as_ref()
            .map(tcl_registry::RegistrySnapshot::semantic_key)
            != Some(registry.snapshot().semantic_key())
        || analysis.resolved_input.as_ref().is_none_or(|input| {
            input.context_registry().context() != availability.context()
                || input
                    .context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
                    != registry.snapshot().semantic_key()
        })
    {
        return json!({ "events": [], "procedures": [], "unavailable": "current-source-context" });
    }
    let mut items: Vec<(&String, &Procedure)> = module.procedures.iter().collect();
    items.sort_by_key(|(_, procedure)| procedure.span.start());
    let regular_procs = items
        .iter()
        .copied()
        .filter(|(label, procedure)| {
            tcl_compiler::source_graph::event_body_for_procedure(module, label, procedure, registry)
                .is_none()
        })
        .collect::<Vec<_>>();
    let procedure_names = regular_procs
        .iter()
        .map(|(_, procedure)| procedure.qualified_name.clone())
        .collect();
    let context = DiagramContext {
        procedure_names: &procedure_names,
        identities: analysis
            .retained_command_realm()
            .expect("current analysis retains its realm"),
        registry,
        source,
        analysis,
        availability,
    };
    let event_registry = EventRegistry::build();
    let mut handlers_by_event: HashMap<String, Vec<&Procedure>> = HashMap::new();
    let mut unique_events = Vec::new();
    for (label, procedure) in &items {
        let Some(event) = tcl_compiler::source_graph::event_body_for_procedure(
            module, label, procedure, registry,
        ) else {
            continue;
        };
        let event = event.event().to_owned();
        let entry = handlers_by_event.entry(event.clone()).or_default();
        if entry.is_empty() {
            unique_events.push(event);
        }
        entry.push(procedure);
    }
    for handlers in handlers_by_event.values_mut() {
        handlers.sort_by_key(|procedure| procedure.base_priority);
    }
    let mut events = Vec::new();
    'outer: for event in event_registry.order_events(&unique_events) {
        if let Some(handlers) = handlers_by_event.get(&event) {
            for procedure in handlers {
                events.push(json!({ "name": event,
                    "priority": (procedure.base_priority != 500).then_some(procedure.base_priority),
                    "multiplicity": event_registry.event_multiplicity(&event),
                    "flow": walk_script(&procedure.body, &procedure.qualified_name, &context, 0) }));
                if events.len() >= MAX_EVENTS {
                    break 'outer;
                }
            }
        }
    }
    let procedures = source_procedure_nodes(&regular_procs, module, &context);
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
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/0/body/0/completion"),
            Some(&Value::String("return".to_owned()))
        );

        let overridden = diagram_data_for_dialect(
            "proc paths {} { try { set done [clock seconds] } finally { return stop } }",
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
    fn exit_statuses_project_release_specific_errors_and_typed_dynamic_outcomes() {
        let source = r"
            proc paths {status} {
                try { exit 4294967296 } on error {message options} { set caught [clock seconds] } finally { set cleaned [clock seconds] }
                try { exit $status } on error {message options} { set caught_dynamic [clock seconds] } finally { set cleaned_dynamic [clock seconds] }
                set after [clock seconds]
            }
        ";
        let tcl86 = diagram_data_for_dialect(
            source,
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        assert_eq!(
            tcl86.pointer("/procedures/0/flow/0/body/0/completion"),
            Some(&Value::String("error".to_owned())),
            "Tcl 8.6 rejects the first value above Tcl_GetIntFromObj's UINT_MAX ceiling: {tcl86}"
        );
        assert_eq!(
            tcl86.pointer("/procedures/0/flow/0/handlers/0/body/0/kind"),
            Some(&Value::String("assign".to_owned())),
            "Tcl 8.6's static range error reaches on-error: {tcl86}"
        );
        assert_eq!(
            tcl86.pointer("/procedures/0/flow/0/finally/0/kind"),
            Some(&Value::String("assign".to_owned())),
            "Tcl 8.6's static range error reaches finally: {tcl86}"
        );
        assert_eq!(
            tcl86.pointer("/procedures/0/flow/2/kind"),
            Some(&Value::String("assign".to_owned())),
            "Tcl 8.6's handled error preserves the after path: {tcl86}"
        );

        let tcl90 = diagram_data_for_dialect(
            source,
            tcl_registry::model::ingress::static_context_for("tcl9.0").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert_eq!(
            tcl90.pointer("/procedures/0/flow/0/body/0/completion"),
            Some(&Value::String("process_exit".to_owned())),
            "Tcl 9.0's bit-preserving wide conversion accepts 4294967296: {tcl90}"
        );
        for (dialect, data) in [("tcl8.6", &tcl86), ("tcl9.0", &tcl90)] {
            assert_eq!(
                data.pointer("/procedures/0/flow/1/body/0/completion"),
                Some(&Value::String("exit_or_error".to_owned())),
                "{dialect}: dynamic exit has no ordinary completion"
            );
        }
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
                tcl_registry::model::ingress::static_context_for_profile(
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                )
                .commands(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
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
                tcl_registry::model::ingress::static_context_for_profile(
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                )
                .commands(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
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
                tcl_registry::model::ingress::static_context_for_profile(
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                )
                .commands(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
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
                tcl_registry::model::ingress::static_context_for_profile(
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                )
                .commands(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
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
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
                tcl_registry::model::ingress::static_context_for_profile(
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                )
                .commands(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
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
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
                tcl_registry::model::ingress::static_context_for_profile(
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                )
                .commands(),
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
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
            let profile = tcl_registry::model::ingress::static_context_for_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
            .commands()
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
    fn procedure_calls_resolve_qualified_names_in_the_callers_namespace() {
        let data = diagram_data_for_dialect(
            r"
                namespace eval ::alpha {
                    proc helper {} {}
                    proc caller {} {
                        helper
                        ::alpha::helper
                    }
                }
                namespace eval ::beta {
                    proc helper {} {}
                    proc caller {} {
                        helper
                        alpha::helper
                        ::alpha::helper
                    }
                }
                namespace eval ::unrelated {
                    proc caller {} { helper }
                }
            ",
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );

        let procedures = data["procedures"].as_array().expect("procedures");
        assert_eq!(procedures.len(), 5, "{data}");
        assert_eq!(procedures[1]["name"], "caller");
        assert_eq!(procedures[3]["name"], "caller");
        assert_eq!(procedures[4]["name"], "caller");

        for index in [0, 1] {
            assert_eq!(procedures[1]["flow"][index]["kind"], "proc_call");
        }
        // A relative qualified command first probes `::beta::alpha::helper`,
        // then correctly falls back to `::alpha::helper`; rooted spelling is
        // a direct lookup of that same declaration.
        for index in [0, 1, 2] {
            assert_eq!(procedures[3]["flow"][index]["kind"], "proc_call");
        }
        assert!(
            procedures[4]["flow"].as_array().is_some_and(Vec::is_empty),
            "a same-tailed procedure in another namespace must not match: {data}"
        );
    }

    #[test]
    fn registry_actions_resolve_relative_names_in_the_callers_namespace() {
        let data = diagram_data_for_dialect(
            "proc ::HTTP::caller {} { respond 200 content ok }",
            tcl_registry::model::ingress::static_context_for("f5-irules").commands(),
            tcl_dialect::DialectProfile::irules(),
        );
        let flow = data
            .pointer("/procedures/0/flow")
            .and_then(Value::as_array)
            .expect("caller flow");
        assert_eq!(flow.len(), 1, "{data}");
        assert_eq!(flow[0]["kind"], "action");
        assert_eq!(flow[0]["command"], "respond");
    }

    #[test]
    fn procedure_aliases_and_rebound_registry_heads_stay_source_sensitive() {
        let data = diagram_data_for_dialect(
            r"
                proc ::library::helper {} {}
                interp alias {} invoke {} ::library::helper
                rename error saved_error
                proc error {args} {}
                proc caller {} {
                    invoke
                    error user
                    saved_error original
                }
            ",
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );

        let flow = data
            .pointer("/procedures/2/flow")
            .and_then(Value::as_array)
            .expect("caller flow");
        assert_eq!(flow.len(), 3, "{data}");
        assert_eq!(flow[0]["kind"], "proc_call");
        assert_eq!(flow[0]["command"], "invoke");
        assert_eq!(flow[1]["kind"], "proc_call");
        assert_eq!(flow[1]["command"], "error");
        assert_eq!(flow[2]["kind"], "action");
        assert_eq!(flow[2]["command"], "saved_error");
        assert_eq!(flow[2]["completion"], "error");
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

#[cfg(test)]
mod original_source_tests {
    use super::*;
    use std::sync::Arc;

    fn analysed(
        source: &str,
        dialect: &str,
    ) -> (
        AnalysisResult,
        CompilationUnit,
        Arc<tcl_registry::model::ContextRegistry>,
    ) {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        let unit = CompilationUnit::build_for_profile(source, context.commands(), false, profile);
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.set_cu_override(Arc::new(unit.clone()));
        let analysis = analyser.analyse(source, dialect);
        (analysis, unit, context)
    }

    #[test]
    fn original_diagrams_keep_opaque_declarations_and_body_allocations_after_labels_clear() {
        // Implementation contract: naming.consumer.original-structural-diagrams
        // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
        let source = r"proc p\uD800 {} {return FIRST}; p\uD800; proc p\uD801 {} {return OTHER}; p\uD801; proc p\uD800 {} {return LAST}; p\uD800";
        let (mut analysis, mut unit, context) = analysed(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name = "counterfactual".to_owned();
        }
        for procedure in unit.ir_module.procedures.values_mut() {
            procedure.name = "counterfactual".to_owned();
            procedure.qualified_name = "counterfactual".to_owned();
        }
        let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context);
        let declarations = data["procedures"].as_array().unwrap();
        assert_eq!(declarations.len(), 3, "{data}");
        assert_eq!(declarations[0]["name"], declarations[2]["name"]);
        assert_ne!(declarations[0]["name"], declarations[1]["name"]);
        assert_eq!(
            declarations
                .iter()
                .map(|row| row["id"].as_str().unwrap())
                .collect::<HashSet<_>>()
                .len(),
            3
        );
        assert!(
            declarations.iter().any(|row| row["flow"].is_array()),
            "{data}"
        );
        let mut without_bodies = unit.ir_module.clone();
        without_bodies.procedure_implementation_bodies = Arc::from([]);
        without_bodies.original_declaration_body_units.clear();
        without_bodies.installed_procedure_body_units.clear();
        let data = diagram_data_for_analysis(source, &analysis, &without_bodies, &context);
        assert!(
            data["procedures"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["flow"].is_null()),
            "{data}"
        );
        assert!(
            diagram_data_for_analysis(
                &format!("#changed\n{source}"),
                &analysis,
                &unit.ir_module,
                &context
            )["procedures"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let mut wrong_config = analysis.clone();
        wrong_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting = !analysis.body_lexer_config.unwrap().strict_quoting;
        assert!(diagram_data_for_analysis(source, &wrong_config, &unit.ir_module, &context)
            ["procedures"].as_array().unwrap().is_empty());
        let foreign = tcl_registry::model::ingress::context_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let changed_registry = context.with_command_store(foreign.commands().clone());
        assert!(diagram_data_for_analysis(source, &analysis, &unit.ir_module, &changed_registry)
            ["procedures"].as_array().unwrap().is_empty());
    }

    #[test]
    fn original_diagram_calls_use_retained_allocations_and_decline_missing_lookup() {
        // Implementation contract: naming.consumer.original-structural-diagrams
        // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
        let source = "proc target {} {return OK}; target; proc other {} {return OTHER}; other";
        let (mut analysis, unit, availability) = analysed(source, "tcl8.6");
        let names = HashSet::from(["counterfactual".to_owned()]);
        let context = DiagramContext {
            source,
            analysis: &analysis,
            registry: availability.commands(),
            availability: &availability,
            identities: analysis.retained_command_realm().unwrap(),
            procedure_names: &names,
        };
        let at = u32::try_from(source.find("; target").unwrap() + 2).unwrap();
        assert!(is_procedure_call(
            "counterfactual",
            None,
            "counterfactual",
            at,
            &context
        ));
        for invocation in &mut analysis.command_invocations {
            invocation.original_lookup = None;
        }
        let context = DiagramContext {
            source,
            analysis: &analysis,
            registry: availability.commands(),
            availability: &availability,
            identities: analysis.retained_command_realm().unwrap(),
            procedure_names: &names,
        };
        assert!(!is_procedure_call(
            "target",
            Some("::target"),
            "::",
            at,
            &context
        ));
        let _ = unit;
    }

    #[test]
    fn original_diagram_events_require_actual_event_descriptors_instead_of_when_labels() {
        // Implementation contract: naming.consumer.original-structural-diagrams
        // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
        let source = "namespace eval when {proc HTTP_REQUEST {} {return ordinary}}";
        let (analysis, unit, context) = analysed(source, "tcl8.6");
        let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context);
        assert!(data["events"].as_array().unwrap().is_empty(), "{data}");
        assert_eq!(data["procedures"].as_array().unwrap().len(), 1, "{data}");
        let source = "when HTTP_REQUEST { set seen [clock seconds] }";
        let (analysis, mut unit, context) = analysed(source, "f5-irules");
        let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context);
        assert_eq!(data["events"].as_array().unwrap().len(), 1, "{data}");
        assert_eq!(data["events"][0]["name"], "HTTP_REQUEST");
        unit.ir_module.irules_event_bodies.clear();
        assert!(
            diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context)["events"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn original_diagram_actions_keep_source_schema_separate_from_display_labels() {
        // Implementation contract: naming.consumer.original-irules-source-context
        // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
        for source in [
            "when HTTP_REQUEST {pool /Common/first; HTTP::respond 503}",
            "when HTTP_REQUEST {pool /Common/first\nHTTP::respond 503}",
        ] {
            let (mut analysis, unit, availability) = analysed(source, "f5-irules");
            for invocation in &mut analysis.command_invocations {
                invocation.name = "counterfactual".to_owned();
            }
            analysis.all_procs.clear();
            analysis.global_scope.classes.clear();
            let at = u32::try_from(source.find("pool /Common/first").unwrap()).unwrap();
            let (metadata, segment) = tcl_compiler::registry_invocation::source_structure::selected_vendor_registry_words_at(
            source, &analysis, at,
        ).expect("the exact retained pool source vector owns its conditional card");
            assert_eq!(segment.span.start(), at);
            assert_eq!(metadata.shape().command(), "pool");
            assert!(
                metadata
                    .shape()
                    .possible_traits()
                    .contains(tcl_registry::Traits::DIAGRAM_ACTION)
            );
            let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &availability);
            let action = data.pointer("/events/0/flow/0").unwrap();
            assert_eq!(action["source_schema"]["command"], "pool");
            let start = action["source_span"][0].as_u64().unwrap() as usize;
            let end = action["source_span"][1].as_u64().unwrap() as usize;
            assert_eq!(&source[start..end], "pool /Common/first");
            assert_eq!(
                action["source_schema"]["applicability"],
                "conditional-source"
            );
            let stale = diagram_data_for_analysis(
                &format!("#changed\n{source}"),
                &analysis,
                &unit.ir_module,
                &availability,
            );
            assert_eq!(stale["unavailable"], "current-source");
            assert!(stale["events"].as_array().unwrap().is_empty());
            assert!(stale["procedures"].as_array().unwrap().is_empty());
        }
    }
}
