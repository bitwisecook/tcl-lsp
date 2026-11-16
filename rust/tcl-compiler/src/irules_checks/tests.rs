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

use tcl_dialect::model::{Family, SurfaceQuery};

use super::*;

fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(tcl_dialect::DialectProfile::irules())
}

fn warnings_for_irules(source: &str) -> Vec<IrulesCheckWarning> {
    let cu = CompilationUnit::build_for(source, &registry(), false);
    find_unnormalised_getter_warnings(
        &cu,
        &registry(),
        Some(SurfaceQuery::any_release(Family::F5Irules)),
    )
}

#[test]
fn irule3102_warns_on_bare_http_uri_getter() {
    let w = warnings_for_irules("set u [HTTP::uri]");
    assert_eq!(w.len(), 1, "expected one IRULE3102, got {w:?}");
    assert_eq!(w[0].code, DiagCode::Irule3102);
    assert!(w[0].message.contains("HTTP::uri -normalized"));
}

#[test]
fn irule3102_clean_when_normalized_flag_present() {
    let w = warnings_for_irules("set u [HTTP::uri -normalized]");
    assert_eq!(w.len(), 0, "expected no IRULE3102, got {w:?}");
}

#[test]
fn irule3102_setter_form_not_flagged() {
    // `HTTP::path /x` — first arg `/x` is non-flag → setter form → no warning.
    let w = warnings_for_irules("HTTP::path /x");
    assert_eq!(w.len(), 0, "expected no IRULE3102 on setter, got {w:?}");
}

#[test]
fn irule3102_warns_on_getter_nested_in_sink_argument() {
    // `HTTP::respond 200 content [HTTP::uri]` — the unnormalised
    // `[HTTP::uri]` getter is a command-substitution *argument* of the
    // HTTP sink, not a top-level call/assignment, but must still fire.
    let w =
        warnings_for_irules("when HTTP_REQUEST {\n    HTTP::respond 200 content [HTTP::uri]\n}\n");
    assert_eq!(w.len(), 1, "expected one IRULE3102, got {w:?}");
    assert_eq!(w[0].code, DiagCode::Irule3102);
    assert!(w[0].message.contains("HTTP::uri -normalized"));
}

#[test]
fn irule3102_silent_for_constant_sink_argument() {
    // A constant fed into the same sink stays silent.
    let w = warnings_for_irules(
        "when HTTP_REQUEST {\n    HTTP::respond 200 content \"static body\"\n}\n",
    );
    assert_eq!(w.len(), 0, "expected no IRULE3102, got {w:?}");
}

#[test]
fn irule3102_silent_for_normalized_getter_in_sink_argument() {
    // The nested getter with `-normalized` is canonicalised → silent.
    let w = warnings_for_irules(
        "when HTTP_REQUEST {\n    HTTP::respond 200 content [HTTP::uri -normalized]\n}\n",
    );
    assert_eq!(w.len(), 0, "expected no IRULE3102, got {w:?}");
}

#[test]
fn irule3102_non_irules_dialect_returns_empty() {
    let cu = CompilationUnit::build_for("set u [HTTP::uri]", &registry(), false);
    assert_eq!(
        find_unnormalised_getter_warnings(&cu, &registry(), None),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    assert_eq!(
        find_unnormalised_getter_warnings(
            &cu,
            &registry(),
            Some(SurfaceQuery::any_release(Family::Tcl))
        )
        .len(),
        0
    );
}

#[test]
fn irule3102_also_fires_inside_assign_value_cmd_sub() {
    // Plain `set u [HTTP::query]` — command-sub form, must still fire.
    let w = warnings_for_irules("set u [HTTP::query]");
    assert_eq!(w.len(), 1, "expected IRULE3102 for HTTP::query, got {w:?}");
    assert!(w[0].message.contains("HTTP::query"));
}

#[test]
fn irule3102_fires_on_http_path_without_flag() {
    let w = warnings_for_irules("set u [HTTP::path]");
    assert_eq!(w.len(), 1);
    assert!(w[0].message.contains("HTTP::path"));
}

#[test]
fn is_getter_form_recognises_flags_only() {
    assert!(is_getter_form(&[]));
    assert!(is_getter_form(&["-normalized".to_owned()]));
    assert!(!is_getter_form(&["/foo".to_owned()]));
    assert!(!is_getter_form(&[
        "foo".to_owned(),
        "-normalized".to_owned()
    ]));
}

/// IRULE3102 must derive its command set from the
/// registry's `OptionSpec` table (not a hardcoded list). When
/// the registry-side option is present, the check fires; when
/// the registered command does not declare `-normalized`, no
/// diagnostic is produced.
#[test]
fn arch3_normalised_option_is_registry_driven() {
    let registry = registry();
    // Registry-side: HTTP::uri / HTTP::path / HTTP::query carry
    // the `-normalized` option in their command spec.
    for name in ["HTTP::uri", "HTTP::path", "HTTP::query"] {
        assert!(
            supports_normalized_flag(&registry, name),
            "{name} should declare -normalized in its registry OptionSpec",
        );
    }
    // A registered iRules command without `-normalized` is not a
    // candidate (e.g. `HTTP::header`, which has no
    // `-normalized` option).
    assert!(
        !supports_normalized_flag(&registry, "HTTP::header"),
        "HTTP::header has no -normalized option in the registry",
    );

    // End-to-end proof: HTTP::uri without -normalized fires
    // IRULE3102; HTTP::header (no -normalized in registry) does not.
    let with_uri = warnings_for_irules("set u [HTTP::uri]");
    assert!(
        with_uri.iter().any(|w| w.code == DiagCode::Irule3102),
        "expected IRULE3102 on HTTP::uri, got {with_uri:?}",
    );
    let with_header = warnings_for_irules("set u [HTTP::header Content-Type]");
    assert_eq!(
        with_header.len(),
        0,
        "no IRULE3102 expected on HTTP::header (no -normalized), got {with_header:?}",
    );
}

// IRULE5002 / IRULE5004

fn drop_warnings(source: &str) -> Vec<IrulesCheckWarning> {
    source_flow_warnings(source)
        .into_iter()
        .filter(|warning| matches!(warning.code, DiagCode::Irule5002 | DiagCode::Irule5004))
        .collect()
}

#[test]
fn irule5002_drop_without_return_fires() {
    let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule5002),
        "expected IRULE5002, got {ws:?}",
    );
}

#[test]
fn irule5002_drop_followed_by_return_clean() {
    let ws = drop_warnings("when CLIENT_ACCEPTED { drop; return }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule5002),
        "no IRULE5002 expected when `return` guards the drop, got {ws:?}",
    );
}

#[test]
fn irule5002_drop_followed_by_event_disable_all_clean() {
    let ws = drop_warnings("when CLIENT_ACCEPTED { drop; event disable all }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule5002),
        "no IRULE5002 expected when `event disable all` guards the drop, got {ws:?}",
    );
}

#[test]
fn irule5002_reject_also_fires() {
    let ws = drop_warnings("when CLIENT_ACCEPTED { reject }");
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule5002),
        "expected IRULE5002 on `reject`, got {ws:?}",
    );
}

