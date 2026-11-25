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

use regex::Regex;

use super::*;

fn reset_request_closure_builds() {
    tcl_irules::reset_executable_closure_builds_for_tests();
}

fn request_closure_builds() -> usize {
    tcl_irules::executable_closure_builds_for_tests()
}

fn registry() -> &'static tcl_registry::CommandRegistry {
    crate::environment::store_for_dialect("f5-irules")
}

fn executable(src: &str) -> SourceInventory {
    SourceInventory::capture(src, registry())
}

fn request_syntax() -> (LexerConfig, ExprParseContext) {
    executable("when HTTP_REQUEST {pool p}")
        .context
        .expect("original source context")
        .expression_syntax()
}

fn object_refs(src: &str) -> ObjectRefs {
    let context = tcl_irules::OriginalIrulesSourceContext::capture(src, registry()).unwrap();
    extract_object_refs(src, &context)
}

fn if_cond(condition: &str) -> Value {
    json!({"kind": "if", "condition": condition})
}

#[test]
fn scaffold_ssl_profiles_follow_the_side_of_the_requested_events() {
    let registry = EventRegistry::build();
    let server_events = ["SERVERSSL_CLIENTHELLO_SEND".to_owned()];
    assert_eq!(
        infer_profiles(&registry, &server_events),
        ["TCP", "SERVERSSL"]
    );
    let both_events = [
        "CLIENTSSL_HANDSHAKE".to_owned(),
        "HTTP_REQUEST".to_owned(),
        "SERVERSSL_HANDSHAKE".to_owned(),
    ];
    assert_eq!(
        infer_profiles(&registry, &both_events),
        ["TCP", "CLIENTSSL", "SERVERSSL", "HTTP"]
    );
}

#[test]
fn generated_description_is_one_inert_tcl_word() {
    for description in [
        "$suspicious",
        "[HTTP::uri]",
        "a \"quoted\" description",
        r"path\\with\\slashes",
        "unbalanced { brace",
    ] {
        let rendered = render_test_case("case", description, &[]);
        assert!(
            rendered.starts_with(&format!(
                "::orch::test \"case\" {} -body {{\n",
                list_element(description)
            )),
            "description must use the shared Tcl word encoder: {rendered:?}"
        );
    }
}

