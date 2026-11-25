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

//! `generate_irule_test` — generate a runnable iRule test scaffold.
//!
//! Combines the `_build_test_script_with_metadata` chain: the extractors
//! (`_extract_irule_commands`, `_extract_object_refs`, `_extract_variables`),
//! `_infer_profiles`, `_needs_multi_tmm`, and the Tcl test-script template
//! (the Jinja2 `irule_test.tcl.j2` / `test_case.tcl.j2` / `multi_tmm.tcl.j2`
//! trio).
//!
//! Event ordering reuses [`tcl_registry::events::EventRegistry`] and the
//! per-CFG-path cases reuse [`crate::irule_test::cfg_paths_json`]; the output
//! is a functionally-equivalent Tcl test scaffold.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::{Value, json};
use tcl_lexer::LexerConfig;
use tcl_registry::ArgRole;
use tcl_registry::events::EventRegistry;
use tcl_registry::value_transfer::{EvalRoute, NativeEvalId};
use tcl_syntax::expr::parser::ExprParseContext;
use tcl_syntax::list::list_element;

/// The prose emitted for `multi_tmm_hint` when multi-TMM patterns are detected.
const MULTI_TMM_HINT: &str = "Source patterns suggest checking behavior across TMMs \
     (possible static:: writes, counters, or shared table state). \
     The generated test includes a multi-TMM scenario using fakeCMP \
     distribution.  Use ::orch::fakecmp_suggest_sources to plan which \
     client addresses hit which TMMs.";

/// Events implying an HTTP profile (`_infer_profiles`).
const HTTP_EVENTS: &[&str] = &[
    "HTTP_REQUEST",
    "HTTP_RESPONSE",
    "HTTP_REQUEST_DATA",
    "HTTP_RESPONSE_DATA",
    "HTTP_REQUEST_RELEASE",
    "HTTP_RESPONSE_RELEASE",
    "HTTP_REQUEST_SEND",
];

/// Events implying DNS (`_infer_profiles`).
const DNS_EVENTS: &[&str] = &["DNS_REQUEST", "DNS_RESPONSE"];

/// Events implying bare UDP (`_infer_profiles`).
const UDP_EVENTS: &[&str] = &["CLIENT_DATA"];

/// Hot events scanned for `static::` writes (`_needs_multi_tmm`).
const HOT_EVENTS: &[&str] = &[
    "HTTP_REQUEST",
    "HTTP_RESPONSE",
    "CLIENT_ACCEPTED",
    "SERVER_CONNECTED",
    "DNS_REQUEST",
    "DNS_RESPONSE",
];

/// Object references pulled out of an iRule (`_extract_object_refs`).
///
/// `nodes` and `virtuals` could also be collected, but the `generate_irule_test`
/// tool only surfaces `pools` and `datagroups` (in the wire shape and the
/// generated setup/test cases), so only those two are kept here.
#[derive(Default)]
struct ObjectRefs {
    pools: Vec<String>,
    datagroups: Vec<String>,
}

/// Variable-usage patterns (`_extract_variables`). Only `static` is populated.
#[derive(Default)]
struct Variables {
    static_vars: Vec<String>,
}

/// One captured source context and its presentation projection.
#[derive(Debug)]
struct SourceInventory {
    context: Option<tcl_irules::OriginalIrulesSourceContext>,
    commands: Vec<tcl_irules::IrulesExecutableCommand>,
}

impl SourceInventory {
    fn capture(source: &str, registry: &tcl_registry::CommandRegistry) -> Self {
        let context = tcl_irules::OriginalIrulesSourceContext::capture(source, registry);
        let commands = context
            .as_ref()
            .map_or_else(Vec::new, |context| context.presentation_commands(source));
        Self { context, commands }
    }
}