#[test]
fn irule5002_only_in_irules_dialect() {
    let cu = CompilationUnit::build_for("when CLIENT_ACCEPTED { drop }", &registry(), false);
    let none_dialect = find_unguarded_drop_warnings(&cu, None);
    assert_eq!(none_dialect.len(), 0, "got {none_dialect:?}");
    let tcl_dialect =
        find_unguarded_drop_warnings(&cu, Some(SurfaceQuery::any_release(Family::Tcl)));
    assert_eq!(tcl_dialect.len(), 0, "got {tcl_dialect:?}");
}

#[test]
fn irule5004_dns_return_without_return_fires() {
    let ws = drop_warnings("when DNS_REQUEST { DNS::return }");
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule5004),
        "expected IRULE5004, got {ws:?}",
    );
}

#[test]
fn irule5004_dns_return_followed_by_return_clean() {
    let ws = drop_warnings("when DNS_REQUEST { DNS::return; return }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule5004),
        "no IRULE5004 expected when `return` follows `DNS::return`, got {ws:?}",
    );
}

#[test]
fn no_drop_warnings_for_clean_when_body() {
    let ws = drop_warnings("when CLIENT_ACCEPTED { log local0. \"connection open\" }");
    assert_eq!(ws.len(), 0, "got {ws:?}");
}

fn drop_codes(source: &str) -> Vec<String> {
    let mut c: Vec<String> = drop_warnings(source)
        .iter()
        .map(|w| w.code.to_string())
        .collect();
    c.sort();
    c
}

/// A handler whose condition and arms run only the dialect's own commands
/// keeps its command world: on the closed load-time surface the arms'
/// compiled `HTTP::respond` dispatches to its catalogued handler, and the
/// condition's error edge writes the C core's error storage, so no command
/// in the handler is taken to reach unseen code.
#[test]
fn a_handler_branching_on_dialect_commands_reaches_no_unseen_code() {
    let (cu, _context) = source_unit(
        "when HTTP_REQUEST {\n    set x 1\n    if {$x == 1} {HTTP::respond 200 content one} else {HTTP::respond 200 content two}\n}\n",
    );
    let handler = cu
        .cfg_module
        .procedures
        .values()
        .find(|function| function.name == "::when::HTTP_REQUEST")
        .expect("the HTTP_REQUEST handler has a flow graph");
    let markers: Vec<_> = handler
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|stmt| match stmt {
            crate::ir::Statement::Call {
                tokens: Some(tokens),
                ..
            }
            | crate::ir::Statement::Barrier {
                tokens: Some(tokens),
                ..
            } => tokens.synthetic,
            _ => None,
        })
        .collect();
    assert!(
        markers.is_empty(),
        "dialect commands must not mark unseen code: {markers:?}"
    );
}

#[test]
fn irule5002_fires_when_only_one_branch_guarded() {
    // The `drop` path has no following `event disable all` / `return`, so
    // it is unguarded — the path-sensitive walk keeps that path's state and
    // flags it even though the other branch returns.
    assert!(
        drop_codes("when CLIENT_ACCEPTED { if {$x} { drop } else { return } }")
            .contains(&"IRULE5002".to_string())
    );
}

#[test]
fn irule5002_quiet_when_every_branch_guarded() {
    // Both branches guard their drop with `return`, so no surviving path
    // is left unguarded.
    assert_eq!(
        drop_codes("when CLIENT_ACCEPTED { if {$x} { drop; return } else { reject; return } }")
            .len(),
        0
    );
}

/// A drop or DNS answer in a block the solver proves unreachable leaves
/// nothing unguarded; a live arm's still does.
#[test]
fn irule5002_ignores_a_drop_in_a_dead_arm() {
    for source in [
        "when CLIENT_ACCEPTED { if {0} { drop } }",
        "when CLIENT_ACCEPTED { set flag 0; if {$flag} { reject } }",
        "when CLIENT_ACCEPTED { while {0} { discard } }",
    ] {
        assert!(
            drop_codes(source).is_empty(),
            "{source}: {:?}",
            drop_codes(source)
        );
    }
    for source in [
        "when CLIENT_ACCEPTED { if {1} { drop } }",
        "when CLIENT_ACCEPTED { if {$x} { drop } }",
    ] {
        assert!(
            drop_codes(source).contains(&"IRULE5002".to_owned()),
            "{source}: {:?}",
            drop_codes(source),
        );
    }
}

#[test]
fn irule5004_ignores_a_dns_return_in_a_dead_arm() {
    assert_eq!(
        drop_codes("when DNS_REQUEST { if {0} { DNS::return \"1.2.3.4\" } }"),
        [] as [std::string::String; 0]
    );
    assert!(
        drop_codes("when DNS_REQUEST { if {$x} { DNS::return \"1.2.3.4\" } }")
            .contains(&"IRULE5004".to_owned())
    );
}

#[test]
fn irule5002_drop_points_at_the_drop_command() {
    // The diagnostic span must land on the `drop` word itself (offset 23
    // in this source), not past it — the structured-IR walk carries the
    // command's absolute span.
    let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
    let w = ws
        .iter()
        .find(|w| w.code == DiagCode::Irule5002)
        .expect("IRULE5002");
    assert_eq!(w.span.start(), 23, "span should point at `drop`");
}

#[test]
fn irule5002_carries_insertion_fix() {
    // The quick-fix inserts `event disable all` + `return` *after* the drop
    // (a zero-width edit anchored at the drop command's end), independent of
    // the diagnostic span.
    let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
    let w = ws
        .iter()
        .find(|w| w.code == DiagCode::Irule5002)
        .expect("IRULE5002");
    assert_eq!(w.fixes.len(), 1, "expected one fix, got {:?}", w.fixes);
    let fix = &w.fixes[0];
    assert_eq!(fix.new_text, "\n    event disable all\n    return");
    assert_eq!(fix.description, "Add 'event disable all' + 'return'");
    // Zero-width insertion at the drop command's end.
    assert_eq!(fix.span.start(), w.span.end());
    assert_eq!(fix.span.start(), fix.span.end());
}

#[test]
fn irule5004_carries_insertion_fix() {
    let ws = drop_warnings("when DNS_REQUEST { DNS::return }");
    let w = ws
        .iter()
        .find(|w| w.code == DiagCode::Irule5004)
        .expect("IRULE5004");
    assert_eq!(w.fixes.len(), 1);
    let fix = &w.fixes[0];
    assert_eq!(fix.new_text, "\n    return");
    assert_eq!(fix.description, "Add 'return' after DNS::return");
    assert_eq!(fix.span.start(), fix.span.end());
}

#[test]
fn irule5004_fires_through_catch_body() {
    // A `DNS::return` inside `catch { … }` still leaves the path needing a
    // following `return` (the linear scan missed the catch body).
    assert!(
        drop_codes("when DNS_REQUEST { catch { DNS::return } }").contains(&"IRULE5004".to_string())
    );
}

#[test]
fn irule5002_drop_and_return_inside_catch_still_warns() {
    // `catch` swallows the `return`, so it does NOT stop iRule processing —
    // the unguarded `drop` still leaks past the catch and must be flagged.
    let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { drop; return }\n}");
    let hits: Vec<_> = ws
        .iter()
        .filter(|w| w.code == DiagCode::Irule5002)
        .collect();
    assert_eq!(hits.len(), 1, "expected one IRULE5002, got {ws:?}");
    assert!(
        hits[0].message.contains("drop"),
        "expected the unguarded drop to be flagged, got {:?}",
        hits[0].message,
    );
}

