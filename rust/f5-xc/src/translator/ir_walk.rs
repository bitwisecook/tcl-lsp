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

//! The walk over the lowered iRule: statements, `switch` arms and translated calls.

use super::match_criteria::{
    decompose_and_condition, decompose_or_condition, enclosing_has_matches, extract_all_matches,
    merge_enclosing, switch_host_match, switch_method_match, switch_path_match,
    switch_subject_kind,
};
use super::source_advice::{SourceExpressionAdvice, source_xc_advice_allowed};
use super::{EnclosingContext, MAX_DEPTH, TranslationContext};
use crate::mapping::{UNTRANSLATABLE_PREFIXES, command_xc_map, header_subcommand_map};
use crate::model::{
    TranslateStatus, TranslationItem, XCConstructKind, XCDirectResponseAction, XCHeaderAction,
    XCOriginPoolRef, XCRedirectAction, XCRoute, XCServicePolicyRule, XCWafExclusionRule,
};
use tcl_compiler::ir::{Script, Statement, SwitchArm, SwitchMode};
use tcl_core_types::DiagCode;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_syntax::expr::ast::{ExprNode, expr_text};

// IR walking

pub(super) fn walk_script(
    script: &Script,
    ctx: &mut TranslationContext<'_>,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
    for stmt in &script.statements {
        walk_statement(stmt, ctx, registry, depth, enclosing);
    }
}