impl std::ops::Deref for SourceInventory {
    type Target = [tcl_irules::IrulesExecutableCommand];

    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

/// MCP tool entry point: analyse the `source` argument and return the
/// `generate_irule_test` wire shape (the `test_script` is a
/// conditional source scaffold with independently supplied outcome obligations).
#[must_use]
pub fn generate_irule_test(args: &Value) -> Value {
    let source: &str = &crate::tools::analysis_source(args);

    let registry = crate::environment::store_for_dialect("f5-irules");
    let inventory = SourceInventory::capture(source, registry);
    let source_context = inventory.context.as_ref();
    let event_registry = EventRegistry::build();
    let mut seen_events = BTreeSet::new();
    let event_names = source_context.map_or_else(Vec::new, |context| {
        context
            .events()
            .iter()
            .map(|event| event.event().to_owned())
            .filter(|event| seen_events.insert(event.clone()))
            .collect::<Vec<_>>()
    });
    let ordered_events = event_registry.order_events(&event_names);
    let profiles = infer_profiles(&event_registry, &ordered_events);
    let closure = &inventory.commands;
    let commands_used = extract_irule_commands(closure);
    let objects = source_context.map_or_else(ObjectRefs::default, |context| {
        extract_object_refs(source, context)
    });
    let variables = extract_variables(&inventory);

    let cfg_paths = crate::irule_test::cfg_paths_json(source);
    let multi_tmm = needs_multi_tmm(source_context, closure, &variables);

    let ctx = ScriptContext {
        basename: "irule.tcl",
        source,
        ordered_events: &ordered_events,
        profiles: &profiles,
        commands_used: &commands_used,
        objects: &objects,
        variables: &variables,
        cfg_paths: &cfg_paths,
        request_syntax: source_context
            .map(tcl_irules::OriginalIrulesSourceContext::expression_syntax),
        multi_tmm,
    };
    let test_script = build_test_script(&ctx);

    json!({
        "test_script": test_script,
        "analysis_kind": "conditional-source",
        "outcome_assertions_verified": false,
        "obligations": ["handler-applicability", "event-reachability", "observed-terminal-outcome"],
        "events": ordered_events,
        "profiles": profiles,
        "commands_used": commands_used,
        "pools": objects.pools,
        "datagroups": objects.datagroups,
        "multi_tmm_detected": multi_tmm,
        "multi_tmm_hint": if multi_tmm { json!(MULTI_TMM_HINT) } else { Value::Null },
        "cfg_paths": cfg_paths,
        "cfg_hint": if cfg_paths.is_empty() {
            Value::Null
        } else {
            json!(format!(
                "Source analysis found {} candidate paths to terminal actions. \
                 Generated scenarios require independently observed expectations. Use \
                 irule_cfg_paths to inspect their source and outcome obligations.",
                cfg_paths.len()
            ))
        },
    })
}

// ── Extractors ────────────────────────────────────────────────────────

/// Sort and deduplicate actual source-schema labels for presentation.
fn extract_irule_commands(commands: &[tcl_irules::IrulesExecutableCommand]) -> Vec<String> {
    commands
        .iter()
        .map(|fact| fact.command.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Retain literal configuration-reference candidates from the same source owner.
fn extract_object_refs(
    source: &str,
    context: &tcl_irules::OriginalIrulesSourceContext,
) -> ObjectRefs {
    let mut pools: BTreeSet<String> = BTreeSet::new();
    let mut datagroups: BTreeSet<String> = BTreeSet::new();
    for reference in
        tcl_irules::extract_original_irules_source_object_references(source, context, None)
    {
        if reference.category == tcl_irules::IrulesObjectReferenceCategory::Pool {
            pools.insert(reference.name);
        } else if reference.category == tcl_irules::IrulesObjectReferenceCategory::DataGroup {
            datagroups.insert(reference.name);
        }
    }

    ObjectRefs {
        pools: pools.into_iter().collect(),
        datagroups: datagroups.into_iter().collect(),
    }
}

/// Collect conditional static-name hints from source tokens and authored roles.
fn extract_variables(commands: &SourceInventory) -> Variables {
    let mut static_vars: BTreeSet<String> = BTreeSet::new();
    for fact in commands.iter() {
        for name in &fact.variable_names {
            if let Some(tail) = tcl_syntax::naming::authored_namespace_hint_tail(name, "static") {
                static_vars.insert(tail.to_owned());
            }
        }
    }
    if let Some(context) = &commands.context {
        for command in context.commands() {
            let words = command.words();
            let _ = words.with_source_schema(context.context_registry(), |selected| {
                if !selected
                    .argument_count_for_arity()
                    .is_some_and(|count| selected.semantics.arity.accepts(count))
                {
                    return;
                }
                for (argument, _) in selected.authored_source_variable_name_arguments() {
                    if words.original_argument_word(argument).is_none() {
                        continue;
                    }
                    let Some(name) = words
                        .arguments()
                        .get(argument)
                        .and_then(|word| word.literal_bytes())
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    else {
                        continue;
                    };
                    if let Some(tail) =
                        tcl_syntax::naming::authored_namespace_hint_tail(name, "static")
                    {
                        static_vars.insert(tail.to_owned());
                    }
                }
            });
        }
    }
    Variables {
        static_vars: static_vars.into_iter().collect(),
    }
}

/// Infer the required profiles from the events present (`_infer_profiles`).
fn infer_profiles(registry: &EventRegistry, events: &[String]) -> Vec<String> {
    let mut profiles = vec!["TCP".to_owned()];
    let has = |set: &[&str]| events.iter().any(|e| set.contains(&e.as_str()));

    let has_http = has(HTTP_EVENTS);
    for profile in ["CLIENTSSL", "SERVERSSL"] {
        if events.iter().any(|event| {
            registry
                .get_props(event)
                .is_some_and(|props| props.implied_profiles.contains(&profile))
        }) {
            profiles.push(profile.to_owned());
        }
    }
    if has_http {
        profiles.push("HTTP".to_owned());
    }
    if has(DNS_EVENTS) {
        return vec!["UDP".to_owned(), "DNS".to_owned()];
    }
    if has(UDP_EVENTS) && !has_http {
        return vec!["UDP".to_owned()];
    }
    profiles
}

/// Suggest a multi-TMM scenario from conditional source patterns.
/// Missing advice does not prove the absence of writes or shared state.
fn needs_multi_tmm(
    context: Option<&tcl_irules::OriginalIrulesSourceContext>,
    commands: &[tcl_irules::IrulesExecutableCommand],
    variables: &Variables,
) -> bool {
    let has_static_vars = !variables.static_vars.is_empty();
    let mut static_in_hot = false;
    let mut has_counter = false;
    if let Some(context) = context {
        for command in context.commands() {
            let _ = command
                .words()
                .with_source_schema(context.context_registry(), |selected| {
                    let Some(EvalRoute::Direct {
                        id: id @ (NativeEvalId::CellWrite | NativeEvalId::CellIncrement),
                    }) = selected.semantics.value.route()
                    else {
                        return;
                    };
                    if !selected
                        .argument_count_for_arity()
                        .is_some_and(|count| selected.semantics.arity.accepts(count))
                    {
                        return;
                    }
                    for (argument, role) in selected.authored_source_variable_name_arguments() {
                        if role != ArgRole::VarWrite {
                            continue;
                        }
                        let Some(name) = command
                            .words()
                            .arguments()
                            .get(argument)
                            .and_then(|word| word.literal_bytes())
                            .and_then(|bytes| std::str::from_utf8(bytes).ok())
                        else {
                            continue;
                        };
                        let static_hint =
                            tcl_syntax::naming::authored_namespace_hint_tail(name, "static")
                                .is_some();
                        static_in_hot |=
                            HOT_EVENTS.contains(&command.event_source().event()) && static_hint;
                        has_counter |= id == NativeEvalId::CellIncrement
                            && (static_hint || name.to_ascii_lowercase().contains("count"));
                    }
                });
        }
    }
    let uses_shared_table = commands.iter().any(|fact| {
        fact.command == "table"
            && fact
                .args
                .first()
                .is_some_and(|arg| matches!(arg.as_str(), "incr" | "set" | "add"))
    });

    static_in_hot || (has_counter && has_static_vars) || uses_shared_table
}

// ── Test-script rendering ─────────────────────────────────────────────

/// Everything `build_test_script` needs, grouped so no single fn takes a long
/// argument list.
struct ScriptContext<'a> {
    basename: &'a str,
    source: &'a str,
    ordered_events: &'a [String],
    profiles: &'a [String],
    commands_used: &'a [String],
    objects: &'a ObjectRefs,
    variables: &'a Variables,
    cfg_paths: &'a [Value],
    request_syntax: Option<(LexerConfig, ExprParseContext)>,
    multi_tmm: bool,
}

/// Build the complete Tcl test script (`_build_test_script` + Jinja2 templates,
/// rendered directly in Rust as a functionally-equivalent scaffold).
fn build_test_script(ctx: &ScriptContext) -> String {
    let test_name = ctx
        .basename
        .trim_end_matches(".tcl")
        .trim_end_matches(".irul")
        .trim_end_matches(".irule");

    let mut out = String::new();
    let _ = writeln!(
        out,
        "# test_{test_name}.tcl -- Generated tests for {}",
        ctx.basename
    );
    out.push_str("#\n");
    out.push_str("# Generated by: tcl-mcp generate_irule_test\n");
    out.push_str("# Customize the test scenarios below to match expected behavior.\n");
    out.push_str("#\n");
    let _ = writeln!(out, "# Run with: tclsh test_{test_name}.tcl\n");

    out.push_str("set script_dir [file dirname [info script]]\n\n");
    out.push_str("# Source the test framework\n");
    for f in [
        "compat84.tcl",
        "state_layers.tcl",
        "tmm_shim.tcl",
        "expr_ops.tcl",
        "profiler.tcl",
        "command_mocks.tcl",
    ] {
        let _ = writeln!(out, "source [file join $script_dir {f}]");
    }
    out.push_str("if {[file exists [file join $script_dir _mock_stubs.tcl]]} {\n");
    out.push_str("    source [file join $script_dir _mock_stubs.tcl]\n");
    out.push_str("}\n");
    out.push_str("source [file join $script_dir itest_core.tcl]\n");
    out.push_str("source [file join $script_dir orchestrator.tcl]\n\n");

    out.push_str("# \u{2500}\u{2500} Configure test defaults \u{2500}\u{2500}\n\n");
    out.push_str("::orch::configure_tests \\\n");
    let _ = writeln!(out, "    -profiles {{{}}} \\", ctx.profiles.join(" "));
    let _ = writeln!(out, "    -irule {}", list_element(ctx.source));

    let setup_lines = build_setup_lines(ctx.objects, ctx.variables);
    if !setup_lines.is_empty() {
        out.push_str("\n::orch::configure_tests -setup {\n");
        for line in &setup_lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("}\n");
    }

    out.push('\n');
    for block in build_all_test_blocks(test_name, ctx) {
        out.push_str(&unverified_generated_assertions(&block));
        out.push_str("\n\n");
    }

    if ctx.multi_tmm {
        out.push_str(&build_multi_tmm_block(test_name, ctx));
        out.push_str("\n\n");
    }

    out.push_str("# \u{2500}\u{2500} Summary \u{2500}\u{2500}\n\n");
    out.push_str("exit [::orch::done]\n");
    out
}

// Generated guesses remain comments. Original user source is never processed.
fn unverified_generated_assertions(block: &str) -> String {
    let mut output = String::new();
    for line in block.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("::orch::assert_that ") || trimmed.starts_with("::orch::assert ") {
            let indent = &line[..line.len() - trimmed.len()];
            let _ = writeln!(
                output,
                "{indent}# Outcome obligation: supply an independently observed expectation."
            );
            let _ = writeln!(output, "{indent}# {trimmed}");
        } else {
            let _ = writeln!(output, "{line}");
        }
    }
    output
}

/// Build the shared `-setup` body lines (`_build_setup_lines`).
fn build_setup_lines(objects: &ObjectRefs, variables: &Variables) -> Vec<String> {
    let mut lines = Vec::new();
    for pool in &objects.pools {
        lines.push(format!(
            "    ::orch::add_pool {} {{{{10.0.0.1:80}} {{10.0.0.2:80}}}}",
            list_element(pool)
        ));
    }
    for dg in &objects.datagroups {
        lines.push(format!(
            "    ::orch::add_datagroup {} string {{",
            list_element(dg)
        ));
        lines.push("        \"example_key\" \"example_value\"".to_owned());
        lines.push("    }".to_owned());
    }
    for var in &variables.static_vars {
        lines.push(format!(
            "    ::orch::configure_static {} \"\"",
            list_element(var)
        ));
    }
    lines
}

/// Build every test-case block (`_build_all_test_blocks`).
fn build_all_test_blocks(test_name: &str, ctx: &ScriptContext) -> Vec<String> {
    let mut blocks = Vec::new();

    if ctx.cfg_paths.is_empty() {
        // Fallback: template-based tests from command heuristics.
        blocks.push("# Test cases".to_owned());
        let has_http = ctx.profiles.iter().any(|p| p == "HTTP");
        let has_dns = ctx.profiles.iter().any(|p| p == "DNS");
        if has_http {
            blocks.extend(build_http_test_blocks(
                test_name,
                ctx.ordered_events,
                ctx.commands_used,
                ctx.objects,
                ctx.request_syntax.map(|syntax| syntax.0),
            ));
        } else if has_dns {
            blocks.push(build_dns_test_block(test_name));
        } else {
            blocks.push(build_tcp_test_block(test_name));
        }
        return blocks;
    }

    // CFG-informed: generate tests based on actual control-flow paths.
    blocks.push(
        "# CFG-informed test cases#\n\
         # Generated from control flow analysis of the iRule.\n\
         # Each test targets a specific branch path through the code."
            .to_owned(),
    );

    // Group paths by event, preserving first-seen order.
    let mut order: Vec<String> = Vec::new();
    let mut by_event: std::collections::HashMap<String, Vec<&Value>> =
        std::collections::HashMap::new();
    for p in ctx.cfg_paths {
        if p.get("source_kind").and_then(Value::as_str) != Some("event") {
            continue;
        }
        let event = p
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if !by_event.contains_key(&event) {
            order.push(event.clone());
        }
        by_event.entry(event).or_default().push(p);
    }

    let mut test_idx = 0;
    for event_name in &order {
        for path in &by_event[event_name] {
            test_idx += 1;
            let desc = build_test_description(path);
            let body = build_test_body(event_name, path, ctx.request_syntax);
            blocks.push(render_test_case(
                &format!("{test_name}-cfg-{test_idx}.0"),
                &desc,
                &body,
            ));
        }
    }
    blocks
}

/// Render a single `::orch::test` block (`test_case.tcl.j2`).
fn render_test_case(test_id: &str, desc: &str, body: &[String]) -> String {
    let mut out = format!(
        "::orch::test \"{test_id}\" {} -body {{\n",
        list_element(desc)
    );
    for line in body {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('}');
    out
}

/// Sanitise a CFG path into a test description (`_build_test_description`).
fn build_test_description(path: &Value) -> String {
    let cmd = path
        .get("action")
        .and_then(|a| a.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let conditions = path.get("conditions").and_then(Value::as_array);

    let mut parts: Vec<String> = Vec::new();
    for c in conditions.into_iter().flatten() {
        match c.get("kind").and_then(Value::as_str) {
            Some("if") => {
                parts.push(cond_field(c, "condition").to_owned());
            }
            Some("switch") => {
                parts.push(format!(
                    "{} = {}",
                    cond_field(c, "subject"),
                    cond_field(c, "pattern")
                ));
            }
            _ => {}
        }
    }

    let mut desc = if parts.is_empty() {
        format!("{cmd} (unconditional)")
    } else {
        format!("{cmd} when {}", parts.join(" and "))
    };
    if desc.chars().count() > 80 {
        desc = desc.chars().take(77).collect::<String>() + "...";
    }
    desc
}

fn cond_field<'a>(c: &'a Value, field: &str) -> &'a str {
    c.get(field).and_then(Value::as_str).unwrap_or("")
}

/// Build the inner body lines for a CFG test case (`_build_test_body`).
fn build_test_body(
    event_name: &str,
    path: &Value,
    syntax: Option<(LexerConfig, ExprParseContext)>,
) -> Vec<String> {
    let action = path.get("action");
    let cmd = action
        .and_then(|a| a.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let args: Vec<String> = action
        .and_then(|a| a.get("args"))
        .and_then(Value::as_array)
        .map_or_default(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        });
    let conditions: Vec<&Value> = path
        .get("conditions")
        .and_then(Value::as_array)
        .map_or_default(|a| a.iter().collect());

    let mut body: Vec<String> = Vec::new();
    if !conditions.is_empty() {
        body.push("# Path conditions:".to_owned());
        for c in &conditions {
            match c.get("kind").and_then(Value::as_str) {
                Some("if") => body.push(format!(
                    "#   if {} ({})",
                    cond_field(c, "condition"),
                    cond_field(c, "branch")
                )),
                Some("switch") => body.push(format!(
                    "#   switch {} matches {}",
                    cond_field(c, "subject"),
                    cond_field(c, "pattern")
                )),
                _ => {}
            }
        }
        body.push(String::new());
    }

    body.extend(build_request_setup(event_name, &conditions, syntax));
    body.push(String::new());
    body.extend(build_assertion(cmd, &args));
    body
}

/// Source action labels are questions, never predicted observed outcomes.
fn build_assertion(cmd: &str, args: &[String]) -> Vec<String> {
    let label = format!("{cmd} {}", args.join(" "))
        .replace('\r', "\\r")
        .replace('\n', "\\n");
    vec![
        "# Outcome obligation: observe this scenario before adding an assertion.".to_owned(),
        format!("# Source candidate: {label}"),
    ]
}

/// Build conditional request examples under the actual captured source grammar.
fn build_request_setup(
    event_name: &str,
    conditions: &[&Value],
    syntax: Option<(LexerConfig, ExprParseContext)>,
) -> Vec<String> {
    use tcl_syntax::expr::{BinOp, ExprNode};
    let Some((config, context)) = syntax else {
        return Vec::new();
    };
    let mut host_hint: Option<String> = None;
    let mut uri_hint: Option<String> = None;
    let mut header_hints: Vec<String> = Vec::new();

    for condition in conditions {
        if condition.get("kind").and_then(Value::as_str) == Some("switch") {
            let subject = cond_field(condition, "subject");
            let pattern = cond_field(condition, "pattern");
            let lower = subject.to_lowercase();
            let value = cfg_pattern_to_value(pattern);
            if lower.contains("host") {
                host_hint = value;
            } else if lower.contains("uri") || subject.contains("HTTP::path") {
                uri_hint = value;
            }
            continue;
        }
        let source = cond_field(condition, "condition");
        let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(source, &context);
        let ExprNode::Binary { op, left, right } = &parsed else {
            continue;
        };
        if let Some(header) = source_header_hint(left, config) {
            header_hints.push(header);
        }
        let ExprNode::String { text, .. } = right.as_ref() else {
            continue;
        };
        let Some(value) = sole_source_word(text, config.nested())
            .and_then(|word| tcl_syntax::word_rules::original_static_word_unicode_value(&word))
            .and_then(|value| String::from_utf8(value).ok())
        else {
            continue;
        };
        // Source labels select heuristic example categories, not command identity
        // or a claim that this input reaches or satisfies the path.
        let (ExprNode::Command { text: subject, .. }
        | ExprNode::Var { text: subject, .. }
        | ExprNode::String { text: subject, .. }) = left.as_ref()
        else {
            continue;
        };
        let lower = subject.to_lowercase();
        let host_related = subject.contains("HTTP::host") || lower.contains("host");
        let uri_related = subject.contains("HTTP::uri")
            || subject.contains("HTTP::path")
            || lower.contains("uri")
            || lower.contains("path");
        if host_related && !uri_related && host_hint.is_none() && *op == BinOp::StrEq {
            host_hint = Some(value.clone());
        }
        if uri_related
            && uri_hint.is_none()
            && matches!(
                op,
                BinOp::StrEq | BinOp::StartsWith | BinOp::MatchesGlob | BinOp::MatchesRegex
            )
        {
            uri_hint = Some(synthesize_uri_example(op.as_str(), &value));
        }
    }

    let mut result = Vec::new();
    if event_name == "HTTP_REQUEST" || event_name == "HTTP_RESPONSE" || event_name.contains("HTTP")
    {
        let host = host_hint.as_deref().unwrap_or("example.com");
        let uri = uri_hint.as_deref().unwrap_or("/");
        if let (Some(host), Some(uri)) = (
            request_literal_word(host, config),
            request_literal_word(uri, config),
        ) {
            result.push(format!("::orch::run_http_request -host {host} -uri {uri}"));
        }
        for header in header_hints {
            if let Some(header) = request_literal_word(&header, config) {
                result.push(format!("# Ensure header: {header} = test-value"));
            }
        }
    } else if event_name == "DNS_REQUEST" || event_name == "DNS_RESPONSE" {
        result.push("set ::state::dns::qname \"example.com\"".to_owned());
        result.push("set ::state::dns::qtype \"A\"".to_owned());
        result.push(format!("::itest::fire_event {event_name}"));
    } else if event_name == "CLIENT_ACCEPTED" || event_name == "SERVER_CONNECTED" {
        result.push("::orch::configure -client_addr \"10.0.0.1\"".to_owned());
        result.push(format!("::itest::fire_event {event_name}"));
    } else {
        result.push(format!("::itest::fire_event {event_name}"));
    }
    result
}

fn sole_source_word(source: &str, config: LexerConfig) -> Option<tcl_lexer::NativeWord> {
    let parsed = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    let [command] = parsed.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    (parsed.fatal_tail.is_none()
        && !word.group().expand
        && word.written_bytes() == source.as_bytes())
    .then(|| word.clone())
}

/// Keep a proposed word literal both alone and inside the generated braced script.
fn request_literal_word(value: &str, config: LexerConfig) -> Option<String> {
    let quoted = tcl_syntax::backslash::literal_quoted_source_fragment(value, config.escapes)
        .map(|content| format!("\"{content}\""));
    for proposal in quoted
        .into_iter()
        .chain(std::iter::once(list_element(value)))
    {
        let Some(word) = sole_source_word(&proposal, config.nested()) else {
            continue;
        };
        if tcl_syntax::word_rules::original_static_word_unicode_value(&word).as_deref()
            != Some(value.as_bytes())
        {
            continue;
        }
        let container = format!("{{{proposal}}}");
        let Some(container_word) = sole_source_word(&container, config.nested()) else {
            continue;
        };
        if tcl_syntax::word_rules::original_static_word_unicode_value(&container_word).as_deref()
            == Some(proposal.as_bytes())
        {
            return Some(proposal);
        }
    }
    None
}

/// Read a literal header-name operand from the actual command-substitution syntax.
/// The named source vocabulary is a scaffold hint, never a resolved command.
fn source_header_hint(parsed: &tcl_syntax::expr::ExprNode, config: LexerConfig) -> Option<String> {
    let tcl_syntax::expr::ExprNode::Command { text, .. } = parsed else {
        return None;
    };
    let word = sole_source_word(text, config.nested())?;
    let arena = word.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let tcl_lexer::ExecutablePart::Command { body } = &part.part else {
        return None;
    };
    let parsed =
        tcl_lexer::native_script_words_in(word.image().clone(), *body, config.nested()).ok()?;
    let [command] = parsed.commands.as_slice() else {
        return None;
    };
    let [head, name] = command.words.as_slice() else {
        return None;
    };
    if parsed.fatal_tail.is_some()
        || head.group().expand
        || name.group().expand
        || tcl_syntax::word_rules::original_static_word_unicode_value(head)?.as_slice()
            != b"HTTP::header"
    {
        return None;
    }
    String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        name,
    )?)
    .ok()
}