#[test]
fn irule5002_catch_body_without_return_still_warns_drop() {
    // A catch whose body does not return continues normally and the
    // unguarded drop still leaks out.
    let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { drop }\n}");
    assert_eq!(
        ws.iter().filter(|w| w.code == DiagCode::Irule5002).count(),
        1,
        "expected one IRULE5002, got {ws:?}",
    );
}

#[test]
fn irule5002_catch_without_drop_no_warning() {
    // A benign catch body must not spuriously warn.
    let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { set x 1; return }\n}");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule5002),
        "no IRULE5002 expected for a benign catch, got {ws:?}",
    );
}

#[test]
fn irule5004_dns_return_and_return_inside_catch_still_warns() {
    // `catch` swallows the `return` so it does not stop processing — the
    // unguarded `DNS::return` still leaks out.
    let ws = drop_warnings("when DNS_REQUEST {\n    catch { DNS::return; return }\n}");
    let hits: Vec<_> = ws
        .iter()
        .filter(|w| w.code == DiagCode::Irule5004)
        .collect();
    assert_eq!(hits.len(), 1, "expected one IRULE5004, got {ws:?}");
    assert!(
        hits[0].message.contains("DNS::return"),
        "expected the unguarded DNS::return to be flagged, got {:?}",
        hits[0].message,
    );
}

// IRULE1005 / 1006 / 1007 / 1008

fn source_unit(
    source: &str,
) -> (
    CompilationUnit,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
) {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let context = environment.default_context_registry();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let input = crate::analyser::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        context.clone(),
        config,
    );
    let cu = CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(environment.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    (cu, context)
}

fn source_flow_warnings(source: &str) -> Vec<IrulesCheckWarning> {
    let (cu, context) = source_unit(source);
    find_conditional_irule_source_flow_advice(
        &cu,
        context.commands(),
        Some(SurfaceQuery::any_release(Family::F5Irules)),
    )
    .map_or_else(Vec::new, |advice| {
        advice
            .findings
            .iter()
            .map(|finding| finding.warning.clone())
            .collect()
    })
}

fn flow_warnings(source: &str) -> Vec<IrulesCheckWarning> {
    let (cu, context) = source_unit(source);
    find_conditional_collection_source_pairing_advice(
        &cu,
        context.commands(),
        Some(SurfaceQuery::any_release(Family::F5Irules)),
    )
    .map_or_else(Vec::new, |advice| {
        advice
            .pairings
            .iter()
            .map(|pairing| pairing.warning.clone())
            .collect()
    })
}

#[test]
fn irule1005_http_data_event_without_collect_fires() {
    // HTTP_REQUEST_DATA has no transport alternative: HTTP::collect is
    // required for it to fire.
    let ws = flow_warnings("when HTTP_REQUEST_DATA { log local0. \"data\" }");
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule1005),
        "expected IRULE1005, got {ws:?}",
    );
}

#[test]
fn irule1005_retains_the_original_event_operand_instead_of_a_body_anchor() {
    // naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    // Source geometry, with no Native event/TMM activation claim.
    for source in [
        "set marker 😀\nwhen {HTTP_REQUEST_DATA} {set body value}\n",
        "when HTTP_REQUEST_DATA priority 123 { }\n",
    ] {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let diagnostics = find_collect_flow_warnings(
            &cu,
            &reg,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        );
        let diagnostic = diagnostics
            .iter()
            .find(|row| row.code == DiagCode::Irule1005)
            .expect("source event requiring collection");
        let body = cu.ir_module.irules_event_bodies.values().next().unwrap();
        let word = body
            .event_word(&cu.ir_module.source, cu.ir_module.lexer_config, &reg)
            .expect("genuine complete original event operand");
        assert_eq!(diagnostic.span, word.span());
        let written = &source[word.span().as_range()];
        assert!(matches!(
            written,
            "{HTTP_REQUEST_DATA}" | "HTTP_REQUEST_DATA"
        ));
        assert!(
            body.event_word(
                &tcl_lexer::SourceImage::document(&format!("#{source}")),
                cu.ir_module.lexer_config,
                &reg
            )
            .is_none()
        );
        let mut stale_config = cu.ir_module.lexer_config;
        stale_config.strict_quoting ^= true;
        assert!(
            body.event_word(&cu.ir_module.source, stale_config, &reg)
                .is_none()
        );
    }
}

#[test]
fn irule1005_satisfied_by_matching_collect() {
    // HTTP_REQUEST issues a client-side HTTP::collect, satisfying the
    // HTTP_REQUEST_DATA requirement.
    let ws = flow_warnings(
        "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { log local0. \"data\" }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1005),
        "no IRULE1005 expected — CLIENT_ACCEPTED supplies the collect, got {ws:?}",
    );
}

#[test]
fn irule1006_payload_without_collect_fires() {
    let ws = flow_warnings("when HTTP_REQUEST { HTTP::payload }");
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule1006),
        "expected IRULE1006 on HTTP::payload without HTTP::collect, got {ws:?}",
    );
}

#[test]
fn udp_and_asm_payloads_do_not_require_collect() {
    // F5 exposes every UDP datagram to UDP::payload, and ASM owns the
    // payload in ASM events. Neither protocol has a corresponding
    // `::collect` command, so suffix-based classification was a false
    // positive for both.
    let udp = flow_warnings("when CLIENT_DATA { UDP::payload }");
    assert!(
        !udp.iter()
            .any(|w| w.code == DiagCode::Irule1005 || w.code == DiagCode::Irule1006),
        "UDP data is available without collect, got {udp:?}",
    );
    let asm = flow_warnings("when ASM_REQUEST_VIOLATION { ASM::payload }");
    assert!(
        !asm.iter().any(|w| w.code == DiagCode::Irule1006),
        "ASM owns its payload, got {asm:?}",
    );
}

#[test]
fn mqtt_payload_call_forms_have_distinct_collection_requirements() {
    let bare = flow_warnings("when MQTT_CLIENT_DATA { set bytes [MQTT::payload] }");
    assert!(
        bare.iter()
            .any(|warning| warning.code == DiagCode::Irule1006),
        "bare MQTT payload requires a prior MQTT::collect, got {bare:?}",
    );

    let append = flow_warnings("when MQTT_CLIENT_DATA { MQTT::payload append bytes }");
    assert!(
        append
            .iter()
            .any(|warning| warning.code == DiagCode::Irule1006),
        "MQTT payload append requires collected bytes, got {append:?}",
    );

    for form in ["length", "replace bytes", "prepend bytes"] {
        let source = format!("when MQTT_CLIENT_INGRESS {{ MQTT::payload {form} }}");
        let warnings = flow_warnings(&source);
        assert!(
            !warnings
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "MQTT payload {form} uses the current message without collect, got {warnings:?}",
        );
    }

    let dynamic = flow_warnings("when MQTT_CLIENT_DATA { MQTT::payload $operation bytes }");
    assert!(
        !dynamic
            .iter()
            .any(|warning| warning.code == DiagCode::Irule1006),
        "a dynamic MQTT payload form must abstain, got {dynamic:?}",
    );
}