/// The URI hint must recognise the pattern-bearing iRules word
/// operators `matches_glob`/`matches_regex` — the bare `matches` is
/// kept out of the alternation, so it never shadows them — and extract
/// the pattern instead of falling through to the "/" default.
#[test]
fn uri_hint_extracts_matches_glob_and_matches_regex() {
    let cond = if_cond(r#"[HTTP::uri] matches_glob "/api/*""#);
    let lines = build_request_setup("HTTP_REQUEST", &[&cond], Some(request_syntax()));
    assert!(
        lines.iter().any(|l| l.contains(r#"-uri "/api/x""#)),
        "{lines:?}"
    );

    let cond = if_cond(r#"[HTTP::uri] matches_regex "^/api/.*$""#);
    let lines = build_request_setup("HTTP_REQUEST", &[&cond], Some(request_syntax()));
    assert!(
        lines.iter().any(|l| l.contains(r#"-uri "/api/x""#)),
        "{lines:?}"
    );
}

/// Reusing the raw `matches_glob`/
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

/// `eq`/`starts_with` extraction must keep working when the
/// alternation is built from `BinOp` spellings instead of hand-typed
/// strings.
#[test]
fn uri_hint_still_extracts_eq_and_starts_with() {
    let cond = if_cond(r#"[HTTP::uri] eq "/login""#);
    let lines = build_request_setup("HTTP_REQUEST", &[&cond], Some(request_syntax()));
    assert!(
        lines.iter().any(|l| l.contains(r#"-uri "/login""#)),
        "{lines:?}"
    );

    let cond = if_cond(r#"[HTTP::uri] starts_with "/admin""#);
    let lines = build_request_setup("HTTP_REQUEST", &[&cond], Some(request_syntax()));
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
fn original_generated_static_setup_preserves_literal_repeated_prefixes() {
    // naming.consumer.original-irules-source-context
    // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
    // A source scaffold preserves labels; it proves no loaded variable,
    // event execution, storage identity or observed outcome.
    let generated = generate_irule_test(&json!({
        "source": concat!(
            "when HTTP_REQUEST {\n",
            " if {$static::static::hits} { pool live }\n",
            " if {$static::hits} {}\n",
            " set data {${static::static::inert}}\n",
            "}\n",
        ),
    }));
    let script = generated["test_script"].as_str().unwrap();
    assert!(
        script.contains("::orch::configure_static static::hits \"\""),
        "{script}"
    );
    assert!(
        script.contains("::orch::configure_static hits \"\""),
        "{script}"
    );
    assert!(
        !script.contains("::orch::configure_static inert "),
        "{script}"
    );
    assert!(
        !script.contains("::orch::configure_static static::inert "),
        "{script}"
    );
    assert_eq!(generated["analysis_kind"], "conditional-source");
    assert_eq!(generated["outcome_assertions_verified"], false);
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
    let context = tcl_irules::OriginalIrulesSourceContext::capture(inert, registry());
    assert!(!needs_multi_tmm(context.as_ref(), &commands, &variables));
    assert_eq!(object_refs(inert).pools, ["live"]);

    let live = "when HTTP_REQUEST { set static::hits 0; incr static::hits; table incr key; HTTP::respond 200 }";
    let commands = executable(live);
    let variables = extract_variables(&commands);
    assert_eq!(variables.static_vars, ["hits"]);
    let context = tcl_irules::OriginalIrulesSourceContext::capture(live, registry());
    assert!(needs_multi_tmm(context.as_ref(), &commands, &variables));
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
        "proc rooted_helper {} { pool helper_pool; call chain_helper }\n",
        "proc chain_helper {} { set selected [class match [HTTP::uri] equals helper_dg]; call rooted_helper }\n",
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
        "when HTTP_RESPONSE timing on priority 10 { pool reversed }\n",
        "when CLIENT_CLOSED timing on timing off { pool doubled }\n",
    );
    let generated = generate_irule_test(&json!({"source": src}));
    assert_eq!(
        generated["events"],
        json!(["CLIENT_DATA", "SERVER_DATA", "HTTP_RESPONSE"])
    );
    assert_eq!(generated["pools"], json!(["ordered", "reversed", "timed"]));
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
#[test]
fn original_irule_scaffolds_keep_source_candidates_without_predicted_outcomes() {
    // Implementation contract: naming.mcp.original-irule-test-source-obligations
    // docs/design/analysis/name-resolution-proofs/original-irule-test-source-obligations.md
    // Implementation contract: naming.consumer.original-irules-source-context
    // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
    let source = "when HTTP_REQUEST {pool /Common/candidate; HTTP::respond 503}";
    let generated = generate_irule_test(&json!({"source": source}));
    assert_eq!(generated["analysis_kind"], "conditional-source");
    assert_eq!(generated["outcome_assertions_verified"], false);
    let script = generated["test_script"].as_str().unwrap();
    assert!(script.contains("Outcome obligation"));
    assert!(
        !script
            .lines()
            .any(|line| line.trim_start().starts_with("::orch::assert_that "))
    );
    assert!(
        generated["pools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "/Common/candidate")
    );
    let custom =
        generate_irule_test(&json!({"source":"proc when args {}; when HTTP_REQUEST {pool fake}"}));
    assert_eq!(
        custom["events"].as_array().unwrap().as_slice(),
        [] as [serde_json::Value; 0]
    );
    assert_eq!(
        custom["pools"].as_array().unwrap().as_slice(),
        [] as [serde_json::Value; 0]
    );
    assert_eq!(
        build_assertion("pool", &["target\nset forged 1".to_owned()])[1],
        "# Source candidate: pool target\\nset forged 1"
    );
}
#[test]
fn cfg_test_generation_uses_retained_source_kind_instead_of_event_labels() {
    let objects = ObjectRefs {
        pools: Vec::new(),
        datagroups: Vec::new(),
    };
    let variables = Variables {
        static_vars: Vec::new(),
    };
    let paths = [
        json!({"source_kind":"event", "event":"proc:report-data", "conditions":[], "action":{"command":"pool", "args":["p"]}}),
        json!({"source_kind":"procedure", "event":"HTTP_REQUEST", "conditions":[], "action":{"command":"pool", "args":["q"]}}),
        json!({"event":"HTTP_RESPONSE", "conditions":[], "action":{"command":"pool", "args":["r"]}}),
    ];
    let context = ScriptContext {
        basename: "owned-kind",
        source: "",
        ordered_events: &[],
        profiles: &[],
        commands_used: &[],
        objects: &objects,
        variables: &variables,
        cfg_paths: &paths,
        // Event setup is written only under a captured source grammar.
        request_syntax: Some(request_syntax()),
        multi_tmm: false,
    };
    let blocks = build_all_test_blocks("owned-kind", &context);
    assert_eq!(
        blocks
            .iter()
            .filter(|block| block.starts_with("::orch::test "))
            .count(),
        1
    );
    assert!(
        blocks
            .iter()
            .any(|block| block.contains("proc:report-data"))
    );
    assert!(
        blocks
            .iter()
            .all(|block| !block.contains("HTTP_REQUEST") && !block.contains("HTTP_RESPONSE"))
    );
}

#[test]
fn static_namespace_hints_preserve_actual_literal_variable_roles() {
    for (name, expected) in [
        ("static::flag", Some("flag")),
        ("static::::flag", Some("::flag")),
        ("static::", Some("")),
        ("::static::rooted", None),
        ("static_other::flag", None),
    ] {
        let source = format!("when HTTP_REQUEST {{set {name} 1}}");
        let inventory = executable(&source);
        assert!(inventory.context.is_some(), "{source}");
        let variables = extract_variables(&inventory);
        assert_eq!(
            variables.static_vars,
            expected.into_iter().map(str::to_owned).collect::<Vec<_>>(),
            "{source}"
        );
        assert_eq!(
            needs_multi_tmm(inventory.context.as_ref(), &inventory.commands, &variables),
            expected.is_some(),
            "{source}"
        );
    }
    let inventory = executable("when HTTP_REQUEST {set data {static::inert}}");
    assert_eq!(
        extract_variables(&inventory).static_vars,
        [] as [std::string::String; 0]
    );
}
