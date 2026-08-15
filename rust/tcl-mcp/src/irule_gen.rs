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
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};
use tcl_registry::events::EventRegistry;

/// The prose emitted for `multi_tmm_hint` when multi-TMM patterns are detected.
const MULTI_TMM_HINT: &str = "This iRule uses patterns that behave differently across TMMs \
     (static:: writes in hot events, counters, or shared table state). \
     The generated test includes a multi-TMM scenario using fakeCMP \
     distribution.  Use ::orch::fakecmp_suggest_sources to plan which \
     client addresses hit which TMMs.";

/// Top-level bareword commands `_extract_irule_commands` probes for.
const TOPLEVEL_CMDS: &[&str] = &[
    "pool", "node", "snat", "snatpool", "reject", "drop", "discard", "class", "table", "persist",
    "event", "log", "after", "virtual",
];

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

static IRULE_CMD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([A-Z][A-Z0-9]*::[a-z_][a-z0-9_]*)\b").expect("valid regex"));
static STATIC_VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:set\s+|[\$])static::(\w+)").expect("valid regex"));
static SET_INCR_STATIC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:set|incr)\s+static::").expect("valid regex"));
static COUNTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bincr\s+(?:static::\w+|\$?\w*count\w*)").expect("valid regex"));
static TABLE_STATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\btable\s+(?:incr|set|add)").expect("valid regex"));

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
    let event_names: Vec<String> = when_blocks
        .iter()
        .map(|block| block.event.clone())
        .collect();
    let ordered_events = EventRegistry::build().order_events(&event_names);
    let profiles = infer_profiles(&ordered_events);
    let commands_used = extract_irule_commands(source, &when_blocks);
    let objects = extract_object_refs(source, &when_blocks);
    let variables = extract_variables(source, &when_blocks);

    let cfg_paths = crate::irule_test::cfg_paths_json(source);
    let multi_tmm = needs_multi_tmm(source, &when_blocks, &commands_used, &variables);

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

/// Extract the iRule command names used in `source` (`_extract_irule_commands`):
/// `NS::cmd` tokens plus any bareword top-level commands, sorted + deduped.
fn extract_irule_commands(source: &str, blocks: &[tcl_irules::WhenBlock]) -> Vec<String> {
    let mut commands: BTreeSet<String> = BTreeSet::new();
    for body in handler_bodies(source, blocks) {
        for cap in IRULE_CMD_RE.captures_iter(body) {
            commands.insert(cap[1].to_owned());
        }
        for &cmd in TOPLEVEL_CMDS {
            if word_present(body, cmd) {
                commands.insert(cmd.to_owned());
            }
        }
    }
    commands.into_iter().collect()
}

/// True when `word` appears in `text` bounded by non-word characters on both
/// sides (equivalent to `re.search(r"\bword\b", text)` for `\w` words).
fn word_present(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let wlen = word.len();
    let mut start = 0;
    while let Some(pos) = text[start..].find(word) {
        let idx = start + pos;
        let before_ok = idx == 0 || !is_word_byte(bytes[idx - 1]);
        let after = idx + wlen;
        let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        start = idx + 1;
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Strip Tcl syntax characters from a reference (`_clean_ref`).
fn clean_ref(raw: &str) -> &str {
    raw.trim_matches(|c| matches!(c, '{' | '}' | '[' | ']' | '"' | '\''))
}

/// Extract pool / data-group / node / virtual references (`_extract_object_refs`).
fn extract_object_refs(source: &str, blocks: &[tcl_irules::WhenBlock]) -> ObjectRefs {
    let mut pools: BTreeSet<String> = BTreeSet::new();
    let mut datagroups: BTreeSet<String> = BTreeSet::new();

    for body in handler_bodies(source, blocks) {
        for command in tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            body,
            0,
            tcl_lexer::LexerConfig::for_file_dialect("f5-irules"),
        ) {
            if command.name() == "pool" {
                let Some(raw) = command.args().first() else {
                    continue;
                };
                let name = clean_ref(raw);
                if !name.is_empty() && !name.starts_with('$') && name != "member" {
                    pools.insert(name.to_owned());
                }
            } else if command.name() == "class" {
                let args = command.args();
                if matches!(args.first().map(String::as_str), Some("match" | "lookup"))
                    && let Some(raw) = args.last()
                {
                    let name = clean_ref(raw);
                    if !name.is_empty() && !name.starts_with('$') {
                        datagroups.insert(name.to_owned());
                    }
                }
            }
        }
    }

    ObjectRefs {
        pools: pools.into_iter().collect(),
        datagroups: datagroups.into_iter().collect(),
    }
}

/// Extract `static::` variable names (`_extract_variables`), sorted + deduped.
fn extract_variables(source: &str, blocks: &[tcl_irules::WhenBlock]) -> Variables {
    let mut static_vars: BTreeSet<String> = BTreeSet::new();
    for body in handler_bodies(source, blocks) {
        for cap in STATIC_VAR_RE.captures_iter(body) {
            static_vars.insert(cap[1].to_owned());
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
    source: &str,
    blocks: &[tcl_irules::WhenBlock],
    commands_used: &[String],
    variables: &Variables,
) -> bool {
    let has_static_vars = !variables.static_vars.is_empty();
    let uses_table = commands_used.iter().any(|c| c == "table");

    // Pattern 1: static:: written in a hot event body.
    let static_in_hot = HOT_EVENTS.iter().any(|event| {
        event_body(source, blocks, event).is_some_and(|body| SET_INCR_STATIC_RE.is_match(body))
    });

    // Pattern 2: rate-limiting / counter patterns.
    let has_counter = handler_bodies(source, blocks).any(|body| COUNTER_RE.is_match(body));

    // Pattern 3: table used for shared state.
    let uses_shared_table =
        uses_table && handler_bodies(source, blocks).any(|body| TABLE_STATE_RE.is_match(body));

    static_in_hot || (has_counter && has_static_vars) || uses_shared_table
}

/// Return the segmented Tcl bodies of every live event handler.
fn handler_bodies<'a>(
    source: &'a str,
    blocks: &'a [tcl_irules::WhenBlock],
) -> impl Iterator<Item = &'a str> {
    blocks
        .iter()
        .filter_map(|block| source.get(block.body_span.as_range()))
}

/// The segmented body of the first live handler for `event`.
fn event_body<'a>(
    source: &'a str,
    blocks: &'a [tcl_irules::WhenBlock],
    event: &str,
) -> Option<&'a str> {
    blocks
        .iter()
        .find(|block| block.event == event)
        .and_then(|block| source.get(block.body_span.as_range()))
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
        let blocks = tcl_irules::when_blocks(src);
        assert_eq!(
            extract_object_refs(src, &blocks).pools,
            vec!["live".to_owned()]
        );
        assert!(!extract_irule_commands(src, &blocks).contains(&"stale".to_owned()));
    }

    #[test]
    fn extractors_ignore_commented_pool_inside_live_handler() {
        let src = "when HTTP_REQUEST {\n # pool stale\n pool live\n}\n";
        let blocks = tcl_irules::when_blocks(src);
        assert_eq!(
            extract_object_refs(src, &blocks).pools,
            vec!["live".to_owned()]
        );
    }

    #[test]
    fn object_refs_ignore_inline_comments_and_pool_like_data() {
        let src = "when HTTP_REQUEST {\n set x 1; # pool stale\n set quoted \"pool quoted\"\n set braced {pool braced}\n pool live\n}\n";
        let blocks = tcl_irules::when_blocks(src);
        assert_eq!(
            extract_object_refs(src, &blocks).pools,
            vec!["live".to_owned()]
        );
    }
}