/// Build a concrete example URI that actually satisfies a condition using
/// `op`, given the raw pattern text extracted from it.
///
/// `eq`/`starts_with` patterns are already literal strings that trivially
/// satisfy themselves, so they pass through unchanged. `matches_glob`/
/// `matches_regex` patterns are not: reusing `"^/api/.*$"` (a regex) or
/// `"/api/*"` (a glob) as the literal simulated request URI produces a
/// generated test whose own simulated request doesn't satisfy the very
/// condition its branch exercises (confirmed against tclsh8.6:
/// `regexp {^/api/.*$} {^/api/.*$}` is false — the literal pattern text
/// starts with `^`, not `/api/`). This substitutes each wildcard with a
/// concrete placeholder instead: `matches_glob`'s `*`/`?` each become `x`;
/// `matches_regex`'s anchors (`^`/`$`) are stripped and its `.*`/`.+`/`.`
/// spans each become `x` too. This is a heuristic, not a full glob/regex
/// solver — good enough for the common single-wildcard URI patterns this
/// generator actually sees, not a claim to handle arbitrary regex.
fn synthesize_uri_example(op: &str, pattern: &str) -> String {
    use tcl_syntax::expr::ast::BinOp;
    if op == BinOp::MatchesGlob.as_str() {
        pattern
            .chars()
            .map(|c| if matches!(c, '*' | '?') { 'x' } else { c })
            .collect()
    } else if op == BinOp::MatchesRegex.as_str() {
        let stripped = pattern.strip_prefix('^').unwrap_or(pattern);
        let stripped = stripped.strip_suffix('$').unwrap_or(stripped);
        let mut out = String::with_capacity(stripped.len());
        let mut chars = stripped.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '.' {
                while matches!(chars.peek(), Some('*' | '+')) {
                    chars.next();
                }
                out.push('x');
            } else {
                out.push(c);
            }
        }
        out
    } else {
        pattern.to_owned()
    }
}