// One flat arm per statement kind in the IR walk; splitting obscures the dispatch.
#[allow(clippy::too_many_lines)]
fn walk_statement(
    stmt: &Statement,
    ctx: &mut TranslationContext<'_>,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
    if depth > MAX_DEPTH {
        return;
    }

    match stmt {
        Statement::Switch {
            subject,
            subject_span,
            subject_braced,
            arms,
            default_body,
            span,
            mode,
            nocase,
            ..
        } => walk_switch(
            switch_subject_kind(
                (!subject_braced)
                    .then(|| {
                        let advice = SourceExpressionAdvice::new(ctx, None);
                        advice.scalar_operand(&advice.original_operand(*subject_span)?)
                    })
                    .flatten(),
            ),
            subject,
            arms,
            default_body.as_ref(),
            *mode,
            *nocase,
            *span,
            ctx,
            registry,
            depth,
            enclosing,
        ),
        Statement::If {
            clauses,
            else_body,
            span,
            ..
        } => {
            if clauses.iter().any(|clause| {
                !SourceExpressionAdvice::new(ctx, clause.condition_base)
                    .command_owners_available(&clause.condition)
            }) {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Untranslatable,
                    kind: XCConstructKind::Note,
                    irule_command: "if".to_owned(),
                    irule_range: Some(*span),
                    xc_description: "Condition commands have no current original source owner"
                        .to_owned(),
                    note: String::new(),
                    diagnostic_code: DiagCode::Xc203,
                });
                return;
            }
            let mut any_matched = false;
            for clause in clauses {
                let cond = &clause.condition;
                let advice = SourceExpressionAdvice::new(ctx, clause.condition_base);
                let mut or_branches: Vec<&ExprNode> = Vec::new();
                decompose_or_condition(cond, &mut or_branches);
                if or_branches.len() > 1 {
                    for branch in or_branches {
                        let mut and_subs: Vec<&ExprNode> = Vec::new();
                        decompose_and_condition(branch, &mut and_subs);
                        let matches = extract_all_matches(&and_subs, &advice);
                        let merged = merge_enclosing(enclosing, &matches);
                        if enclosing_has_matches(&matches) {
                            any_matched = true;
                        }
                        walk_script(&clause.body, ctx, registry, depth + 1, &merged);
                    }
                } else {
                    let mut and_subs: Vec<&ExprNode> = Vec::new();
                    decompose_and_condition(cond, &mut and_subs);
                    let matches = extract_all_matches(&and_subs, &advice);
                    let merged = merge_enclosing(enclosing, &matches);
                    if enclosing_has_matches(&matches) {
                        any_matched = true;
                    }
                    walk_script(&clause.body, ctx, registry, depth + 1, &merged);
                }
            }
            if let Some(eb) = else_body {
                // The `else` runs when every clause condition is false, but the
                // outer (enclosing) criteria still hold — mirror `walk_switch`'s
                // default-body handling, which keeps `enclosing` and only clears
                // its own key. Preserve `enclosing` so an `else`-body action does
                // not become an unconditional catch-all route.
                // Any criterion the `if` clauses matched on is not part of
                // `enclosing`, so it is already excluded here.
                walk_script(eb, ctx, registry, depth + 1, enclosing);
            }
            if any_matched {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Translated,
                    kind: XCConstructKind::ServicePolicyRule,
                    irule_command: "if condition".to_owned(),
                    irule_range: Some(*span),
                    xc_description: "XC service policy rule with match criteria".to_owned(),
                    note: String::new(),
                    diagnostic_code: DiagCode::Xc102,
                });
            } else {
                let cond_text = clauses
                    .first()
                    .map_or_else(|| "?".to_owned(), |c| expr_text(&c.condition));
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Partial,
                    kind: XCConstructKind::ServicePolicyRule,
                    irule_command: format!("if {{{cond_text}}}"),
                    irule_range: Some(*span),
                    xc_description: "Conditional logic — review manually for XC match criteria"
                        .to_owned(),
                    note: "Complex condition could not be automatically mapped".to_owned(),
                    diagnostic_code: DiagCode::Xc203,
                });
            }
        }
        Statement::Call {
            command,
            span,
            tokens,
            ..
        } => {
            let selected = tokens.as_ref().and_then(|tokens| {
                tcl_compiler::source_graph::registry_words_for_original_call(
                    ctx.source,
                    ctx.analysis,
                    ctx.module,
                    tokens,
                    registry,
                )
            });
            let static_arguments: Option<Vec<String>> = selected.as_ref().and_then(|words| {
                words
                    .arguments()
                    .iter()
                    .map(|argument| {
                        std::str::from_utf8(argument.literal_bytes()?)
                            .ok()
                            .map(str::to_owned)
                    })
                    .collect()
            });
            if let (Some(selected), Some(arguments)) = (selected, static_arguments)
                && source_xc_advice_allowed(&selected, ctx.analysis)
            {
                walk_call(
                    selected.command(),
                    &arguments,
                    *span,
                    ctx,
                    registry,
                    enclosing,
                );
            } else {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Untranslatable,
                    kind: XCConstructKind::Note,
                    irule_command: command.clone(),
                    irule_range: Some(*span),
                    xc_description: "Command or operands have no current static source owner"
                        .to_owned(),
                    note: String::new(),
                    diagnostic_code: DiagCode::Xc203,
                });
            }
        }
        Statement::Barrier { command, span, .. } => {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: command.clone(),
                irule_range: Some(*span),
                xc_description: "Dynamic construct — cannot be statically translated".to_owned(),
                note: format!(
                    "'{command}' creates dynamic behaviour with no XC equivalent. \
                     Consider App Stack for this logic."
                ),
                diagnostic_code: DiagCode::Xc300,
            });
        }
        Statement::Return { .. } => {}
        Statement::For { body, span, .. }
        | Statement::While { body, span, .. }
        | Statement::Foreach { body, span, .. } => {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: "loop".to_owned(),
                irule_range: Some(*span),
                xc_description: "Loop construct — no XC equivalent".to_owned(),
                note: "XC policies are declarative, not procedural. \
                       Consider App Stack for iterative logic."
                    .to_owned(),
                diagnostic_code: DiagCode::Xc300,
            });
            walk_script(body, ctx, registry, depth + 1, &EnclosingContext::default());
        }
        Statement::Catch { body, .. } | Statement::Try { body, .. } => {
            walk_script(body, ctx, registry, depth + 1, &EnclosingContext::default());
        }
        // Assignments and other statements (Incr, ExprEval, Block, UpFrame)
        // carry no XC construct — ignore.
        _ => {}
    }
}

/// Which criterion a recognised `switch` subject constrains.
#[derive(Debug, Clone, Copy)]
enum SwitchTarget {
    Path,
    Host,
    Method,
}

/// The pattern-matching semantics of a recognised `switch`, derived from its
/// subject and its IR-carried `mode`/`nocase`.
#[derive(Debug, Clone, Copy)]
struct SwitchArmSpec {
    target: SwitchTarget,
    mode: SwitchMode,
    nocase: bool,
}

/// Layer one arm's `pattern` (translated per the switch's mode/nocase) onto
/// `base`, replacing only the target criterion.
fn arm_context(base: &EnclosingContext, spec: SwitchArmSpec, pattern: &str) -> EnclosingContext {
    let mut enc = base.clone();
    match spec.target {
        SwitchTarget::Path => enc.path = Some(switch_path_match(pattern, spec.mode, spec.nocase)),
        SwitchTarget::Host => enc.host = Some(switch_host_match(pattern, spec.mode, spec.nocase)),
        SwitchTarget::Method => enc.method = Some(switch_method_match(pattern)),
    }
    enc
}

