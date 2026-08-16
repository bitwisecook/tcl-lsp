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

use regex::Regex;
use serde_json::{Value, json};
use tcl_registry::events::EventRegistry;

/// The prose emitted for `multi_tmm_hint` when multi-TMM patterns are detected.
const MULTI_TMM_HINT: &str = "This iRule uses patterns that behave differently across TMMs \
     (static:: writes in hot events, counters, or shared table state). \
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

/// Events implying a CLIENTSSL profile (`_infer_profiles`).
const SSL_EVENTS: &[&str] = &[
    "CLIENTSSL_HANDSHAKE",
    "CLIENTSSL_DATA",
    "CLIENTSSL_CLIENTCERT",
    "CLIENTSSL_CLIENTHELLO",
    "SERVERSSL_HANDSHAKE",
    "SERVERSSL_DATA",
    "SERVERSSL_SERVERHELLO",
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

/// MCP tool entry point: analyse the `source` argument and return the
/// `generate_irule_test` wire shape (the `test_script` is a
/// functionally-equivalent runnable Tcl scaffold).
#[must_use]
pub fn generate_irule_test(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");

    // Discover handlers once at the request boundary. Lower-level registry
    // code only orders these owner-derived names and never re-scans source.
    let when_blocks = tcl_irules::when_blocks(source);
    let event_registry = EventRegistry::build();
    let mut seen_events = BTreeSet::new();
    let event_names: Vec<String> = when_blocks
        .iter()
        .map(|block| block.event.clone())
        .filter(|event| event_registry.is_known(event))
        .filter(|event| seen_events.insert(event.clone()))
        .collect();
    let ordered_events = event_registry.order_events(&event_names);
    let profiles = infer_profiles(&ordered_events);
    let registry = tcl_registry::registry_for_profile(tcl_dialect::DialectProfile::irules());
    // Build the event-rooted executable closure once for this request.  Every
    // execution-sensitive output below consumes this exact proof of liveness;
    // re-building it for object references could make setup disagree with the
    // command/static/multi-TMM inventory after either walker evolves.
    let closure = tcl_irules::irules_executable_commands(source, registry);
    let commands_used = extract_irule_commands(&closure);
    let objects = extract_object_refs(source, registry, &closure);
    let variables = extract_variables(&closure);

    let cfg_paths = crate::irule_test::cfg_paths_json(source);
    let multi_tmm = needs_multi_tmm(&closure, &variables);

    let ctx = ScriptContext {
        basename: "irule.tcl",
        source,
        ordered_events: &ordered_events,
        profiles: &profiles,
        commands_used: &commands_used,
        objects: &objects,
        variables: &variables,
        cfg_paths: &cfg_paths,
        multi_tmm,
    };
    let test_script = build_test_script(&ctx);

    json!({
        "test_script": test_script,
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
                "CFG analysis found {} unique paths to terminal actions. \
                 The generated test includes a test case per path. Use \
                 irule_cfg_paths to inspect paths individually for deeper analysis.",
                cfg_paths.len()
            ))
        },
    })
}

// ── Extractors ────────────────────────────────────────────────────────

