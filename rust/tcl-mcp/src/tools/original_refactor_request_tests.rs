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

use super::*;

fn authored_datagroup_registry() -> std::sync::Arc<CommandRegistry> {
    const OUTPUTS: &[tcl_registry::SubCommand] = &[
        tcl_registry::SubCommand {
            name: "match",
            arity: tcl_registry::Arity::exact(3),
            ..tcl_registry::SubCommand::DEFAULT
        },
        tcl_registry::SubCommand {
            name: "lookup",
            arity: tcl_registry::Arity::exact(2),
            ..tcl_registry::SubCommand::DEFAULT
        },
    ];
    let mut registry = CommandRegistry::build_default();
    // An authored source output surface is independent of loaded handlers.
    registry.insert(tcl_registry::CommandSpec {
        name: "class",
        surface: Some(tcl_dialect::model::SpecSurface::TCL90_PLUS),
        subcommands: OUTPUTS,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    std::sync::Arc::new(registry)
}

fn datagroup_analysis(
    source: &str,
    environment: &str,
    registry: std::sync::Arc<CommandRegistry>,
) -> AnalysisResult {
    let profile = DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(
            tcl_registry::model::ingress::resolve_environment(environment)
                .default_context_registry()
                .with_command_store(registry),
        ),
        LexerConfig::for_file_grammar(profile.grammar),
    );
    let result = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(result.allows_lexical_declaration_advice());
    result
}

#[test]
fn original_datagroup_tool_advice_keeps_actual_output_availability_and_selected_horizons() {
    // naming.consumer.original-mcp-refactor-context
    // docs/design/analysis/name-resolution-proofs/original-mcp-refactor-context.md
    let source = "if {$x eq {one}} {set answer FIRST} elseif {$x eq {two}} {set answer SECOND}";
    let registry = authored_datagroup_registry();
    let analysis = datagroup_analysis(source, "tcl9.0", registry.clone());
    let index = LineIndex::new(source);
    let action = datagroup_advice(source, 0, "answers", &analysis, &index)
        .expect("actual authored output and original conditional arm source");
    assert_eq!(
        action.try_apply(source).unwrap(),
        "set answer [class lookup $x answers]"
    );
    assert_eq!(action.data_group.as_ref().unwrap().records.len(), 2);
    let older = datagroup_analysis(source, "tcl8.4", registry.clone());
    assert!(datagroup_advice(source, 0, "answers", &older, &index).is_none());
    for prefix in ["proc set args {}; ", "proc class args {}; "] {
        let replaced = format!("{prefix}{source}");
        let owner = datagroup_analysis(&replaced, "tcl9.0", registry.clone());
        assert!(
            datagroup_advice(
                &replaced,
                u32::try_from(prefix.len()).unwrap(),
                "answers",
                &owner,
                &LineIndex::new(&replaced)
            )
            .is_none()
        );
    }
    let alias = "interp alias {} choose {} if
choose {$x eq {one}} {puts SAME} elseif {$x eq {two}} {puts SAME}";
    let alias_owner = datagroup_analysis(alias, "tcl9.0", registry.clone());
    assert!(
        datagroup_advice(
            alias,
            u32::try_from(alias.rfind("choose").unwrap()).unwrap(),
            "answers",
            &alias_owner,
            &LineIndex::new(alias)
        )
        .is_some()
    );
    let captured = "interp alias {} choose {} if {$x eq {one}}
choose {puts SAME} elseif {$x eq {two}} {puts SAME}";
    let owner = datagroup_analysis(captured, "tcl9.0", registry);
    assert!(
        datagroup_advice(
            captured,
            u32::try_from(captured.rfind("choose").unwrap()).unwrap(),
            "answers",
            &owner,
            &LineIndex::new(captured)
        )
        .is_none()
    );
}

#[test]
fn original_datagroup_tool_advice_refuses_missing_stale_foreign_and_unavailable_owners() {
    // naming.consumer.original-mcp-refactor-context
    // docs/design/analysis/name-resolution-proofs/original-mcp-refactor-context.md
    let source = "if {$x eq {one}} {set answer FIRST} elseif {$x eq {two}} {set answer SECOND}";
    let registry = authored_datagroup_registry();
    let analysis = datagroup_analysis(source, "tcl9.0", registry.clone());
    let index = LineIndex::new(source);
    assert!(datagroup_advice(source, 0, "answers", &analysis, &index).is_some());
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let mut stale = analysis.clone();
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    let mut foreign = analysis.clone();
    foreign.resolved_input = datagroup_analysis(source, "tcl8.4", registry).resolved_input;
    let mut unavailable = analysis.clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl9.0".into(),
        overlay: u64::MAX - 405,
    });
    for refused in [&missing, &stale, &foreign, &unavailable] {
        assert!(datagroup_advice(source, 0, "answers", refused, &index).is_none());
    }
    assert!(datagroup_advice("# changed", 0, "answers", &analysis, &index).is_none());
    assert!(datagroup_advice(source, 0, "answers", &analysis, &index).is_some());
}

// Implementation contract: naming.consumer.original-mcp-refactor-context
// docs/design/analysis/name-resolution-proofs/original-mcp-refactor-context.md
#[test]
fn original_refactor_requests_do_not_replace_native_or_vendor_policy_with_irules() {
    let source = "switch -- $value { a {set result 1} b {set result 2} }";
    for dialect in ["tcl8.6", "tcl9.0", "jimtcl", "f5-irules", "f5-tmsh"] {
        let args = json!({"source":source,"dialect":dialect,"line":0,"character":0,
                "start_line":0,"start_character":0,"end_line":0,"end_character":0});
        assert!(
            extract_datagroup(&args)["refused"]["reason"].is_string(),
            "{dialect}"
        );
        let candidates = refactor(&args);
        assert!(
            candidates["available"]
                .as_array()
                .unwrap()
                .iter()
                .all(|candidate| candidate["tool"] != "extract_datagroup"),
            "{dialect}"
        );
    }
}

#[test]
fn original_code_action_responses_retain_disabled_rewrite_obligations() {
    // Implementation contract: naming.consumer.original-mcp-refactor-context
    // docs/design/analysis/name-resolution-proofs/original-mcp-refactor-context.md
    // A read trace observes how often the subject is read, so the chain
    // keeps its control-flow obligation (evidence/tcl/lsp-if-to-switch-read-count.md).
    let source = "set x a\ntrace add variable x read {incr ::reads;#}\nif {$x eq \"a\"} {puts YES} elseif {$x eq \"b\"} {puts NO}";
    let response = code_actions(&json!({
        "source": source,
        "dialect": "tcl8.6",
        "start_line": 2,
        "start_character": 0,
        "end_line": 2,
        "end_character": 0,
    }));
    let action = response["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Convert if chain to switch")
        .expect("selected Native structure retains its rewrite obligation");
    assert!(action["edits"].is_null());
    assert!(
        action["refused"]["reason"]
            .as_str()
            .unwrap()
            .starts_with("missing-control-flow-equivalence:")
    );
}

// Implementation contract: naming.consumer.original-mcp-refactor-context
// docs/design/analysis/name-resolution-proofs/original-mcp-refactor-context.md
#[test]
fn original_refactor_requests_use_the_selected_analysis_for_shadowed_heads() {
    let source = "proc expr {value} {return $value}\nexpr $x + 1\n";
    let args = json!({"source":source,"dialect":"tcl8.6","line":1,"character":2});
    let result = brace_expr(&args);
    assert!(result["refused"]["reason"].is_string(), "{result}");
    assert_eq!(arg_str(&args, "dialect"), "tcl8.6");
}