#[test]
fn mqtt_collection_lifecycle_balances_across_registry_events() {
    let warnings = flow_warnings(
        "when MQTT_CLIENT_INGRESS { MQTT::collect 200 }
             when MQTT_CLIENT_DATA { set bytes [MQTT::payload]; MQTT::release }",
    );
    assert!(
        !warnings.iter().any(|warning| matches!(
            warning.code,
            DiagCode::Irule1005 | DiagCode::Irule1006 | DiagCode::Irule1007 | DiagCode::Irule1008
        )),
        "balanced MQTT collection should be clean, got {warnings:?}",
    );
}

#[test]
fn every_explicit_payload_family_warns_without_collect() {
    for (protocol, event) in [
        ("MR", "MR_DATA"),
        ("RTSP", "RTSP_REQUEST_DATA"),
        ("SCTP", "CLIENT_DATA"),
        ("WS", "WS_CLIENT_DATA"),
    ] {
        let source = format!("when {event} {{ set bytes [{protocol}::payload] }}");
        let warnings = flow_warnings(&source);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "{protocol} payload without collect must warn, got {warnings:?}",
        );
    }
}

#[test]
fn event_owned_payload_families_do_not_invent_collect_lifecycles() {
    for (protocol, event) in [
        ("CACHE", "CACHE_RESPONSE"),
        ("DIAMETER", "DIAMETER_INGRESS"),
        ("GTP", "GTP_SIGNALLING_INGRESS"),
        ("REWRITE", "REWRITE_REQUEST_DONE"),
        ("SIP", "SIP_REQUEST"),
        ("XML", "XML_CONTENT_BASED_ROUTING"),
    ] {
        let source = format!("when {event} {{ set bytes [{protocol}::payload] }}");
        let warnings = flow_warnings(&source);
        assert!(
            !warnings
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "{protocol} payload is event-owned, got {warnings:?}",
        );
    }
}

#[test]
fn irule1007_collect_without_release_fires() {
    let ws = flow_warnings("when CLIENT_ACCEPTED { TCP::collect }");
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule1007),
        "expected IRULE1007, got {ws:?}",
    );
}

#[test]
fn irule1008_release_without_collect_fires() {
    let ws = flow_warnings("when CLIENT_ACCEPTED { TCP::release }");
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule1008),
        "expected IRULE1008, got {ws:?}",
    );
}

#[test]
fn irule1007_satisfied_by_matching_release_same_side() {
    let ws = flow_warnings(
        "when CLIENT_ACCEPTED { TCP::collect }
             when CLIENT_DATA { TCP::release }",
    );
    // Both sides — same side — should NOT fire 1007 or 1008.
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1007),
        "1007 unexpected, got {ws:?}",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1008),
        "1008 unexpected, got {ws:?}",
    );
}

#[test]
fn http_data_event_implicitly_releases_collection() {
    // F5 releases HTTP collection when the matching DATA event completes
    // unless another HTTP::collect starts. An explicit HTTP::release here
    // is optional, unlike TCP and SSL.
    let ws = flow_warnings(
        "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { set payload [HTTP::payload] }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1007),
        "HTTP data event supplies the release, got {ws:?}",
    );
}

#[test]
fn http_response_data_implicitly_releases_server_side_collection() {
    let ws = flow_warnings(
        "when HTTP_RESPONSE { HTTP::collect }
             when HTTP_RESPONSE_DATA { set payload [HTTP::payload] }",
    );
    assert!(
        !ws.iter().any(|warning| matches!(
            warning.code,
            DiagCode::Irule1005 | DiagCode::Irule1006 | DiagCode::Irule1007
        )),
        "registry side and implicit release should satisfy response collection, got {ws:?}",
    );
}

#[test]
fn recollect_inside_http_data_event_still_needs_a_later_release() {
    let ws = flow_warnings(
        "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { HTTP::collect }",
    );
    let hits = ws
        .iter()
        .filter(|warning| warning.code == DiagCode::Irule1007)
        .count();
    assert_eq!(
        hits, 1,
        "the setup collect is implicit-release safe, but the DATA-event recollect is not: {ws:?}",
    );
}

// Side-switch body descent and peer flip.

#[test]
fn collect_inside_clientside_body_is_seen() {
    // The collect lives inside a `clientside { ... }` nesting script.
    // With the BODY role set, the flow check descends into the body
    // and registers the (client-side) collect, so CLIENTSSL_DATA's
    // IRULE1005 requirement is satisfied.  Without the descent the
    // collect was invisible and CLIENTSSL_DATA wrongly fired IRULE1005.
    let ws = flow_warnings(
        "when CLIENTSSL_HANDSHAKE { clientside { SSL::collect } }
             when CLIENTSSL_DATA { log local0. \"data\" }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1005),
        "no IRULE1005 expected — the clientside body supplies the collect, got {ws:?}",
    );
}

#[test]
fn peer_flips_side_for_collect_flow() {
    // `peer { SSL::collect }` evaluates under the *opposite* side of
    // the surrounding client-side event. It therefore satisfies
    // SERVERSSL_DATA but not CLIENTSSL_DATA — the inverse of a plain
    // client-side collection.
    let ws = flow_warnings(
        "when CLIENTSSL_HANDSHAKE { peer { SSL::collect } }
             when CLIENTSSL_DATA { log local0. \"c\" }
             when SERVERSSL_DATA { log local0. \"s\" }",
    );
    let irule1005: Vec<&str> = ws
        .iter()
        .filter(|w| w.code == DiagCode::Irule1005)
        .map(|w| w.message.as_str())
        .collect();
    assert_eq!(
        irule1005.len(),
        1,
        "expected exactly one IRULE1005 (CLIENTSSL_DATA only), got {ws:?}",
    );
    assert!(
        irule1005[0].contains("CLIENTSSL_DATA"),
        "the surviving IRULE1005 must be CLIENTSSL_DATA (peer flipped the collect to the server side), got {irule1005:?}",
    );
}

/// A collect or payload read inside a `clientside` / `serverside` body,
/// in an arm of that body the solver proves unreachable, is not seen.
#[test]
fn irule1007_ignores_a_collect_in_a_dead_side_switch_arm() {
    let live =
        flow_warnings("when CLIENT_ACCEPTED { clientside { if {$x} { TCP::collect 1024 } } }");
    assert!(
        live.iter().any(|w| w.code == DiagCode::Irule1007),
        "expected IRULE1007, got {live:?}",
    );
    let dead =
        flow_warnings("when CLIENT_ACCEPTED { clientside { if {0} { TCP::collect 1024 } } }");
    assert!(
        !dead.iter().any(|w| w.code == DiagCode::Irule1007),
        "no IRULE1007 expected, got {dead:?}",
    );
    let dead_payload =
        flow_warnings("when CLIENT_ACCEPTED { serverside { set n 0; if {$n} { TCP::payload } } }");
    assert!(
        !dead_payload.iter().any(|w| w.code == DiagCode::Irule1006),
        "no IRULE1006 expected, got {dead_payload:?}",
    );
}

#[test]
fn nested_side_switch_warning_keeps_its_absolute_source_span() {
    let source = "when CLIENT_ACCEPTED {\n  serverside {\n    TCP::payload\n  }\n}";
    let warning = flow_warnings(source)
        .into_iter()
        .find(|warning| warning.code == DiagCode::Irule1006)
        .expect("server-side payload without collect");
    let start = usize::try_from(warning.span.start()).unwrap();
    let end = usize::try_from(warning.span.end()).unwrap();
    assert_eq!(&source[start..end], "TCP::payload");
}

#[test]
fn side_switch_specs_have_body_role_and_arity() {
    // Registry-level guard for the spec edits.
    let r = registry();
    for cmd in ["clientside", "serverside", "peer"] {
        assert!(r.is_side_switch(cmd), "{cmd} must be a side-switch");
        assert!(
            r.side_switch_target(cmd).is_some(),
            "{cmd} must declare its side-switch target",
        );
        assert_eq!(
            r.arg_indices_for_role(cmd, &["{ TCP::collect }"], tcl_registry::ArgRole::Body),
            vec![0],
            "{cmd} body script must be ArgRole::Body at index 0",
        );
    }
    // clientside/serverside accept the bare query form or one body;
    // peer requires its body; after's timer body is the trailing arg.
    let cs = r.get("clientside").unwrap().arity;
    assert_eq!((cs.min, cs.max), (0, 1), "clientside arity");
    let ss = r.get("serverside").unwrap().arity;
    assert_eq!((ss.min, ss.max), (0, 1), "serverside arity");
    let pr = r.get("peer").unwrap().arity;
    assert_eq!((pr.min, pr.max), (1, 1), "peer arity");
    // `after`'s deferred timer body runs in its own dispatch context.
    assert_eq!(
        r.get("after").unwrap().body_kind,
        tcl_registry::BodyKind::Structural,
        "after timer body must be Structural",
    );
}

#[test]
fn collect_flow_only_in_irules_dialect() {
    let reg = registry();
    let cu = CompilationUnit::build_for("when CLIENT_ACCEPTED { TCP::collect }", &reg, false);
    let none = find_collect_flow_warnings(&cu, &reg, None);
    assert_eq!(none.len(), 0, "got {none:?}");
}

// IRULE1201 / 1202

fn http_warnings(source: &str) -> Vec<IrulesCheckWarning> {
    source_flow_warnings(source)
        .into_iter()
        .filter(|warning| matches!(warning.code, DiagCode::Irule1201 | DiagCode::Irule1202))
        .collect()
}

#[test]
fn irule1202_double_respond_fires() {
    let ws = http_warnings(
        "when HTTP_REQUEST { HTTP::respond 200 content x; HTTP::respond 404 content y }",
    );
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule1202),
        "expected IRULE1202, got {ws:?}",
    );
}

#[test]
fn irule1201_http_after_respond_fires() {
    let ws = http_warnings(
        "when HTTP_REQUEST { HTTP::respond 200 content ok; HTTP::header Cache-Control no-cache }",
    );
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule1201),
        "expected IRULE1201, got {ws:?}",
    );
}