/// Extract executable iRule command identities, sorted and deduplicated.
fn extract_irule_commands(commands: &[tcl_irules::IrulesExecutableCommand]) -> Vec<String> {
    commands
        .iter()
        .map(|fact| fact.command.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Extract pool / data-group references from this request's executable closure.
fn extract_object_refs(
    source: &str,
    registry: &tcl_registry::CommandRegistry,
    closure: &[tcl_irules::IrulesExecutableCommand],
) -> ObjectRefs {
    let mut pools: BTreeSet<String> = BTreeSet::new();
    let mut datagroups: BTreeSet<String> = BTreeSet::new();
    // The shared reference owner consumes the event-rooted executable closure
    // itself, so invalid top-level/nested declarations and dormant procedures
    // cannot seed test setup.  Do not reclassify reachability by checking
    // whether a reference happens to sit inside a handler's physical body:
    // reached helper procedures live outside that body.
    for reference in
        tcl_irules::extract_irules_object_references_in_closure(source, None, registry, closure)
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

/// Extract `static::` variable names (`_extract_variables`), sorted + deduped.
fn extract_variables(commands: &[tcl_irules::IrulesExecutableCommand]) -> Variables {
    let mut static_vars: BTreeSet<String> = BTreeSet::new();
    for fact in commands {
        if fact.command == "set"
            && let Some(name) = fact.args.first()
            && name.starts_with("static::")
        {
            static_vars.insert(name.trim_start_matches("static::").to_owned());
        }
        for name in &fact.variable_names {
            if name.starts_with("static::") {
                static_vars.insert(name.trim_start_matches("static::").to_owned());
            }
        }
    }
    Variables {
        static_vars: static_vars.into_iter().collect(),
    }
}

/// Infer the required profiles from the events present (`_infer_profiles`).
fn infer_profiles(events: &[String]) -> Vec<String> {
    let mut profiles = vec!["TCP".to_owned()];
    let has = |set: &[&str]| events.iter().any(|e| set.contains(&e.as_str()));

    let has_http = has(HTTP_EVENTS);
    if has_http {
        profiles.push("HTTP".to_owned());
    }
    if has(SSL_EVENTS) {
        profiles.insert(1, "CLIENTSSL".to_owned());
    }
    if has(DNS_EVENTS) {
        return vec!["UDP".to_owned(), "DNS".to_owned()];
    }
    if has(UDP_EVENTS) && !has_http {
        return vec!["UDP".to_owned()];
    }
    profiles
}

/// Detect whether an iRule should be tested in multi-TMM mode
/// (`_needs_multi_tmm`).
fn needs_multi_tmm(
    commands: &[tcl_irules::IrulesExecutableCommand],
    variables: &Variables,
) -> bool {
    let has_static_vars = !variables.static_vars.is_empty();
    let static_in_hot = commands.iter().any(|fact| {
        fact.event
            .as_deref()
            .is_some_and(|event| HOT_EVENTS.contains(&event))
            && matches!(fact.command.as_str(), "set" | "incr")
            && fact
                .args
                .first()
                .is_some_and(|arg| arg.starts_with("static::"))
    });
    let has_counter = commands.iter().any(|fact| {
        fact.command == "incr"
            && fact.args.first().is_some_and(|arg| {
                arg.starts_with("static::") || arg.to_ascii_lowercase().contains("count")
            })
    });
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
    out.push_str("    -irule {\n");
    push_indented(&mut out, ctx.source.trim(), 8);
    out.push_str("    }\n");

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
        out.push_str(&block);
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

/// Append `text`, prefixing every line with `n` spaces (empty lines stay empty).
fn push_indented(out: &mut String, text: &str, n: usize) {
    let pad = " ".repeat(n);
    for line in text.split('\n') {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&pad);
            out.push_str(line);
            out.push('\n');
        }
    }
}

/// Build the shared `-setup` body lines (`_build_setup_lines`).
fn build_setup_lines(objects: &ObjectRefs, variables: &Variables) -> Vec<String> {
    let mut lines = Vec::new();
    for pool in &objects.pools {
        lines.push(format!(
            "    ::orch::add_pool {pool} {{{{10.0.0.1:80}} {{10.0.0.2:80}}}}"
        ));
    }
    for dg in &objects.datagroups {
        lines.push(format!("    ::orch::add_datagroup {dg} string {{"));
        lines.push("        \"example_key\" \"example_value\"".to_owned());
        lines.push("    }".to_owned());
    }
    for var in &variables.static_vars {
        lines.push(format!("    ::orch::configure_static {var} \"\""));
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
        if event_name.starts_with("proc:") {
            continue;
        }
        for path in &by_event[event_name] {
            test_idx += 1;
            let desc = build_test_description(path);
            let body = build_test_body(event_name, path);
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
    let mut out = format!("::orch::test \"{test_id}\" \"{desc}\" -body {{\n");
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
    desc = desc.replace('"', "'").replace('\\', "");
    if desc.chars().count() > 80 {
        desc = desc.chars().take(77).collect::<String>() + "...";
    }
    desc
}

fn cond_field<'a>(c: &'a Value, field: &str) -> &'a str {
    c.get(field).and_then(Value::as_str).unwrap_or("")
}

/// Build the inner body lines for a CFG test case (`_build_test_body`).
fn build_test_body(event_name: &str, path: &Value) -> Vec<String> {
    let action = path.get("action");
    let cmd = action
        .and_then(|a| a.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let args: Vec<String> = action
        .and_then(|a| a.get("args"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let conditions: Vec<&Value> = path
        .get("conditions")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default();

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

    body.extend(build_request_setup(event_name, &conditions));
    body.push(String::new());
    body.extend(build_assertion(cmd, &args));
    body
}

/// Build assertion lines for a terminal action (`_build_assertion`).
fn build_assertion(cmd: &str, args: &[String]) -> Vec<String> {
    match cmd {
        "pool" if !args.is_empty() => {
            vec![format!(
                "::orch::assert_that pool_selected equals \"{}\"",
                args[0]
            )]
        }
        "reject" => vec!["::orch::assert_that decision connection reject was_called".to_owned()],
        "drop" | "discard" => {
            vec![format!(
                "::orch::assert_that decision connection {cmd} was_called"
            )]
        }
        "HTTP::redirect" => {
            vec!["::orch::assert_that decision http redirect was_called".to_owned()]
        }
        "HTTP::respond" => vec![
            "# Verify HTTP::respond was called".to_owned(),
            "::orch::assert_that decision http respond was_called".to_owned(),
        ],
        "node" if !args.is_empty() => vec![
            format!("# Verify node selection: {}", args.join(" ")),
            format!("::orch::assert_that node_selected equals \"{}\"", args[0]),
        ],
        _ => vec![format!("# Verify: {cmd} {}", args.join(" "))],
    }
}

/// Build request-setup lines based on path conditions (`_build_request_setup`).
fn build_request_setup(event_name: &str, conditions: &[&Value]) -> Vec<String> {
    // The word operators real iRules use are `matches_glob`/`matches_regex`
    // (`tcl_syntax::expr::ast::BinOp`'s `MatchesGlob`/`MatchesRegex`), never a
    // bare "matches", so a condition like `HTTP::uri matches_glob "/api/*"`
    // must yield "/api/*" rather than falling through to the bare-value
    // fallback below. Derived from `BinOp` rather than hand-typed so the
    // spelling cannot drift.
    let uri_ops = [
        tcl_syntax::expr::ast::BinOp::StrEq.as_str(),
        tcl_syntax::expr::ast::BinOp::StartsWith.as_str(),
        tcl_syntax::expr::ast::BinOp::MatchesGlob.as_str(),
        tcl_syntax::expr::ast::BinOp::MatchesRegex.as_str(),
    ]
    .join("|");
    let host_re = Regex::new(r#"eq\s+"([^"]+)""#).expect("valid regex");
    let host_bare_re = Regex::new(r"eq\s+(\S+)").expect("valid regex");
    // Captures the operator too (group 1), not just the pattern (group 2):
    // `matches_glob`/`matches_regex` need the pattern *synthesized* into a
    // concrete example (see `synthesize_uri_example`), not reused verbatim.
    let uri_re = Regex::new(&format!(r#"({uri_ops})\s+"([^"]+)""#)).expect("valid regex");
    let uri_bare_re = Regex::new(&format!(r"({uri_ops})\s+(\S+)")).expect("valid regex");
    let header_re = Regex::new(r#"HTTP::header\s+"?([^"\s]+)"?"#).expect("valid regex");

    let mut host_hint: Option<String> = None;
    let mut uri_hint: Option<String> = None;
    let mut header_hints: Vec<String> = Vec::new();

    for c in conditions {
        let kind = c.get("kind").and_then(Value::as_str);
        if kind == Some("switch") {
            let subject = cond_field(c, "subject");
            let pattern = cond_field(c, "pattern");
            let subj_lower = subject.to_lowercase();
            let value = cfg_pattern_to_value(pattern);
            if subj_lower.contains("host") {
                host_hint = value;
            } else if subj_lower.contains("uri") || subject.contains("HTTP::path") {
                uri_hint = value;
            }
            continue;
        }

        let cond_text = cond_field(c, "condition");
        let lower = cond_text.to_lowercase();
        let host_related = cond_text.contains("HTTP::host") || lower.contains("host");
        let uri_related = cond_text.contains("HTTP::uri")
            || cond_text.contains("HTTP::path")
            || lower.contains("uri")
            || lower.contains("path");

        if host_related && !uri_related && host_hint.is_none() {
            if let Some(m) = host_re.captures(cond_text) {
                host_hint = Some(m[1].to_owned());
            } else if let Some(m) = host_bare_re.captures(cond_text) {
                let v = &m[1];
                if !v.starts_with('$') {
                    host_hint = Some(trim_quotes_braces(v).to_owned());
                }
            }
        }
        if uri_related && uri_hint.is_none() {
            if let Some(m) = uri_re.captures(cond_text) {
                uri_hint = Some(synthesize_uri_example(&m[1], &m[2]));
            } else if let Some(m) = uri_bare_re.captures(cond_text) {
                let v = &m[2];
                if !v.starts_with('$') {
                    uri_hint = Some(synthesize_uri_example(&m[1], trim_quotes_braces(v)));
                }
            }
        }
        // The regex itself requires "HTTP::header ...", so no separate
        // `contains("HTTP::header")` guard is needed.
        if let Some(m) = header_re.captures(cond_text) {
            header_hints.push(m[1].to_owned());
        }
    }

    let mut result: Vec<String> = Vec::new();
    if event_name == "HTTP_REQUEST" || event_name == "HTTP_RESPONSE" || event_name.contains("HTTP")
    {
        let host = host_hint.as_deref().unwrap_or("example.com");
        let uri = uri_hint.as_deref().unwrap_or("/");
        result.push(format!(
            "::orch::run_http_request -host \"{host}\" -uri \"{uri}\""
        ));
        for hname in &header_hints {
            result.push(format!("# Ensure header: {hname} = test-value"));
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

fn trim_quotes_braces(s: &str) -> &str {
    s.trim_matches(|c| matches!(c, '"' | '{' | '}'))
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
    let pattern = trim_quotes_braces(pattern);
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
) -> Vec<String> {
    let has = |c: &str| commands.iter().any(|x| x == c);
    let mut blocks = Vec::new();

    // Test 1: happy path.
    let mut body = vec!["::orch::run_http_request -host \"example.com\" -uri \"/\"".to_owned()];
    if has("pool") && !objects.pools.is_empty() {
        body.push(format!(
            "::orch::assert_that pool_selected equals \"{}\"",
            objects.pools[0]
        ));
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
        let pool = (has("pool") && !objects.pools.is_empty()).then(|| objects.pools[0].clone());
        if let Some(p) = &pool {
            body.push(format!("::orch::assert_that pool_selected equals \"{p}\""));
        }
        body.push(String::new());
        body.push("::orch::run_next_request -host \"example.com\" -uri \"/second\"".to_owned());
        if let Some(p) = &pool {
            body.push(format!("::orch::assert_that pool_selected equals \"{p}\""));
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
    out.push_str("# The iRule uses patterns that behave differently across TMMs.\n");
    out.push_str("# These tests verify correctness with 4 simulated TMMs.\n");
    out.push_str("# fakeCMP hashes (src_ip, src_port, dst_ip, dst_port) to pick TMM.\n\n");

    out.push_str("::orch::configure_tests \\\n");
    out.push_str("    -tmm_count 4 \\\n");
    out.push_str("    -tmm_select auto \\\n");
    let _ = writeln!(out, "    -profiles {{{}}} \\", ctx.profiles.join(" "));
    out.push_str("    -irule {\n");
    push_indented(&mut out, ctx.source.trim(), 8);
    out.push_str("    }\n");

    if !ctx.objects.pools.is_empty() {
        out.push_str("\n::orch::configure_tests -setup {\n");
        for pool in &ctx.objects.pools {
            let _ = writeln!(
                out,
                "    ::orch::add_pool {pool} {{{{10.0.0.1:80}} {{10.0.0.2:80}}}}"
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
    out.push_str("    # Verify all TMMs received traffic\n");
    out.push_str("    set active 0\n");
    out.push_str("    foreach tmm_id [::orch::tmm_ids] {\n");
    if let Some(var) = check_var {
        let _ = writeln!(
            out,
            "        set val [::orch::tmm_get_static $tmm_id {var}]"
        );
        out.push_str("        if {$val ne \"\"} { incr active }\n");
    } else {
        out.push_str("        # TODO: add per-TMM state check here\n");
        out.push_str("        incr active\n");
    }
    out.push_str("    }\n");
    out.push_str("    ::orch::assert {$active >= 2} \\\n");
    let _ = writeln!(
        out,
        "        \"{test_name}-multi-1.0: only $active TMMs got traffic\""
    );
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_request_closure_builds() {
        tcl_irules::reset_executable_closure_builds_for_tests();
    }

    fn request_closure_builds() -> usize {
        tcl_irules::executable_closure_builds_for_tests()
    }

    fn registry() -> &'static tcl_registry::CommandRegistry {
        tcl_registry::registry_for_profile(tcl_dialect::DialectProfile::irules())
    }

    fn executable(src: &str) -> Vec<tcl_irules::IrulesExecutableCommand> {
        tcl_irules::irules_executable_commands(src, registry())
    }

    fn object_refs(src: &str) -> ObjectRefs {
        let closure = executable(src);
        extract_object_refs(src, registry(), &closure)
    }

    fn if_cond(condition: &str) -> Value {
        json!({"kind": "if", "condition": condition})
    }

    /// The URI hint must recognise the real iRules word operators
    /// `matches_glob`/`matches_regex` — a bare "matches" alternative matches
    /// nothing — and extract the pattern instead of falling through to the
    /// "/" default.
    #[test]
    fn uri_hint_extracts_matches_glob_and_matches_regex() {
        let cond = if_cond(r#"[HTTP::uri] matches_glob "/api/*""#);
        let lines = build_request_setup("HTTP_REQUEST", &[&cond]);
        assert!(
            lines.iter().any(|l| l.contains(r#"-uri "/api/x""#)),
            "{lines:?}"
        );

        let cond = if_cond(r#"[HTTP::uri] matches_regex "^/api/.*$""#);
        let lines = build_request_setup("HTTP_REQUEST", &[&cond]);
        assert!(
            lines.iter().any(|l| l.contains(r#"-uri "/api/x""#)),
            "{lines:?}"
        );
    }

    /// Adversarial-review finding: reusing the raw `matches_glob`/
    /// `matches_regex` pattern text verbatim as the simulated request URI
    /// produces a generated test whose own simulated request doesn't
    /// satisfy the very condition its branch exercises — confirmed against
    /// a real `tclsh8.6`: `regexp {^/api/.*$} {^/api/.*$}` is false (the
    /// pattern text starts with `^`, not `/api/`), and `string match
    /// {/api/*} {/api/*}` happens to be true only by coincidence (a literal
    /// `*` in the subject also matches the glob's own `*`), a degenerate,
    /// unrepresentative example either way. Confirms `synthesize_uri_example`
    /// actually produces a self-consistent example for both operators.
    #[test]
    fn synthesized_uri_actually_satisfies_its_own_condition() {
        let glob_uri = synthesize_uri_example("matches_glob", "/api/*");
        assert!(
            tcl_string_match("/api/*", &glob_uri),
            "{glob_uri:?} must satisfy string match {{/api/*}}"
        );

        let regex_uri = synthesize_uri_example("matches_regex", "^/api/.*$");
        assert!(
            tcl_regex_match(r"^/api/.*$", &regex_uri),
            "{regex_uri:?} must satisfy regexp {{^/api/.*$}}"
        );
    }

    /// A minimal `string match` implementation covering the `*`/`?`
    /// wildcards this test needs — just enough to verify
    /// `synthesize_uri_example`'s glob output against its own pattern
    /// without depending on a real `tclsh`.
    fn tcl_string_match(pattern: &str, subject: &str) -> bool {
        fn go(p: &[char], s: &[char]) -> bool {
            match p.first() {
                None => s.is_empty(),
                Some('*') => go(&p[1..], s) || (!s.is_empty() && go(p, &s[1..])),
                Some('?') => !s.is_empty() && go(&p[1..], &s[1..]),
                Some(c) => s.first() == Some(c) && go(&p[1..], &s[1..]),
            }
        }
        let p: Vec<char> = pattern.chars().collect();
        let s: Vec<char> = subject.chars().collect();
        go(&p, &s)
    }

    /// `regex` crate stand-in for tclsh's `regexp` on the narrow pattern
    /// shape this generator produces (anchors + literal + `.*`) — real
    /// ARE/Tcl regex and Rust `regex` agree on this subset.
    fn tcl_regex_match(pattern: &str, subject: &str) -> bool {
        Regex::new(pattern).expect("valid regex").is_match(subject)
    }

    /// Regression guard: `eq`/`starts_with` (already working before this
    /// fix) must keep working now that the alternation is built from
    /// `BinOp` spellings instead of hand-typed.
    #[test]
    fn uri_hint_still_extracts_eq_and_starts_with() {
        let cond = if_cond(r#"[HTTP::uri] eq "/login""#);
        let lines = build_request_setup("HTTP_REQUEST", &[&cond]);
        assert!(
            lines.iter().any(|l| l.contains(r#"-uri "/login""#)),
            "{lines:?}"
        );

        let cond = if_cond(r#"[HTTP::uri] starts_with "/admin""#);
        let lines = build_request_setup("HTTP_REQUEST", &[&cond]);
        assert!(
            lines.iter().any(|l| l.contains(r#"-uri "/admin""#)),
            "{lines:?}"
        );
    }

    #[test]
    fn extractors_ignore_commented_out_handlers() {
        let src = "# when HTTP_REQUEST { pool stale }\nwhen HTTP_REQUEST { pool live }\n";
        let commands = executable(src);
        assert_eq!(object_refs(src).pools, vec!["live".to_owned()]);
        assert!(!extract_irule_commands(&commands).contains(&"stale".to_owned()));
    }

    #[test]
    fn extractors_ignore_commented_pool_inside_live_handler() {
        let src = "when HTTP_REQUEST {\n # pool stale\n pool live\n}\n";
        assert_eq!(object_refs(src).pools, vec!["live".to_owned()]);
    }

    #[test]
    fn object_refs_ignore_inline_comments_and_pool_like_data() {
        let src = "when HTTP_REQUEST {\n set x 1; # pool stale\n set quoted \"pool quoted\"\n set braced {pool braced}\n pool live\n}\n";
        assert_eq!(object_refs(src).pools, vec!["live".to_owned()]);
    }

    #[test]
    fn object_refs_recurse_into_if_and_command_substitution() {
        let src = "when HTTP_REQUEST { if {1} { pool nested_pool }; set hit [class match [HTTP::uri] equals uri_dg] }";
        let refs = object_refs(src);
        assert_eq!(refs.pools, vec!["nested_pool".to_owned()]);
        assert!(refs.datagroups.contains(&"uri_dg".to_owned()));
    }

    #[test]
    fn generated_test_inventory_keeps_data_groups_from_braced_expressions() {
        let src = "when HTTP_REQUEST { if {[class match [HTTP::uri] equals expr_dg]} { pool selected_pool } }";
        let refs = object_refs(src);
        assert_eq!(refs.pools, ["selected_pool"]);
        assert_eq!(refs.datagroups, ["expr_dg"]);
    }

    #[test]
    fn generated_test_sets_up_live_braced_expression_and_quoted_operand_static_variables() {
        let generated = generate_irule_test(&json!({
            "source": concat!(
                "when HTTP_REQUEST {\n",
                "  if {$static::maintenance} { return }\n",
                "  if {\"$static::quoted\" eq {${static::braced_data}}} {}\n",
                "  if {$static::broken +} {}\n",
                "}\n",
            ),
        }));
        let script = generated["test_script"]
            .as_str()
            .expect("generated test script");
        assert!(
            script.contains("::orch::configure_static maintenance \"\""),
            "a live braced expression read needs generated setup: {script}"
        );
        assert!(
            script.contains("::orch::configure_static quoted \"\""),
            "a quoted operand evaluated by expr needs generated setup: {script}"
        );
        for inert in ["braced_data", "broken"] {
            assert!(
                !script.contains(&format!("::orch::configure_static {inert} ")),
                "inert or malformed expression variable leaked into setup: {script}"
            );
        }
    }

    #[test]
    fn generated_inventory_keeps_quoted_substitutions_before_an_expr_error() {
        // Tcl 8.6 and 9 substitute the quoted word before `if` rejects its
        // trailing operator. The braced form reaches expr as literal source,
        // so its malformed expression must not seed generated setup.
        let generated = generate_irule_test(&json!({
            "source": concat!(
                "when HTTP_REQUEST {\n",
                "  if \"[class match [HTTP::uri] equals quoted_dg] +\" {}\n",
                "  if {[class match [HTTP::host] equals braced_dg] +} {}\n",
                "}\n",
            ),
        }));
        assert_eq!(generated["datagroups"], json!(["quoted_dg"]));
        let commands = generated["commands_used"].as_array().unwrap();
        for command in ["class", "HTTP::uri"] {
            assert!(
                commands.iter().any(|value| value == command),
                "quoted pre-expression command {command:?} missing: {commands:?}"
            );
        }
        assert!(
            !commands.iter().any(|value| value == "HTTP::host"),
            "braced malformed expression must not run substitutions: {commands:?}"
        );
    }

    #[test]
    fn object_refs_ignore_unavailable_aliases_and_keep_qualified_commands() {
        let src = "interp alias {} choose {} pool\nrename class group\nwhen HTTP_REQUEST { choose aliased_pool; ::pool qualified_pool; group match x equals aliased_dg }";
        let refs = object_refs(src);
        assert!(!refs.pools.contains(&"aliased_pool".to_owned()));
        assert!(refs.pools.contains(&"qualified_pool".to_owned()));
        assert!(!refs.datagroups.contains(&"aliased_dg".to_owned()));
    }

    #[test]
    fn executable_inventory_ignores_comments_and_data_but_keeps_live_substitutions() {
        let inert = "when HTTP_REQUEST {\n # set static::ghost 1; HTTP::respond 200; pool stale; table incr key\n set q {HTTP::respond 201; pool inert; table incr key; set static::data 1}\n set r \"pool quoted; set static::quoted 1\"\n set x [HTTP::uri]\n pool live\n}\n";
        let commands = executable(inert);
        let names = extract_irule_commands(&commands);
        assert!(names.contains(&"HTTP::uri".to_owned()), "{names:?}");
        assert!(names.contains(&"pool".to_owned()), "{names:?}");
        assert!(!names.contains(&"HTTP::respond".to_owned()), "{names:?}");
        assert!(!names.contains(&"table".to_owned()), "{names:?}");
        let variables = extract_variables(&commands);
        assert!(
            variables.static_vars.is_empty(),
            "{:?}",
            variables.static_vars
        );
        assert!(!needs_multi_tmm(&commands, &variables));
        assert_eq!(object_refs(inert).pools, ["live"]);

        let live = "when HTTP_REQUEST { set static::hits 0; incr static::hits; table incr key; HTTP::respond 200 }";
        let commands = executable(live);
        let variables = extract_variables(&commands);
        assert_eq!(variables.static_vars, ["hits"]);
        assert!(needs_multi_tmm(&commands, &variables));
    }

    #[test]
    fn executable_inventory_includes_proc_bodies_and_excludes_invalid_regions() {
        let src = concat!(
            "pool top_level_stale\n",
            "proc helper {} { pool proc_pool; set x [class match value equals proc_dg] }\n",
            "when HTTP_REQUEST { call helper; switch -- x { x { pool event_pool } }; when CLIENT_DATA { pool nested_stale } }\n",
        );
        let commands = executable(src);
        let refs = object_refs(src);
        assert_eq!(refs.pools, ["event_pool", "proc_pool"], "{commands:?}");
        assert_eq!(refs.datagroups, ["proc_dg"]);
        assert!(!extract_irule_commands(&commands).contains(&"when".to_owned()));
    }

    #[test]
    fn generated_object_setup_follows_the_shared_event_closure() {
        let src = concat!(
            "proc helper {} { pool helper_pool }\n",
            "proc dormant {} { pool dormant_pool }\n",
            "when HTTP_REQUEST { call helper; dormant; pool event_pool }\n",
            "when CLIENT_DATA { pool other_event_pool }\n",
        );
        let refs = object_refs(src);
        assert_eq!(
            refs.pools,
            ["event_pool", "helper_pool", "other_event_pool"],
            "the aggregate owner includes each event root and only reached helpers"
        );
        assert!(
            !refs.pools.contains(&"dormant_pool".to_owned()),
            "a direct procedure-looking call cannot make dormant setup live"
        );
    }

    #[test]
    fn called_helper_static_write_preserves_hot_event_for_multi_tmm() {
        let generated = generate_irule_test(&json!({
            "source": concat!(
                "proc helper {} { set static::flag 1 }\n",
                "when HTTP_REQUEST { call helper }\n",
            ),
        }));
        assert_eq!(generated["multi_tmm_detected"], json!(true));
        assert!(
            generated["test_script"]
                .as_str()
                .is_some_and(|script| script.contains("fakeCMP distributes clients across TMMs")),
            "a hot event's called helper write needs the multi-TMM scaffold"
        );
    }

    #[test]
    fn generator_builds_one_request_closure_for_every_execution_sensitive_output() {
        let src = concat!(
            "proc ::rooted_helper {} { pool helper_pool; call chain_helper }\n",
            "proc chain_helper {} { set selected [class match [HTTP::uri] equals helper_dg]; call ::rooted_helper }\n",
            "proc dormant {} { pool dormant_pool; set static::dormant 1; table incr dormant }\n",
            "when HTTP_REQUEST { pool first_pool; set static::hits 0; call rooted_helper; dormant; apply {{} { pool lambda_pool; set static::lambda 1 }}; when CLIENT_DATA { pool nested_other_event_pool; set static::nested 1 } }\n",
            "when http_request { pool second_pool; incr static::hits }\n",
        );

        reset_request_closure_builds();
        let generated = generate_irule_test(&json!({"source": src}));

        assert_eq!(
            request_closure_builds(),
            1,
            "one request must construct exactly one event-rooted closure"
        );
        assert_eq!(generated["events"], json!(["HTTP_REQUEST"]));
        assert_eq!(
            generated["pools"],
            json!(["first_pool", "helper_pool", "second_pool"]),
            "both same-event roots and the rooted call cycle share one closure"
        );
        assert_eq!(generated["datagroups"], json!(["helper_dg"]));
        assert_eq!(generated["multi_tmm_detected"], json!(true));

        let commands = generated["commands_used"].as_array().unwrap();
        for name in ["HTTP::uri", "call", "class", "incr", "pool", "set"] {
            assert!(
                commands.iter().any(|value| value == name),
                "live closure command {name:?} missing: {commands:?}"
            );
        }

        let script = generated["test_script"].as_str().unwrap();
        assert!(script.contains("::orch::configure_static hits"), "{script}");
        for inert in ["dormant_pool", "lambda_pool", "nested_other_event_pool"] {
            assert!(
                !generated["pools"].to_string().contains(inert)
                    && !generated["datagroups"].to_string().contains(inert)
                    && !script.contains(&format!("::orch::add_pool {inert}")),
                "dormant/direct/lambda/nested-other-event pool leaked into setup: {inert:?}\n{script}"
            );
        }
        for inert in ["dormant", "lambda", "nested"] {
            assert!(
                !script.contains(&format!("::orch::configure_static {inert}")),
                "dormant/direct/lambda/nested-other-event static leaked into setup: {inert:?}\n{script}"
            );
        }
    }

    #[test]
    fn unknown_events_and_malformed_procs_are_inert_for_generated_inventory() {
        let src = concat!(
            "when BOGUS_EVENT { pool bogus; set static::bogus 1; table incr bogus }\n",
            "proc missing {}\n",
            "proc extra {} { pool malformed } trailing\n",
            "proc valid {} { pool valid_proc }\n",
            "when HTTP_REQUEST { call valid; pool valid_event }\n",
        );
        let generated = generate_irule_test(&json!({"source": src}));
        assert_eq!(generated["events"], json!(["HTTP_REQUEST"]));
        assert_eq!(generated["pools"], json!(["valid_event", "valid_proc"]));
        assert_eq!(generated["commands_used"], json!(["call", "pool"]));
        assert_eq!(generated["multi_tmm_detected"], json!(false));
        let script = generated["test_script"].as_str().unwrap();
        assert!(
            !script.contains("::orch::configure_static bogus"),
            "{script}"
        );
        assert!(!script.contains("::orch::add_pool bogus"), "{script}");
        assert!(!script.contains("::orch::add_pool malformed"), "{script}");
    }

    #[test]
    fn generated_event_inventory_uses_exact_shared_handler_grammar() {
        let src = concat!(
            "when HTTP_REQUEST { pool malformed } trailing\n",
            "when CLIENT_ACCEPTED pool\n",
            "when CLIENT_DATA timing on { pool timed }\n",
            "when SERVER_DATA priority 10 timing disable { pool ordered }\n",
            "when HTTP_RESPONSE timing on priority 10 { pool wrong_order }\n",
        );
        let generated = generate_irule_test(&json!({"source": src}));
        assert_eq!(generated["events"], json!(["CLIENT_DATA", "SERVER_DATA"]));
        assert_eq!(generated["pools"], json!(["ordered", "timed"]));
    }

    #[test]
    fn scaffold_deduplicates_events_and_sets_up_registry_classified_pools() {
        let src = concat!(
            "interp alias {} members {} active_members\n",
            "interp alias {} retry {} LB::reselect\n",
            "interp alias {} logger {} HSL::open\n",
            "when HTTP_REQUEST { set n [members /Common/active] }\n",
            "when HTTP_REQUEST { retry pool /Common/fallback }\n",
            "when CLIENT_ACCEPTED { set h [::logger -proto UDP -pool /Common/logging] }\n",
        );
        let generated = generate_irule_test(&json!({"source": src}));
        assert_eq!(
            generated["events"],
            json!(["CLIENT_ACCEPTED", "HTTP_REQUEST"])
        );
        assert_eq!(generated["pools"], json!(["/Common/active"]));
        let script = generated["test_script"].as_str().unwrap();
        assert!(
            script.contains("::orch::add_pool /Common/active "),
            "{script}"
        );
        assert!(
            !script.contains("::orch::add_pool /Common/fallback "),
            "{script}"
        );
        assert!(
            !script.contains("::orch::add_pool /Common/logging "),
            "{script}"
        );
    }
}