/// Walk the arms (and default body) of a recognised `switch`.
///
/// Fall-through arms (`"/a*" -`) carry `body: None`; their pattern must be
/// attached to the following non-fall-through arm's body so that every
/// pattern routes to an action. Each arm's pattern is
/// translated honouring the switch mode/nocase, and the
/// default body keeps `enclosing` with only the switch's own key cleared.
fn walk_switch_arms(
    spec: SwitchArmSpec,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    ctx: &mut TranslationContext<'_>,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
    // Patterns of preceding fall-through arms awaiting a shared body.
    let mut pending: Vec<&str> = Vec::new();
    for arm in arms {
        match arm.body.as_ref() {
            // Fall-through arm: defer its pattern to the next arm with a body.
            None => pending.push(&arm.pattern),
            Some(body) => {
                pending.push(&arm.pattern);
                for pattern in pending.drain(..) {
                    let enc = arm_context(enclosing, spec, pattern);
                    walk_script(body, ctx, registry, depth + 1, &enc);
                }
            }
        }
    }
    if let Some(db) = default_body {
        // Fall-through patterns immediately preceding `default`
        // (`"/old" - default { … }`) share the default arm's body: lowering
        // peels `default` into `default_body` while the preceding
        // fall-through arm stays in `arms` with `body: None`, so `pending`
        // still holds those patterns here. Tcl runs the default body for each
        // such pattern too, so emit a criterion-specific route for every
        // pending pattern before the catch-all route with the key cleared.
        for pattern in pending.drain(..) {
            let enc = arm_context(enclosing, spec, pattern);
            walk_script(db, ctx, registry, depth + 1, &enc);
        }
        let mut enc = enclosing.clone();
        match spec.target {
            SwitchTarget::Path => enc.path = None,
            SwitchTarget::Host => enc.host = None,
            SwitchTarget::Method => enc.method = None,
        }
        walk_script(db, ctx, registry, depth + 1, &enc);
    }
}

// Threads the full translation context (registry, depth, enclosing scope, sink)
// positionally through the recursive IR walk; a struct would just re-wrap it.
#[allow(clippy::too_many_arguments)]
fn walk_switch(
    kind: Option<&'static str>,
    subject: &str,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    mode: SwitchMode,
    nocase: bool,
    span: Span,
    ctx: &mut TranslationContext<'_>,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
    match kind {
        Some("path") => {
            let spec = SwitchArmSpec {
                target: SwitchTarget::Path,
                mode,
                nocase,
            };
            walk_switch_arms(spec, arms, default_body, ctx, registry, depth, enclosing);
            ctx.items.push(translated_route_item(
                format!("switch {subject}"),
                span,
                "XC L7 routes with path matching",
                DiagCode::Xc101,
            ));
        }
        Some("host") => {
            let spec = SwitchArmSpec {
                target: SwitchTarget::Host,
                mode,
                nocase,
            };
            walk_switch_arms(spec, arms, default_body, ctx, registry, depth, enclosing);
            ctx.items.push(translated_route_item(
                format!("switch {subject}"),
                span,
                "XC L7 routes with host matching",
                DiagCode::Xc101,
            ));
        }
        Some("method") => {
            let spec = SwitchArmSpec {
                target: SwitchTarget::Method,
                mode,
                nocase,
            };
            walk_switch_arms(spec, arms, default_body, ctx, registry, depth, enclosing);
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Translated,
                kind: XCConstructKind::ServicePolicyRule,
                irule_command: format!("switch {subject}"),
                irule_range: Some(span),
                xc_description: "XC service policy rules with method matching".to_owned(),
                note: String::new(),
                diagnostic_code: DiagCode::Xc102,
            });
        }
        _ => {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Partial,
                kind: XCConstructKind::Route,
                irule_command: format!("switch {subject}"),
                irule_range: Some(span),
                xc_description: "Switch on dynamic value — needs manual XC config".to_owned(),
                note: "Cannot statically determine match criteria".to_owned(),
                diagnostic_code: DiagCode::Xc200,
            });
            // The subject is dynamic, so no arm pattern can be mapped to a
            // criterion — but the enclosing criteria still apply, so preserve
            // them rather than dropping to a catch-all.
            for arm in arms {
                if let Some(body) = arm.body.as_ref() {
                    walk_script(body, ctx, registry, depth + 1, enclosing);
                }
            }
            if let Some(db) = default_body {
                walk_script(db, ctx, registry, depth + 1, enclosing);
            }
        }
    }
}