/// Convert a switch glob/regex pattern to a concrete test value
/// (`_cfg_pattern_to_value`). Returns `None` for the `default` pattern.
fn cfg_pattern_to_value(pattern: &str) -> Option<String> {
    if pattern == "default" {
        return None;
    }
    if pattern.contains('*') {
        return Some(pattern.replace('*', "test"));
    }
    if pattern.contains('?') {
        return Some(pattern.replace('?', "x"));
    }
    Some(pattern.to_owned())
}

/// Build the fallback HTTP test-case blocks (`_build_http_test_blocks`).
fn build_http_test_blocks(
    test_name: &str,
    events: &[String],
    commands: &[String],
    objects: &ObjectRefs,
    config: Option<LexerConfig>,
) -> Vec<String> {
    let has = |c: &str| commands.iter().any(|x| x == c);
    let mut blocks = Vec::new();
    // This remains a heuristic expected outcome. Encoding only preserves the
    // original literal object name and the generated braced script container.
    let pool = if has("pool") {
        objects
            .pools
            .first()
            .and_then(|value| request_literal_word(value, config?))
    } else {
        None
    };

    // Test 1: happy path.
    let mut body = vec!["::orch::run_http_request -host \"example.com\" -uri \"/\"".to_owned()];
    if let Some(pool) = &pool {
        body.push(format!("::orch::assert_that pool_selected equals {pool}"));
    } else {
        body.push("# ::orch::assert_that pool_selected equals \"your_pool\"".to_owned());
    }
    if has("reject") || has("drop") {
        body.push("::orch::assert_that decision connection reject was_not_called".to_owned());
    }
    blocks.push(render_test_case(
        &format!("{test_name}-1.0"),
        "basic request routing",
        &body,
    ));

    // Test 2: headers.
    if commands.iter().any(|c| c.starts_with("HTTP::header")) {
        blocks.push(render_test_case(
            &format!("{test_name}-1.1"),
            "header manipulation",
            &[
                "::orch::run_http_request -host \"example.com\" -uri \"/\"".to_owned(),
                "# ::orch::assert_that http_header \"X-Custom\" equals \"value\"".to_owned(),
            ],
        ));
    }

    // Test 3: edge case.
    blocks.push(render_test_case(
        &format!("{test_name}-2.0"),
        "empty host handling",
        &[
            "::orch::run_http_request -host \"\" -uri \"/\"".to_owned(),
            "# ::orch::assert_that pool_selected equals \"default_pool\"".to_owned(),
        ],
    ));

    // Test 4: rejection path.
    if has("reject") || has("drop") {
        let mut body = vec![
            "::orch::run_http_request -host \"evil.com\" -uri \"/\"".to_owned(),
            "::orch::assert_that decision connection reject was_called".to_owned(),
        ];
        if has("log") {
            body.push("# ::orch::assert_that log matches \"*rejected*\"".to_owned());
        }
        blocks.push(render_test_case(
            &format!("{test_name}-2.1"),
            "rejects bad requests",
            &body,
        ));
    }

    // Test 5: redirect.
    if has("HTTP::redirect") {
        blocks.push(render_test_case(
            &format!("{test_name}-3.0"),
            "redirect behavior",
            &[
                "::orch::run_http_request -host \"example.com\" -uri \"/\"".to_owned(),
                "::orch::assert_that decision http redirect was_called".to_owned(),
            ],
        ));
    }

    // Test 6: keep-alive.
    if events.iter().any(|e| e == "HTTP_RESPONSE") {
        let mut body =
            vec!["::orch::run_http_request -host \"example.com\" -uri \"/first\"".to_owned()];
        if let Some(p) = &pool {
            body.push(format!("::orch::assert_that pool_selected equals {p}"));
        }
        body.push(String::new());
        body.push("::orch::run_next_request -host \"example.com\" -uri \"/second\"".to_owned());
        if let Some(p) = &pool {
            body.push(format!("::orch::assert_that pool_selected equals {p}"));
        }
        body.push(String::new());
        body.push("::orch::close_connection".to_owned());
        blocks.push(render_test_case(
            &format!("{test_name}-4.0"),
            "keep-alive multiple requests",
            &body,
        ));
    }

    blocks
}