#[test]
fn irule1201_uses_the_event_profile_instead_of_its_name() {
    // FN regression: ASM_RESPONSE_VIOLATION does not start with `HTTP`,
    // but its event descriptor implies the HTTP profile.
    let ws = http_warnings(
        "when ASM_RESPONSE_VIOLATION { HTTP::respond 200 content ok; HTTP::header insert X-Test yes }",
    );
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule1201),
        "expected profile-driven IRULE1201, got {ws:?}",
    );
}

#[test]
fn irule1201_allows_the_registered_response_state_query() {
    // FP regression: checking whether a response has been sent is the
    // purpose of HTTP::has_responded, so its spec does not require the
    // still-live transaction context.
    let ws = http_warnings(
        "when HTTP_REQUEST { HTTP::respond 200 content ok; set sent [HTTP::has_responded] }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1201),
        "HTTP::has_responded must remain valid after commit, got {ws:?}",
    );
}

#[test]
fn irule1201_clean_when_respond_followed_by_return_only() {
    let ws = http_warnings("when HTTP_REQUEST { HTTP::respond 200 content ok; return }");
    assert!(
        !ws.iter()
            .any(|w| w.code == DiagCode::Irule1201 || w.code == DiagCode::Irule1202),
        "no IRULE1201/1202 expected, got {ws:?}",
    );
}

#[test]
fn irule1201_respond_then_return_inside_catch_warns() {
    // `catch` swallows the `return`, so control continues past the catch
    // with the response already committed — the post-catch HTTP::header
    // insert must still be flagged, and the at-catch-return `responded`
    // state must propagate out.
    let ws = http_warnings(
        "when HTTP_REQUEST {\n\
            \x20   catch { HTTP::respond 200 content ok; return }\n\
            \x20   HTTP::header insert X-Debug \"yes\"\n\
            }",
    );
    let hits: Vec<_> = ws
        .iter()
        .filter(|w| w.code == DiagCode::Irule1201)
        .collect();
    assert_eq!(hits.len(), 1, "expected one IRULE1201, got {ws:?}");
    assert!(
        hits[0].message.contains("HTTP::header"),
        "expected the post-catch HTTP::header to be flagged, got {:?}",
        hits[0].message,
    );
}

#[test]
fn irule1201_catch_body_without_return_still_continues() {
    // A catch whose body does not return continues normally, carrying the
    // responded state to flag the post-catch command.
    let ws = http_warnings(
        "when HTTP_REQUEST {\n\
            \x20   catch { HTTP::respond 200 content ok }\n\
            \x20   HTTP::header insert X-Debug \"yes\"\n\
            }",
    );
    assert_eq!(
        ws.iter().filter(|w| w.code == DiagCode::Irule1201).count(),
        1,
        "expected one IRULE1201, got {ws:?}",
    );
}

#[test]
fn irule1201_catch_without_respond_no_warning() {
    // A benign catch body must not spuriously flag post-catch HTTP
    // commands.
    let ws = http_warnings(
        "when HTTP_REQUEST {\n\
            \x20   catch { set x 1; return }\n\
            \x20   HTTP::header insert X-Debug 1\n\
            }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1201),
        "no IRULE1201 expected for a benign catch, got {ws:?}",
    );
}

#[test]
fn irule1201_only_in_http_events() {
    // CLIENT_ACCEPTED is non-HTTP; HTTP::respond there is silly
    // but IRULE1201/1202 doesn't apply to non-HTTP events.
    let ws = http_warnings(
        "when CLIENT_ACCEPTED { HTTP::respond 200 content x; HTTP::respond 404 content y }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1202),
        "IRULE1202 should not fire outside HTTP events, got {ws:?}",
    );
}

fn http_codes(source: &str) -> Vec<String> {
    let mut c: Vec<String> = http_warnings(source)
        .iter()
        .map(|w| w.code.to_string())
        .collect();
    c.sort();
    c
}

#[test]
fn irule1202_quiet_on_mutually_exclusive_branches() {
    // A respond in each arm of an if/else is NOT a double-respond — only
    // one executes per path.  The path-sensitive walk keeps the branch
    // states separate (the linear scan wrongly fired IRULE1202 here).
    assert_eq!(
        http_codes(
            "when HTTP_REQUEST { if {$x} { HTTP::respond 200 content a } \
             else { HTTP::respond 404 content b } }"
        )
        .len(),
        0
    );
    // Same for mutually-exclusive switch arms.
    assert_eq!(
        http_codes(
            "when HTTP_REQUEST { switch $x { a { HTTP::respond 200 content a } \
             b { HTTP::respond 404 content b } } }"
        )
        .len(),
        0
    );
}