fn translated_route_item(
    irule_command: String,
    span: Span,
    xc_description: &str,
    code: DiagCode,
) -> TranslationItem {
    TranslationItem {
        status: TranslateStatus::Translated,
        kind: XCConstructKind::Route,
        irule_command,
        irule_range: Some(span),
        xc_description: xc_description.to_owned(),
        note: String::new(),
        diagnostic_code: code,
    }
}

/// Parse a base-10 integer the way `int()` does for these args:
/// trim surrounding whitespace, accept an optional sign. Returns `None` on
/// failure, in which case the caller keeps its default.
fn py_int(arg: Option<&String>) -> Option<i64> {
    arg.and_then(|s| s.trim().parse::<i64>().ok())
}

// One flat arm per translated command name; splitting obscures the dispatch.
#[allow(clippy::too_many_lines)]
fn walk_call(
    command: &str,
    args: &[String],
    span: Span,
    ctx: &mut TranslationContext<'_>,
    registry: &CommandRegistry,
    enclosing: &EnclosingContext,
) {
    match command {
        "pool" if !args.is_empty() => {
            let pool_name = args[0].clone();
            ctx.add_origin_pool(&pool_name, Some(span));
            ctx.routes.push(XCRoute {
                path_match: enclosing.path.clone(),
                host_match: enclosing.host.clone(),
                method_match: enclosing.method.clone(),
                header_matches: enclosing.header_matches.clone(),
                query_match: enclosing.query_match.clone(),
                cookie_matches: enclosing.cookie_matches.clone(),
                origin_pool: Some(XCOriginPoolRef {
                    name: pool_name,
                    source_range: Some(span),
                }),
                source_range: Some(span),
                ..XCRoute::default()
            });
        }
        "HTTP::redirect" if !args.is_empty() => {
            let url = args[0].clone();
            let code = py_int(args.get(1)).unwrap_or(302);
            ctx.routes.push(XCRoute {
                path_match: enclosing.path.clone(),
                host_match: enclosing.host.clone(),
                method_match: enclosing.method.clone(),
                header_matches: enclosing.header_matches.clone(),
                query_match: enclosing.query_match.clone(),
                cookie_matches: enclosing.cookie_matches.clone(),
                redirect: Some(XCRedirectAction {
                    url: url.clone(),
                    response_code: code,
                    source_range: Some(span),
                }),
                source_range: Some(span),
                ..XCRoute::default()
            });
            ctx.items.push(translated_route_item(
                format!("HTTP::redirect {url}"),
                span,
                "XC redirect route",
                DiagCode::Xc101,
            ));
        }
        "HTTP::respond" if !args.is_empty() => {
            let status_code = py_int(args.first()).unwrap_or(200);
            let mut body = String::new();
            for (i, arg) in args.iter().enumerate() {
                if arg == "content" && i + 1 < args.len() {
                    body.clone_from(&args[i + 1]);
                    break;
                }
            }
            if matches!(status_code, 403 | 401 | 429) {
                let name = ctx.next_rule_name();
                ctx.policy_rules.push(XCServicePolicyRule {
                    name,
                    action: "deny".to_owned(),
                    path_match: enclosing.path.clone(),
                    host_match: enclosing.host.clone(),
                    method_match: enclosing.method.clone(),
                    header_matches: enclosing.header_matches.clone(),
                    query_match: enclosing.query_match.clone(),
                    cookie_matches: enclosing.cookie_matches.clone(),
                    ip_prefix_list: enclosing.ip_prefix_list.clone(),
                    description: format!("Deny with {status_code} (from HTTP::respond)"),
                    source_range: Some(span),
                });
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Translated,
                    kind: XCConstructKind::ServicePolicyRule,
                    irule_command: format!("HTTP::respond {status_code}"),
                    irule_range: Some(span),
                    xc_description: format!("XC service policy deny rule ({status_code})"),
                    note: String::new(),
                    diagnostic_code: DiagCode::Xc102,
                });
            } else {
                ctx.routes.push(XCRoute {
                    path_match: enclosing.path.clone(),
                    host_match: enclosing.host.clone(),
                    method_match: enclosing.method.clone(),
                    header_matches: enclosing.header_matches.clone(),
                    query_match: enclosing.query_match.clone(),
                    cookie_matches: enclosing.cookie_matches.clone(),
                    direct_response: Some(XCDirectResponseAction {
                        status_code,
                        body,
                        source_range: Some(span),
                    }),
                    source_range: Some(span),
                    ..XCRoute::default()
                });
                ctx.items.push(translated_route_item(
                    format!("HTTP::respond {status_code}"),
                    span,
                    &format!("XC direct response route ({status_code})"),
                    DiagCode::Xc101,
                ));
            }
        }
        "HTTP::header" if !args.is_empty() => {
            let subcommand = args[0].to_lowercase();
            if let Some(operation) = header_subcommand_map(&subcommand) {
                if args.len() >= 2 {
                    let header_name = args[1].clone();
                    let header_value = args.get(2).cloned().unwrap_or_default();
                    let target = if ctx.current_event == "HTTP_RESPONSE" {
                        "response"
                    } else {
                        "request"
                    };
                    ctx.header_actions.push(XCHeaderAction {
                        name: header_name.clone(),
                        value: header_value,
                        operation: operation.to_owned(),
                        target: target.to_owned(),
                        source_range: Some(span),
                    });
                    ctx.items.push(TranslationItem {
                        status: TranslateStatus::Translated,
                        kind: XCConstructKind::HeaderAction,
                        irule_command: format!("HTTP::header {subcommand} {header_name}"),
                        irule_range: Some(span),
                        xc_description: format!("XC {target} header {operation}: {header_name}"),
                        note: String::new(),
                        diagnostic_code: DiagCode::Xc103,
                    });
                }
            }
        }
        "class" if !args.is_empty() && args[0] == "match" => {
            let joined = args[1..args.len().min(3)].join(" ");
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Translated,
                kind: XCConstructKind::ServicePolicyRule,
                irule_command: format!("class match {joined}"),
                irule_range: Some(span),
                xc_description: "XC service policy rule (data-group matching)".to_owned(),
                note: "Each data-group entry may need a separate XC rule".to_owned(),
                diagnostic_code: DiagCode::Xc105,
            });
        }
        "ASM::disable" => {
            let name = ctx.next_rule_name();
            ctx.waf_exclusion_rules.push(XCWafExclusionRule {
                name,
                path_match: enclosing.path.clone(),
                ip_prefix_set: enclosing.ip_prefix_set.clone(),
                description: "Disable WAF (from ASM::disable)".to_owned(),
                source_range: Some(span),
            });
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Translated,
                kind: XCConstructKind::WafExclusion,
                irule_command: "ASM::disable".to_owned(),
                irule_range: Some(span),
                xc_description: "XC WAF exclusion rule (disable App Firewall)".to_owned(),
                note: String::new(),
                diagnostic_code: DiagCode::Xc106,
            });
        }
        "ASM::enable" => {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Translated,
                kind: XCConstructKind::Note,
                irule_command: "ASM::enable".to_owned(),
                irule_range: Some(span),
                xc_description: "No action needed — XC App Firewall is enabled by default"
                    .to_owned(),
                note: String::new(),
                diagnostic_code: DiagCode::Xc107,
            });
        }
        _ => {
            // Other commands: registry-driven translatability.
            if registry.is_xc_never_translatable(command) {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Untranslatable,
                    kind: XCConstructKind::Note,
                    irule_command: command.to_owned(),
                    irule_range: Some(span),
                    xc_description: format!("'{command}' has no XC equivalent"),
                    note: "Consider App Stack for this logic.".to_owned(),
                    diagnostic_code: DiagCode::Xc300,
                });
            } else if UNTRANSLATABLE_PREFIXES
                .iter()
                .any(|p| command.starts_with(p))
            {
                if !registry.is_xc_translatable_override(command) {
                    ctx.items.push(TranslationItem {
                        status: TranslateStatus::Untranslatable,
                        kind: XCConstructKind::Note,
                        irule_command: command.to_owned(),
                        irule_range: Some(span),
                        xc_description: format!("'{command}' has no XC equivalent"),
                        note: "L4/protocol-specific command. Consider App Stack.".to_owned(),
                        diagnostic_code: DiagCode::Xc301,
                    });
                }
            } else if let Some(mapping) = command_xc_map(command) {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Translated,
                    kind: mapping.kind,
                    irule_command: command.to_owned(),
                    irule_range: Some(span),
                    xc_description: mapping.xc_description.to_owned(),
                    note: mapping.note.to_owned(),
                    diagnostic_code: DiagCode::Xc100,
                });
            }
        }
    }
}