/// Build the fallback DNS test-case block (`_build_dns_test_blocks`).
fn build_dns_test_block(test_name: &str) -> String {
    render_test_case(
        &format!("{test_name}-1.0"),
        "DNS query handling",
        &[
            "set ::state::dns::qname \"example.com\"".to_owned(),
            "set ::state::dns::qtype \"A\"".to_owned(),
            "::itest::fire_event DNS_REQUEST".to_owned(),
            "# ::orch::assert_that decision dns return was_called".to_owned(),
        ],
    )
}

/// Build the fallback TCP test-case block (`_build_tcp_test_blocks`).
fn build_tcp_test_block(test_name: &str) -> String {
    render_test_case(
        &format!("{test_name}-1.0"),
        "TCP connection handling",
        &[
            "::orch::configure -client_addr \"10.0.0.1\"".to_owned(),
            "::itest::fire_event CLIENT_ACCEPTED".to_owned(),
            "::orch::assert_that decision connection reject was_not_called".to_owned(),
        ],
    )
}

/// Build the multi-TMM test section (`multi_tmm.tcl.j2` +
/// `_build_multi_tmm_context`).
fn build_multi_tmm_block(test_name: &str, ctx: &ScriptContext) -> String {
    // Pick the first static var that isn't a well-known "config" name.
    let check_var = ctx
        .variables
        .static_vars
        .iter()
        .find(|v| !matches!(v.as_str(), "rate_limit" | "mode" | "version" | "debug"));

    let mut out = String::new();
    out.push_str("# \u{2500}\u{2500} Multi-TMM tests (fakeCMP distribution) \u{2500}\u{2500}\n");
    out.push_str("#\n");
    out.push_str("# Source patterns suggest a multi-TMM scenario; worker effects are unproved.\n");
    out.push_str("# Independently observe expected outcomes in 4 simulated TMMs.\n");
    out.push_str("# fakeCMP hashes (src_ip, src_port, dst_ip, dst_port) to pick TMM.\n\n");

    out.push_str("::orch::configure_tests \\\n");
    out.push_str("    -tmm_count 4 \\\n");
    out.push_str("    -tmm_select auto \\\n");
    let _ = writeln!(out, "    -profiles {{{}}} \\", ctx.profiles.join(" "));
    let _ = writeln!(out, "    -irule {}", list_element(ctx.source));

    if !ctx.objects.pools.is_empty() {
        out.push_str("\n::orch::configure_tests -setup {\n");
        for pool in &ctx.objects.pools {
            let _ = writeln!(
                out,
                "    ::orch::add_pool {} {{{{10.0.0.1:80}} {{10.0.0.2:80}}}}",
                list_element(pool)
            );
        }
        out.push_str("}\n");
    }

    out.push('\n');
    let _ = writeln!(
        out,
        "::orch::test \"{test_name}-multi-1.0\" \"fakeCMP distributes clients across TMMs\" -body {{"
    );
    out.push_str("    # Use fakecmp_suggest_sources to get a source per TMM\n");
    out.push_str("    set plan [::orch::fakecmp_suggest_sources -count 3]\n");
    out.push_str("    foreach tmm_id [::orch::tmm_ids] {\n");
    out.push_str("        set sources [dict get $plan $tmm_id]\n");
    out.push_str("        foreach {addr port} $sources {\n");
    out.push_str("            ::orch::configure -client_addr $addr -client_port $port\n");
    out.push_str("            ::orch::run_http_request -host app.example.com\n");
    out.push_str("        }\n");
    out.push_str("    }\n\n");
    out.push_str("    # Observe mock per-worker state; source metadata does not predict it.\n");
    out.push_str("    set active 0\n");
    out.push_str("    foreach tmm_id [::orch::tmm_ids] {\n");
    if let Some(var) = check_var {
        let _ = writeln!(
            out,
            "        set val [::orch::tmm_get_static $tmm_id {}]",
            list_element(var)
        );
        out.push_str("        if {$val ne \"\"} { incr active }\n");
    } else {
        out.push_str("        # TODO: add per-TMM state check here\n");
        out.push_str("        incr active\n");
    }
    out.push_str("    }\n");
    out.push_str(
        "    # Outcome obligation: independently observe per-worker state before asserting it.\n",
    );
    let _ = writeln!(
        out,
        "        # Observation candidate: {test_name}-multi-1.0; $active mock TMMs have visible state"
    );
    out.push('}');
    out
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_request_source_tests;

#[cfg(test)]
mod fallback_pool_source_tests;

#[cfg(test)]
mod configured_request_literal_value_tests;