#[test]
fn irule1202_reports_one_diagnostic_for_multiple_predecessor_proofs() {
    let ws = http_warnings(
        "when HTTP_REQUEST {\
             if {$x} { HTTP::respond 200 content a } \
             else { HTTP::respond 404 content b }; \
             HTTP::respond 500 content c \
             }",
    );
    let final_respond: Vec<_> = ws
        .iter()
        .filter(|warning| {
            warning.code == DiagCode::Irule1202
                && warning.message.contains("Multiple 'HTTP::respond'")
        })
        .collect();
    assert_eq!(
        final_respond.len(),
        1,
        "one source command must produce one diagnostic, got {ws:?}",
    );
}

#[test]
fn irule1201_quiet_when_respond_and_use_are_exclusive() {
    // respond in one branch, an HTTP command in the *other* — no path
    // both responds and then uses HTTP, so nothing fires.
    assert_eq!(
        http_codes(
            "when HTTP_REQUEST { if {$x} { HTTP::respond 200 content a } \
             else { HTTP::header insert Foo bar } }"
        )
        .len(),
        0
    );
}

#[test]
fn irule1201_fires_through_catch_body() {
    // A respond inside `catch { … }` still commits the response, so a
    // following HTTP command is flagged (the linear scan missed this).
    assert!(http_codes(
            "when HTTP_REQUEST { catch { HTTP::respond 200 content a }; HTTP::header insert Foo bar }"
        )
        .contains(&"IRULE1201".to_string()));
}

#[test]
fn irule1201_quiet_on_unregistered_http_subcommand() {
    // `HTTP::log` is not a registered HTTP:: command, so it must not trip
    // IRULE1201 (it carries no registry context trait).
    assert!(
        !http_codes("when HTTP_REQUEST { HTTP::respond 200 content a; HTTP::log foo }")
            .contains(&"IRULE1201".to_string())
    );
}

#[test]
fn irule1202_fires_after_loop_respond() {
    // A respond in a loop body, then a respond after the loop: when the
    // loop runs at least once, the second respond is a double-respond.
    assert!(
        http_codes(
            "when HTTP_REQUEST { foreach i $list { HTTP::respond 200 content a }; \
             HTTP::respond 404 content b }"
        )
        .contains(&"IRULE1202".to_string())
    );
}

/// A respond or redirect in a block the solver proves unreachable commits
/// no response, so an HTTP command after it is not issued after one, and
/// an HTTP command in such a block is issued after none; a live arm's
/// respond still commits.
#[test]
fn irule1201_ignores_a_respond_in_a_dead_arm() {
    for source in [
        "when HTTP_REQUEST { if {0} { HTTP::respond 200 }; HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { set flag 0; if {$flag} { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { if {0} { HTTP::redirect \"https://example.com/\" }; \
             HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { if {1} { log local0. ok } else { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { while {0} { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { set mode b; \
             switch $mode { a { HTTP::respond 200 } default { log local0. other } }; \
             HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { HTTP::respond 200; if {1} { return }; \
             HTTP::header insert X-Custom val }",
    ] {
        assert!(
            http_codes(source).is_empty(),
            "{source}: {:?}",
            http_codes(source)
        );
    }
    for source in [
        "when HTTP_REQUEST { if {1} { HTTP::respond 200 }; HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { set flag 1; if {$flag} { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { if {$x} { HTTP::respond 200 }; HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { if {0} { log local0. no } else { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
        "when HTTP_REQUEST { set mode a; \
             switch $mode { a { HTTP::respond 200 } default { log local0. other } }; \
             HTTP::header insert X-Custom val }",
    ] {
        assert!(
            http_codes(source).contains(&"IRULE1201".to_owned()),
            "{source}: {:?}",
            http_codes(source),
        );
    }
}

/// A second response in an arm that never runs is not a second response,
/// and a first one that never runs commits nothing for the second to
/// follow.
#[test]
fn irule1202_ignores_a_respond_in_a_dead_arm() {
    for source in [
        "when HTTP_REQUEST { HTTP::respond 200; if {0} { HTTP::respond 404 } }",
        "when HTTP_REQUEST { if {0} { HTTP::respond 200 }; HTTP::respond 404 }",
        "when HTTP_REQUEST { set flag 0; \
             if {$flag} { HTTP::redirect \"https://example.com/\" }; HTTP::respond 404 }",
    ] {
        assert!(
            http_codes(source).is_empty(),
            "{source}: {:?}",
            http_codes(source)
        );
    }
    for source in [
        "when HTTP_REQUEST { HTTP::respond 200; if {1} { HTTP::respond 404 } }",
        "when HTTP_REQUEST { HTTP::respond 200; if {$x} { HTTP::respond 404 } }",
    ] {
        assert!(
            http_codes(source).contains(&"IRULE1202".to_owned()),
            "{source}: {:?}",
            http_codes(source),
        );
    }
}

#[test]
fn bare_redirect_commits_response() {
    // The bare iRules `redirect` (not just `HTTP::redirect`) commits a
    // response — derived from the registry's ResponseCommit side-effect.
    assert!(commits_response_commands().contains("redirect"));
    assert!(commits_response_commands().contains("HTTP::respond"));
}

// IRULE4004

fn hoist_warnings(source: &str) -> Vec<IrulesCheckWarning> {
    let cu = CompilationUnit::build_for(source, &registry(), false);
    find_hoistable_set_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)))
}

#[test]
fn irule4004_literal_set_in_per_request_event_fires() {
    let ws = hoist_warnings(r#"when HTTP_REQUEST { set svc "foo" }"#);
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "expected IRULE4004, got {ws:?}",
    );
}

#[test]
fn irule4004_registry_only_per_request_event_fires() {
    // TP: the registry's per-request classification is the only source —
    // a compiler-local event list would not carry ASM_REQUEST_DONE.
    let ws = hoist_warnings(r#"when ASM_REQUEST_DONE { set svc "foo" }"#);
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "expected registry-driven IRULE4004, got {ws:?}",
    );
}

#[test]
fn irule4004_dynamic_set_clean() {
    // Value depends on per-request data — can't hoist.
    let ws = hoist_warnings("when HTTP_REQUEST { set svc [HTTP::host] }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "no IRULE4004 expected — value depends on request, got {ws:?}",
    );
}

/// A value the lattice proves reads no request data, whatever its
/// commands: `[string range ABCDEFG 0 3]` is `ABCD` on every
/// request and hoists like a literal, where the same range over the host
/// header does not.
#[test]
fn irule4004_proven_command_value_is_hoistable() {
    let ws = hoist_warnings("when HTTP_REQUEST { set svc [string range ABCDEFG 0 3] }");
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "expected IRULE4004 on a proven value, got {ws:?}",
    );
    let ws = hoist_warnings("when HTTP_REQUEST { set svc [string range [HTTP::host] 0 3] }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "no IRULE4004 expected — the value reads the request, got {ws:?}",
    );
}

#[test]
fn irule4004_var_substitution_clean() {
    let ws = hoist_warnings("when HTTP_REQUEST { set svc $session }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "no IRULE4004 expected — value uses $session, got {ws:?}",
    );
}

#[test]
fn irule4004_per_request_reset_is_not_hoistable() {
    // FP regression from the third-party corpus: moving the initial zero
    // changes later requests after a branch sets the connection variable.
    let ws = hoist_warnings(
        "when HTTP_REQUEST { set is_reject 0; if {[HTTP::path] eq \"/deny\"} { set is_reject 1 } }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "a per-request reset must stay in the request event, got {ws:?}",
    );
}

#[test]
fn irule4004_conditional_literal_set_is_not_hoistable() {
    let ws = hoist_warnings(
        "when HTTP_REQUEST { if {[HTTP::path] eq \"/health\"} { set health_check 1 } }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "hoisting a conditional assignment would make it unconditional, got {ws:?}",
    );
}

#[test]
fn irule4004_cross_event_write_is_not_hoistable() {
    let ws =
        hoist_warnings("when HTTP_REQUEST { set state 0 }\nwhen HTTP_RESPONSE { set state 1 }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "a connection variable written by another event is not invariant, got {ws:?}",
    );
}

#[test]
fn irule4004_counts_writes_in_complexity_guarded_events() {
    let padding = "x".repeat(crate::ssa::DEEP_ANALYSIS_BODY_BYTES);
    let source = format!(
        "when HTTP_REQUEST {{ set state 0 }}\n\
             when HTTP_RESPONSE {{ # {padding}\n set state 1 }}"
    );
    let cu = CompilationUnit::build_for(&source, &registry(), false);
    let response = cu
        .functions()
        .find(|fu| fu.name.starts_with("::when::HTTP_RESPONSE"))
        .expect("HTTP_RESPONSE event is lowered");
    assert!(
        response.complexity_guarded,
        "the control event must be guarded"
    );

    let ws = find_hoistable_set_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)));
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "a guarded event's second write must suppress the hoist warning, got {ws:?}",
    );
}

/// A second write in a block the solver proves unreachable never runs,
/// so the first is still the variable's only write; one that runs, or
/// might, keeps the assignment where it is.
#[test]
fn irule4004_counts_only_the_writes_that_run() {
    for source in [
        "when HTTP_REQUEST { set svc foo; if {0} { set svc bar } }",
        "when HTTP_REQUEST { set svc foo }\nwhen HTTP_RESPONSE { if {0} { set svc bar } }",
    ] {
        assert!(
            hoist_warnings(source)
                .iter()
                .any(|w| w.code == DiagCode::Irule4004),
            "{source}"
        );
    }
    for source in [
        "when HTTP_REQUEST { set svc foo; if {1} { set svc bar } }",
        "when HTTP_REQUEST { set svc foo; if {$x} { set svc bar } }",
        "when HTTP_REQUEST { set svc foo }\nwhen HTTP_RESPONSE { if {$x} { set svc bar } }",
    ] {
        assert!(
            !hoist_warnings(source)
                .iter()
                .any(|w| w.code == DiagCode::Irule4004),
            "{source}"
        );
    }
}

#[test]
fn irule4004_array_and_static_destinations_abstain() {
    let ws =
        hoist_warnings("when HTTP_REQUEST { set state(current) 0; set static::feature_enabled 1 }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "array and static variables need different hoist destinations, got {ws:?}",
    );
}

#[test]
fn irule4004_set_in_once_per_connection_clean() {
    // FP control: the registry classifies CLIENT_ACCEPTED as
    // once-per-connection, so there is nothing to hoist.
    let ws = hoist_warnings(r#"when CLIENT_ACCEPTED { set svc "foo" }"#);
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "no IRULE4004 expected — already once-per-connection, got {ws:?}",
    );
}

#[test]
fn irule4004_unknown_event_abstains() {
    // TN: an event absent from the registry is not guessed to be
    // per-request from its spelling.
    let ws = hoist_warnings(r#"when CUSTOM_REQUEST { set svc "foo" }"#);
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "unknown event should abstain, got {ws:?}",
    );
}

#[test]
fn irule4004_only_in_irules_dialect() {
    let cu =
        CompilationUnit::build_for(r#"when HTTP_REQUEST { set svc "foo" }"#, &registry(), false);
    let none = find_hoistable_set_warnings(&cu, None);
    assert_eq!(none.len(), 0, "got {none:?}");
}

// IRULE4002

fn generic_warnings(source: &str) -> Vec<IrulesCheckWarning> {
    let cu = CompilationUnit::build_for(source, &registry(), false);
    find_generic_static_name_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)), None)
}

fn generic_warnings_with(source: &str, patterns: &[String]) -> Vec<IrulesCheckWarning> {
    let cu = CompilationUnit::build_for(source, &registry(), false);
    find_generic_static_name_warnings(
        &cu,
        Some(SurfaceQuery::any_release(Family::F5Irules)),
        Some(patterns),
    )
}

#[test]
fn generic_static_warning_uses_resolved_root_keys_aliases_and_original_source_locations() {
    // naming.analysis.resolved-static-variable-patterns
    // docs/design/analysis/name-resolution-proofs/resolved-static-variable-patterns.md
    // Fixed source-analysis controls; no iRule event or physical TMM executes here.
    for operation in [
        "set static::debug 1",
        "set ::static::debug 1",
        "set ::::static::::debug 1",
        "upvar #0 ::static::debug shared; set shared 1",
    ] {
        let source = format!("when RULE_INIT {{{operation}}}");
        let warnings = generic_warnings(&source);
        let selected: Vec<_> = warnings
            .iter()
            .filter(|warning| warning.code == DiagCode::Irule4002)
            .collect();
        assert_eq!(selected.len(), 1, "{source}: {warnings:?}");
        let text = &source[selected[0].span.start() as usize..selected[0].span.end() as usize];
        assert_eq!(
            text,
            operation
                .rsplit_once("; ")
                .map_or(operation, |(_, write)| write)
        );
    }
    let source = "when RULE_INIT {set static::debug 1; set ::static::debug 2}";
    assert_eq!(
        generic_warnings(source)
            .iter()
            .filter(|warning| warning.code == DiagCode::Irule4002)
            .count(),
        1
    );
    for source in [
        "when RULE_INIT {set other::static::debug 1}",
        "when RULE_INIT {set ::static::inner::debug 1}",
        "when HTTP_REQUEST {set debug 1}",
        "when RULE_INIT {if {0} {set ::static::debug 1}}",
    ] {
        assert!(
            generic_warnings(source)
                .iter()
                .all(|warning| warning.code != DiagCode::Irule4002),
            "{source}"
        );
    }
    let patterns = vec![r"^debug\(key\)$".to_owned()];
    assert_eq!(
        generic_warnings_with("when RULE_INIT {set ::static::debug(key) 1}", &patterns)
            .iter()
            .filter(|warning| warning.code == DiagCode::Irule4002)
            .count(),
        1
    );
}

#[test]
fn generic_static_warning_declines_foreign_module_source_without_spelling_fallback() {
    // naming.analysis.resolved-static-variable-patterns
    // docs/design/analysis/name-resolution-proofs/resolved-static-variable-patterns.md
    // A real built source Module loses its actual document correspondence.
    let source = "when RULE_INIT {set static::debug 1}";
    let mut unit = CompilationUnit::build_for(source, &registry(), false);
    assert_eq!(
        find_generic_static_name_warnings(
            &unit,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
            None
        )
        .iter()
        .filter(|warning| warning.code == DiagCode::Irule4002)
        .count(),
        1
    );
    unit.ir_module.source = tcl_lexer::SourceImage::document("foreign source");
    assert_eq!(
        find_generic_static_name_warnings(
            &unit,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
            None
        ),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
}

#[test]
fn irule4002_fires_for_generic_static_name() {
    let ws = generic_warnings("when RULE_INIT { set static::debug 1 }");
    assert!(
        ws.iter()
            .any(|w| w.code == DiagCode::Irule4002 && w.message.contains("'static::debug'")),
        "expected IRULE4002 for static::debug, got {ws:?}",
    );
}

#[test]
fn irule4002_quiet_for_specific_static_name() {
    let ws = generic_warnings("when RULE_INIT { set static::myapp_cache_ttl 60 }");
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4002),
        "no IRULE4002 expected for a specific name, got {ws:?}",
    );
}

#[test]
fn irule4002_deduplicates_across_events() {
    let src = "when RULE_INIT { set static::timeout 30 }\n\
                   when HTTP_REQUEST { set static::timeout 30 }";
    let ws = generic_warnings(src);
    assert_eq!(
        ws.iter().filter(|w| w.code == DiagCode::Irule4002).count(),
        1,
        "expected a single deduplicated IRULE4002, got {ws:?}",
    );
}

/// A generic `static::` name written only in a block the solver proves
/// unreachable collides with nothing; the warning lands on the write
/// that runs.
#[test]
fn irule4002_ignores_a_static_in_a_dead_arm() {
    assert_eq!(
        generic_warnings("when RULE_INIT { if {0} { set static::debug 1 } }"),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    assert!(
        generic_warnings("when RULE_INIT { if {$x} { set static::debug 1 } }")
            .iter()
            .any(|w| w.code == DiagCode::Irule4002)
    );
    let source = "when RULE_INIT { if {0} { set static::debug 1 }; set static::debug 2 }";
    let ws = generic_warnings(source);
    assert_eq!(ws.len(), 1, "got {ws:?}");
    let span = ws[0].span;
    assert_eq!(
        &source[span.start() as usize..span.end() as usize],
        "set static::debug 2"
    );
}

#[test]
fn irule4002_only_in_irules_dialect() {
    let cu =
        CompilationUnit::build_for("when RULE_INIT { set static::debug 1 }", &registry(), false);
    assert_eq!(
        find_generic_static_name_warnings(&cu, None, None),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
}

#[test]
fn is_generic_static_name_matches_default_set() {
    let compiled = compile_generic_patterns(None);
    assert!(is_generic_static_name("static::debug", &compiled));
    assert!(is_generic_static_name("static::DEBUG", &compiled));
    assert!(is_generic_static_name("static::max_retries", &compiled));
    assert!(!is_generic_static_name("static::myapp_token", &compiled));
    assert!(!is_generic_static_name("static::debugger", &compiled));
}

#[test]
fn irule4002_custom_patterns_replace_defaults() {
    // A custom pattern set replaces the defaults wholesale: `myapp_token`
    // fires, while `debug` — generic under the defaults — stays quiet.
    let patterns = vec![r"^myapp_".to_owned()];
    let ws = generic_warnings_with("when RULE_INIT { set static::myapp_token 1 }", &patterns);
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule4002),
        "expected IRULE4002 for static::myapp_token under custom patterns, got {ws:?}",
    );
    let quiet = generic_warnings_with("when RULE_INIT { set static::debug 1 }", &patterns);
    assert!(
        !quiet.iter().any(|w| w.code == DiagCode::Irule4002),
        "static::debug should be quiet once defaults are replaced, got {quiet:?}",
    );
}

#[test]
fn irule4002_empty_patterns_disable_check() {
    // An explicit empty pattern list disables IRULE4002 entirely.
    let ws = generic_warnings_with("when RULE_INIT { set static::debug 1 }", &[]);
    assert!(
        ws.iter().all(|w| w.code != DiagCode::Irule4002),
        "empty pattern list should disable IRULE4002, got {ws:?}",
    );
}

/// A flag a `switch` arm sets is no constant: the `HTTP::respond` under
/// `if {$is_api}` is reachable, so the header insert after it may follow a
/// committed response. The same handler with no arm writing the flag
/// leaves the respond unreachable and reports nothing.
#[test]
fn irule1201_reads_a_flag_a_switch_arm_sets() {
    let written = concat!(
        "when HTTP_REQUEST { set is_api 0; switch -glob [HTTP::uri] { \"/api*\" { set is_api 1 } }; ",
        "if {$is_api} { HTTP::respond 403 }; HTTP::header insert X-Seen 1 }"
    );
    let ws = http_warnings(written);
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule1201),
        "expected IRULE1201, got {ws:?}",
    );
    let untouched = concat!(
        "when HTTP_REQUEST { set is_api 0; switch -glob [HTTP::uri] { \"/api*\" { set other 1 } }; ",
        "if {$is_api} { HTTP::respond 403 }; HTTP::header insert X-Seen 1 }"
    );
    let ws = http_warnings(untouched);
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule1201),
        "the respond is unreachable, got {ws:?}",
    );
}

/// The same for a drop under a flag a `switch` arm sets.
#[test]
fn irule5002_reads_a_flag_a_switch_arm_sets() {
    let written = concat!(
        "when CLIENT_ACCEPTED { set bad 0; switch -glob [IP::client_addr] { 10.* { set bad 1 } }; ",
        "if {$bad} { drop } }"
    );
    assert!(
        drop_codes(written).contains(&"IRULE5002".to_owned()),
        "{:?}",
        drop_codes(written),
    );
    let untouched = concat!(
        "when CLIENT_ACCEPTED { set bad 0; switch -glob [IP::client_addr] { 10.* { set other 1 } }; ",
        "if {$bad} { drop } }"
    );
    assert!(
        drop_codes(untouched).is_empty(),
        "{:?}",
        drop_codes(untouched)
    );
}

/// A write in an arm of a `switch` the flow graph keeps as one statement
/// counts: the `set svc foo` is one of two writes, so hoisting it to a
/// once-per-connection event would change what a later request sees.
#[test]
fn irule4004_counts_the_writes_a_switch_arm_makes() {
    let ws = hoist_warnings(
        "when HTTP_REQUEST { set svc foo; switch -glob [HTTP::uri] { /a* { set svc bar } }; pool $svc }",
    );
    assert!(
        !ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "no IRULE4004 expected — an arm writes svc too, got {ws:?}",
    );
    let ws = hoist_warnings(
        "when HTTP_REQUEST { set svc foo; switch -glob [HTTP::uri] { /a* { set other bar } }; pool $svc }",
    );
    assert!(
        ws.iter().any(|w| w.code == DiagCode::Irule4004),
        "expected IRULE4004 when no arm writes svc, got {ws:?}",
    );
}
