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
use crate::run_pipeline;

#[test]
fn taint_replay_availability_is_separate_from_empty_warning_arrays() {
    let mut result = run_pipeline("set value 1", "tcl8.6");
    let source = result.source.clone();
    let index = LineIndex::new(&source);
    assert!(result.unit.top_level.cfg.namespace_context.is_some());
    result.unit.top_level.cfg.namespace_context = None;
    assert!(retained_explorer_source(&result, &source).is_some());
    assert_eq!(serialise_taint(&result, &index, &source), json!([]));
    let replays = serialise_taint_replays(&result, &index, &source);
    assert_eq!(replays["status"], "unavailable");
    assert_eq!(replays["complete"], false);
    let stats = serialise_stats(&result);
    assert_eq!(stats["taintWarnings"], 0);
    assert_eq!(stats["taintReplayComplete"], false);
    assert!(serialise_taint_replays(&result, &index, "foreign source").is_null());
}

#[test]
fn interproc_method_rows_retain_equal_labels_and_scoped_call_ids() {
    use tcl_compiler::interprocedural::{MethodSummary, ProcSummary};
    use tcl_compiler::ir::{MethodKind, ProcedureIdIssuer};

    let first_owner = ProcedureIdIssuer::default();
    let second_owner = ProcedureIdIssuer::default();
    let first = first_owner.allocate();
    let second = second_owner.allocate();
    assert_eq!(first.ordinal(), second.ordinal());
    assert_ne!(first, second);
    let mut interproc = InterproceduralAnalysis::default();
    for id in [first, second] {
        let mut base = ProcSummary::unknown("::C::m");
        base.calls = vec![first, second];
        interproc.methods.insert(
            id,
            MethodSummary {
                base,
                class_name: "::C".to_owned(),
                method_kind: MethodKind::Method,
                reads_instance_vars: std::collections::HashSet::default(),
                writes_instance_vars: std::collections::HashSet::default(),
                calls_my: Vec::new(),
                calls_next: false,
            },
        );
    }
    let encoded = serialise_interproc(
        &interproc,
        &tcl_compiler::unit_scope::CallSiteEvidence::default(),
    );
    let rows = encoded.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], rows[1]["name"]);
    assert_eq!(rows[0]["bodyId"], rows[1]["bodyId"]);
    assert_ne!(rows[0]["bodyKey"], rows[1]["bodyKey"]);
    for row in rows {
        assert_eq!(
            row["callBodyKeys"],
            json!([first.reporting_key(), second.reporting_key()])
        );
        assert_eq!(row["calls"], json!(["::C::m", "::C::m"]));
        assert_eq!(row["methodKind"], "method");
    }
}

#[test]
fn ir_rows_keep_equal_names_and_distinct_source_bodies() {
    let source = "proc p {} {return FIRST}; proc p {} {return LAST}; proc ::top {} {return NAMED}";
    let result = run_pipeline(source, "tcl8.6");
    let ir = serialise_ir(&result.unit.ir_module, &LineIndex::new(source), source);
    let rows: Vec<_> = ir["procedures"].as_object().unwrap().values().collect();
    assert_eq!(rows.len(), 3, "{ir}");
    let p_rows: Vec<_> = rows.iter().filter(|row| row["name"] == "::p").collect();
    assert_eq!(p_rows.len(), 2, "{ir}");
    assert_ne!(p_rows[0]["bodyId"], p_rows[1]["bodyId"]);
    assert_ne!(p_rows[0]["range"], p_rows[1]["range"]);
    assert_ne!(p_rows[0]["body"], p_rows[1]["body"]);
    assert_eq!(rows.iter().filter(|row| row["name"] == "::top").count(), 1);
}

#[test]
// Implementation contract: naming.variable.registry-event-frame-identity
// docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
fn original_event_semantic_view_preserves_conditional_descriptor_without_worker_grant() {
    let source = "when HTTP_REQUEST {set local 1}";
    let result = run_pipeline(source, "f5-irules");
    let encoded = serialise_semantic(&result, &LineIndex::new(source), source);
    let event = encoded
        .as_array()
        .unwrap()
        .iter()
        .find_map(|function| {
            let event = &function["conditionalEvent"];
            (!event.is_null()).then_some(event)
        })
        .expect("selected event descriptor appears in semantic view");
    assert_eq!(event["event"], "HTTP_REQUEST");
    assert_eq!(event["purpose"], "conditional-source-frame");
    assert_eq!(event["licensesEnteredEvent"], false);
    let source = "namespace eval ::when {}; proc ::when::HTTP_REQUEST {} {set local 1}";
    let ordinary = run_pipeline(source, "f5-irules");
    let encoded = serialise_semantic(&ordinary, &LineIndex::new(source), source);
    assert!(
        encoded
            .as_array()
            .unwrap()
            .iter()
            .all(|function| function["conditionalEvent"].is_null())
    );
}

#[test]
fn completion_terminator_preserves_pending_native_return() {
    use tcl_registry::completion_route::{InvocationCompletionRoute, ReturnCompletionRoute};
    let source = "error BOOM";
    let result = run_pipeline(source, "tcl8.6");
    let snapshots = result.all_snapshots();
    let term = Terminator::Complete {
        route: InvocationCompletionRoute::Return(ReturnCompletionRoute {
            eventual_code: tcl_registry::completion::CompletionCode::Error,
            remaining_level: 2,
        }),
        span: None,
    };
    let encoded = terminator_dict(
        &snapshots[0].unit.cfg,
        Some(&term),
        &LineIndex::new(source),
        source,
    );
    assert_eq!(encoded["type"], "complete");
    assert_eq!(encoded["completion"]["route"]["kind"], "return");
    assert_eq!(encoded["completion"]["route"]["eventualCode"], 1);
    assert_eq!(encoded["completion"]["route"]["remainingLevel"], 2);
    assert_eq!(encoded["completion"]["normalPossible"], false);
    assert_eq!(encoded["completion"]["abruptPossible"], true);
    assert!(block_successors(&snapshots[0].unit.cfg, Some(&term)).is_empty());
}

#[test]
fn native_lookup_decline_and_interpreter_world_domains_remain_distinct() {
    let reason = OwnedInvocationResolutionUnresolved::UnprovedNativeImplementationLookup {
        lookup: tcl_registry::native_compilation::NativeCompilerImplementationLookup {
            ensemble: "::binary",
            member: "encode",
            slot: "::tcl::binary::encode",
            command: "binary",
            prepended: &["encode"],
        },
    };
    let encoded = unproved_implementation_lookup(&reason);
    assert_eq!(encoded["ensemble"], "::binary");
    assert_eq!(encoded["member"], "encode");
    assert_eq!(encoded["slot"], "::tcl::binary::encode");
    assert_eq!(encoded["command"], "binary");
    assert_eq!(encoded["prepended"], json!(["encode"]));
    assert_eq!(
        invocation_resolution_label(&InvocationResolution::Unresolved(reason)),
        "unresolved-unproved-native-implementation-lookup"
    );
    assert_eq!(
        world_domain_name(WorldRegionKind::InterpreterResult),
        "interpreter-result"
    );
    assert_eq!(
        world_domain_name(WorldRegionKind::CompletionState),
        "completion-state"
    );
    assert_eq!(
        registry_world_domain_name(tcl_registry::WorldStateDomain::InterpreterResult),
        "interpreter-result"
    );
    assert_eq!(
        registry_world_domain_name(tcl_registry::WorldStateDomain::CompletionState),
        "completion-state"
    );
}

#[test]
fn meta_lists_all_dialects_views_and_severities() {
    let meta = serialise_meta();
    // Every environment the registry offers is exposed, and in its order.
    // Derived from the selectable set rather than pinned to a count: the
    // invariant worth holding is "the explorer drops none of them".
    let dialects: Vec<&str> = meta["dialects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    let selectable: Vec<&str> = tcl_dialect::model::EnvironmentRegistry::compiled_selectable()
        .iter()
        .map(|environment| environment.id.as_str())
        .collect();
    assert_eq!(dialects, selectable);
    // Every entry carries its labels, like the `views` entries.
    for entry in meta["dialects"].as_array().unwrap() {
        assert!(entry["displayName"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(entry["shortName"].as_str().is_some_and(|s| !s.is_empty()));
    }
    // The original 27 views, plus World SSA, six durable compiler
    // artefact views, the optimisation-pass toggle surface and the AOT
    // plan.
    assert_eq!(meta["views"].as_array().unwrap().len(), 36);
    assert_eq!(meta["severities"], json!(["error", "warning", "info"]));
    let traits = meta["traits"]
        .as_array()
        .expect("trait presentation metadata");
    assert_eq!(traits.len(), tcl_registry::traits::Trait::ALL.len());
    assert!(traits.iter().all(|item| {
        item["name"].as_str().is_some_and(|text| !text.is_empty())
            && item["summary"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
            && item["group"].as_str().is_some_and(|text| !text.is_empty())
    }));
    // The parse-tree tab is the CST; there is no `greentree` entry.
    assert_eq!(
        meta["views"][0],
        json!({ "id": "cst", "label": "CST", "payload": "cst", "group": "compiler", "renderKind": "frontend" })
    );
    assert!(
        !meta["views"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["id"] == "greentree"),
        "greentree tab must be dropped"
    );
}

#[test]
fn optimiser_passes_preserve_set_only_tk_profile() {
    let result = run_pipeline("proc recurse {} { recurse }\n", "tk");
    let serialised = serialise_result(&result);
    let passes = serialised["optimiserPasses"]
        .as_array()
        .expect("optimiser passes");
    assert!(
        !passes
            .iter()
            .flat_map(|pass| { pass["optimisations"].as_array().into_iter().flatten() })
            .any(|optimisation| optimisation["code"] == "O121"),
        "Tk must not offer tailcall: {passes:?}"
    );
}

#[test]
fn wasm_view_exposes_canonical_semantic_plan_and_serialised_output() {
    let result = run_pipeline("string length hello", "tcl8.6");
    let wasm = serialise_result(&result)["wasm"].clone();
    let header = &wasm.as_array().expect("WASM entries")[0];

    assert_eq!(header["codegenPlan"]["kind"], "generic-invoke");
    assert!(header["codegenPlan"]["semanticDecline"].is_null());
    assert_eq!(header["codegenPlan"]["regionPlanStatus"], "available");
    let regions = header["codegenPlan"]["regions"]
        .as_array()
        .expect("per-region plan evidence");
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0]["node"], json!([0]));
    assert_eq!(regions[0]["selectedKind"], "generic-prebuilt-argv");
    assert_eq!(regions[0]["operation"]["kind"], "intrinsic");
    assert_eq!(regions[0]["operation"]["id"], "string-length");
    assert_eq!(regions[0]["slowPath"]["kind"], "prebuilt-argv");
    assert_eq!(regions[0]["slowPath"]["wordReplay"], false);
    assert_eq!(regions[0]["candidates"][0]["kind"], "guarded-direct");
    assert_eq!(
        regions[0]["candidates"][0]["reason"],
        "direct-callee-identity-unavailable"
    );
    assert_eq!(regions[0]["candidates"][1]["kind"], "guarded-intrinsic");
    assert_eq!(regions[0]["candidates"][1]["reason"], "pass-disabled");
    assert!(
        header["text"]
            .as_str()
            .expect("serialised WAT")
            .contains("tcl_invoke_argv")
    );
}

#[test]
fn wasm_view_exposes_common_native_i64_selection_evidence() {
    use tcl_codegen::codegen::wasm::{SemanticOptimisationPassId, WasmCompileOptions};

    let result = run_pipeline(
        "proc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n",
        "tcl9.0",
    );
    let options = [
        SemanticOptimisationPassId::DirectProc,
        SemanticOptimisationPassId::MaterialisableSlot,
        SemanticOptimisationPassId::FrameElision,
        SemanticOptimisationPassId::NativeInteger,
        SemanticOptimisationPassId::SemanticOperationSpecialisation,
    ]
    .into_iter()
    .fold(
        WasmCompileOptions::hosted().for_sealed_program(),
        WasmCompileOptions::with_semantic_optimisation,
    );
    let wasm = serialise_wasm_with_options(&result, options);
    let plan = &wasm.as_array().expect("WASM entries")[0]["codegenPlan"];
    assert_eq!(plan["kind"], "native-i64-add");
    assert_eq!(plan["nativeI64Add"]["callee"], "::add");
    assert_eq!(plan["nativeI64Add"]["operands"], json!([2, 4]));
    assert_eq!(
        plan["nativeI64Add"]["boundaryOperation"],
        json!({ "kind": "intrinsic", "id": "channel-write" })
    );
    assert_eq!(plan["nativeI64Add"]["frameElided"], true);
    assert_eq!(plan["nativeI64Add"]["closedProgramStatements"], 4);
    assert_eq!(plan["nativeDeclines"], json!([]), "nothing rejected it");
}

/// The `aot` payload is the `wasm` header's plan, and the plan names each
/// premise the sealed native addition rejected: typed, in evaluation
/// order, with the call each concerns.
#[test]
fn aot_view_carries_the_plan_and_every_rejected_native_premise() {
    let result = run_pipeline(
        "proc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n",
        "tcl9.0",
    );
    let mut config = SemanticOptimisationConfig::new();
    for pass in [
        SemanticOptimisationPassId::DirectProc,
        SemanticOptimisationPassId::MaterialisableSlot,
        SemanticOptimisationPassId::FrameElision,
        SemanticOptimisationPassId::SemanticOperationSpecialisation,
    ] {
        config.enable(pass);
    }
    let data = serialise_result_with_optimisations(&result, config);
    assert_eq!(data["aot"], data["wasm"][0]["codegenPlan"]);

    let declines = data["aot"]["nativeDeclines"]
        .as_array()
        .expect("the native record");
    let named: Vec<(&str, &str)> = declines
        .iter()
        .map(|decline| {
            (
                decline["premise"].as_str().expect("a premise"),
                decline["reason"].as_str().expect("a reason"),
            )
        })
        .collect();
    assert_eq!(
        named,
        [
            ("sealed-program", "hosted-environment"),
            ("pass", "pass-disabled"),
            ("closed-program", "hosted-environment"),
            ("direct-body", "native-integer-pass-disabled"),
            ("frame", "frame-not-elidable"),
            ("actuals", "hosted-top-level-observable"),
            ("native-integer", "pass-disabled"),
        ]
    );
    assert_eq!(declines[0]["sites"], json!([]));
    assert_eq!(declines[1]["detail"], json!({ "pass": "native-integer" }));
    assert_eq!(
        declines[3]["sites"],
        json!([{
            "function": "::top",
            "block": 0,
            "statementIndex": 3,
            "nestedArgument": 0,
        }])
    );
}

/// The toggle surface a front end renders from: every pass, in a stable
/// order, with the state the shown module was built with — and the same
/// catalogue in `meta`, so the panel can be drawn before the first
/// compile.
#[test]
fn semantic_optimisations_view_carries_every_pass_and_its_state() {
    let result = run_pipeline("set a 1\nincr a\n", "tcl9.0");
    let off = serialise_result(&result);
    let ids: Vec<&str> = off["semanticOptimisations"]["passes"]
        .as_array()
        .expect("pass rows")
        .iter()
        .map(|row| row["id"].as_str().expect("a pass id"))
        .collect();
    assert_eq!(
        ids,
        SemanticOptimisationPassId::all()
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        off["semanticOptimisations"]["passes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["enabled"] == false),
        "the default is the generic lowering"
    );

    let on = serialise_result_with_optimisations(
        &result,
        SemanticOptimisationConfig::from_names("native-tier").expect("a valid group"),
    );
    let enabled: Vec<&str> = on["semanticOptimisations"]["passes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["enabled"] == true)
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        enabled,
        vec![
            "native-lowering",
            "representation-inference",
            "trace-barrier-elision",
            "cell-demotion"
        ]
    );
    // The selection reaches the emitter, not just the echo.
    assert_eq!(
        on["wasm"].as_array().expect("WASM entries")[0]["codegenPlan"]["nativeLowering"]["enabled"],
        true
    );

    // `meta` carries the catalogue with nothing enabled, so the panel can
    // be built before a compile lands — the same rule dialects follow.
    let meta = serialise_meta();
    assert_eq!(
        meta["semanticOptimisations"]["passes"]
            .as_array()
            .unwrap()
            .len(),
        SemanticOptimisationPassId::all().len()
    );
    let groups: Vec<&str> = meta["semanticOptimisations"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap())
        .collect();
    assert_eq!(groups, vec!["native-tier", "all"]);
}

#[test]
fn wasm_view_exposes_the_native_tier_lowering_record() {
    use tcl_codegen::codegen::wasm::WasmCompileOptions;

    let result = run_pipeline("set a 1\nincr a\nputs $a\n", "tcl9.0");
    let off = serialise_result(&result)["wasm"].clone();
    let off_plan = &off.as_array().expect("WASM entries")[0]["codegenPlan"];
    assert_eq!(off_plan["nativeLowering"]["enabled"], false);

    let wasm = serialise_wasm_with_options(&result, WasmCompileOptions::hosted().native_tier());
    let plan = &wasm.as_array().expect("WASM entries")[0]["codegenPlan"];
    let native = &plan["nativeLowering"];
    assert_eq!(native["enabled"], true);
    let top = &native["functions"]["topLevel"];
    assert_eq!(top["status"], "lowered");
    let statements = top["statements"].as_array().expect("statement records");
    assert!(statements.iter().any(|statement| {
        statement["instruction"] == "execute-lowered"
            && statement["outcome"] == "native"
            && statement["cells"][0]["access"] == "write"
            && statement["cells"][0]["storage"] == "cell"
            && statement["cells"][0]["storageReason"] == "top-level-global"
            && statement["cells"][0]["barrier"] == "elided:no-trace-reaches-cell"
    }));
    assert!(statements.iter().any(|statement| {
        statement["instruction"] == "invoke" && statement["outcome"] == "native-intrinsic"
    }));
}

#[test]
fn wasm_view_exposes_typed_semantic_decline() {
    let result = run_pipeline("string length $value", "tcl8.6");
    let wasm = serialise_result(&result)["wasm"].clone();
    let plan = &wasm.as_array().expect("WASM entries")[0]["codegenPlan"];

    assert_eq!(plan["kind"], "general");
    assert_eq!(
        plan["semanticDecline"]["kind"],
        "backend-selection-declined"
    );
    assert_eq!(
        plan["semanticDecline"]["detailKind"],
        "no-viable-semantic-plan"
    );
    assert_eq!(plan["regionPlanStatus"], "available");
    assert_eq!(plan["regions"][0]["node"], json!([0]));
    assert_eq!(plan["regions"][0]["slowPath"]["kind"], "prebuilt-argv");
}

#[test]
fn wasm_view_serialises_multiple_mixed_regions_in_node_order() {
    let result = run_pipeline(
        "puts one\nset value 2\nif {$enabled} {puts enabled}\nputs two",
        "tcl9.0",
    );
    let wasm = serialise_result(&result)["wasm"].clone();
    let plan = &wasm.as_array().expect("WASM entries")[0]["codegenPlan"];
    let regions = plan["regions"].as_array().expect("mixed regions");

    assert_eq!(
        regions
            .iter()
            .map(|region| region["node"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!([0]),
            json!([1]),
            json!([2]),
            // The `if` body's invocation is a region nested under the
            // structured region's node.
            json!([2, 0, 0]),
            json!([3])
        ]
    );
    assert_eq!(regions[0]["selectedKind"], "generic-prebuilt-argv");
    assert_eq!(regions[1]["selectedKind"], "lowered");
    assert_eq!(regions[2]["selectedKind"], "structured");
    assert_eq!(regions[3]["selectedKind"], "generic-prebuilt-argv");
    assert_eq!(regions[4]["selectedKind"], "generic-prebuilt-argv");
    assert!(regions[1]["slowPath"].is_null());
    assert!(regions[2]["slowPath"].is_null());
}

/// The interprocedural view surfaces, as advice, the literal every call in
/// the file passes, and stops surfacing it when a dynamic dispatch reaches
/// the same procedure with a different literal.
#[test]
fn interproc_view_shows_what_every_call_passes_and_its_withdrawal() {
    const HELPER: &str = "proc helper {mode} {\n\
             if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n\
             }\nhelper prod\nhelper prod\n";
    let seeded = serialise_result(&run_pipeline(HELPER, "tcl8.6"))["interprocedural"].clone();
    let entry = seeded
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "::helper")
        .expect("::helper summarised");
    assert_eq!(entry["everyCallPasses"], json!(["mode = prod"]));

    let src = format!("{HELPER}set cmd helper\n$cmd dev\n");
    let withdrawn = serialise_result(&run_pipeline(&src, "tcl8.6"))["interprocedural"].clone();
    let entry = withdrawn
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "::helper")
        .expect("::helper summarised");
    assert_eq!(
        entry["everyCallPasses"],
        json!([]),
        "the `$cmd dev` dispatch also reaches helper, with a differing literal",
    );
}

#[test]
fn world_ssa_view_keeps_versions_transitions_and_typed_availability() {
    let data = serialise_result(&run_pipeline("puts hello\n", "tcl9.0"));
    let top = data["worldSsa"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == "::top")
        .expect("top-level world sidecar");
    assert!(
        top["availability"]["hasExecutableIr"].as_bool().unwrap(),
        "the availability payload must distinguish an absent graph from absent executable IR"
    );
    assert!(top["availability"]["kind"].is_string());
    assert!(
        !top["operations"].as_array().unwrap().is_empty(),
        "Explorer explicitly requests deep World SSA rather than the interactive no-graph path: {top}",
    );
    assert!(
        top["operations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|operation| {
                operation["location"]["domain"].is_string()
                    && operation["version"].is_number()
                    && operation["site"]["kind"].is_string()
            }),
        "world operations keep structured location, version, and CFG/site evidence",
    );
}

#[test]
fn world_ssa_decline_preserves_transition_policy_evidence() {
    let data = serialise_result(&run_pipeline(
        "interp create child\ninterp hide child puts\n",
        "tcl9.0",
    ));
    let top = data["worldSsa"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == "::top")
        .unwrap();
    assert_eq!(top["availability"]["kind"], "world-state-declined");
    let transition = top["invocations"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|invoke| invoke["transitions"].as_array().unwrap())
        .next()
        .expect("registry transition projection");
    assert!(transition["kind"].is_string());
    assert!(transition["commit"].is_string());
    assert!(transition["abruptTransfer"].is_string());
}

#[test]
fn serialise_result_includes_meta_and_ir() {
    let result = run_pipeline("set x 1", "tcl8.6");
    let value = serialise_result(&result);
    assert!(value.get("meta").is_some());
    let ir = &value["ir"];
    assert!(ir["topLevel"].is_array());
    assert!(ir["procedures"].is_object());
}

#[test]
fn durable_compiler_views_are_present_and_cover_extra_units() {
    let source = "oo::class create C { method m {} { set x 1 } }\n\
            set f [list apply {{} { set y 2 }}]\n";
    let result = run_pipeline(source, "tcl8.6");
    crate::coverage::assert_durable_field_inventory(&result.unit);
    let value = serialise_result(&result);
    for key in ["dominators", "sccp", "liveness", "semantic"] {
        assert!(
            value.get(key).is_some(),
            "missing durable view payload {key}"
        );
    }
    let semantic = value["semantic"].as_array().expect("semantic array");
    assert!(semantic.iter().any(|f| f["name"] == "::top"));
    assert!(semantic.iter().any(|f| f["name"] == "::C::m"));
    let meta = value["meta"]["views"].as_array().expect("view metadata");
    for id in ["dominators", "sccp", "liveness", "semantic"] {
        assert!(
            meta.iter().any(|v| v["id"] == id),
            "missing tab metadata {id}"
        );
    }
    let source_map = &value["sourceMap"];
    assert!(
        source_map["units"]
            .as_array()
            .unwrap()
            .iter()
            .any(|unit| { unit["name"] == "::C::m" && unit["baseOffset"].is_number() })
    );

    let dataflow = serialise_result(&run_pipeline(
        "proc f {name} { upvar 1 $name local }",
        "tcl8.6",
    ));
    let f = dataflow["dataflow"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == "::f")
        .expect("procedure data-flow function");
    assert_eq!(f["kind"], "procedure");
    assert!(f["memorySsa"]["operations"].is_array());
}

#[test]
fn ir_expands_if_clauses_and_collects_procs() {
    let result = run_pipeline(
        "proc f {x} { if {$x > 0} { puts hi } else { puts lo } }",
        "tcl8.6",
    );
    let ir = serialise_result(&result)["ir"].clone();
    let procedure = ir["procedures"]
        .as_object()
        .unwrap()
        .values()
        .find(|row| row["name"] == "::f")
        .unwrap();
    let body = &procedure["body"];
    let if_node = &body[0];
    assert_eq!(if_node["kind"], "IRIf");
    assert_eq!(if_node["colorClass"], "ir-control");
    // Two children: clause 1 and the else arm.
    let children = if_node["children"].as_array().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0]["label"], "clause 1: $x > 0");
    assert_eq!(children[1]["label"], "else");
}

#[test]
fn optimised_source_reflects_constant_fold() {
    let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
    let opt = serialise_result(&result)["optimisedSource"].clone();
    // The constant fold changes the source, so it is a non-null string.
    // SCCP proves `x` is `3` and its only use is propagated, so the
    // computed def couple-removes — the result is
    // `puts 3`, not `set x 3`.
    let s = opt.as_str().expect("optimised source string");
    assert!(s.contains("puts 3"), "{s:?}");
    assert!(!s.contains("set x"), "{s:?}");
}

#[test]
fn optimised_source_null_when_unchanged() {
    let result = run_pipeline("puts hello", "tcl8.6");
    let v = serialise_result(&result);
    assert_eq!(v["optimisedSource"], Value::Null);
    // The double-pipeline keys are also null when nothing changed.
    assert_eq!(v["irOptimised"], Value::Null);
    assert_eq!(v["cfgPreSsaOptimised"], Value::Null);
}

#[test]
fn optimised_ir_and_cfg_present_when_source_changes() {
    let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
    let v = serialise_result(&result);
    // The fold rewrites the source, so the optimised views populate and
    // their IR reflects the folded `set x 3`.
    assert!(v["irOptimised"]["topLevel"].is_array());
    assert!(v["cfgPreSsaOptimised"].is_array());
    assert!(!v["cfgPreSsaOptimised"].as_array().unwrap().is_empty());
}

#[test]
fn dead_stores_surface_from_o109() {
    // `set x 1` is overwritten by `set x 2` before any read — the
    // optimiser's O109 finding surfaces in cfgPostSsa.analysis, stats,
    // and as a deadStore callout (Rust has no standalone liveness pass).
    let result = run_pipeline("proc f {} { set x 1; set x 2; return $x }", "tcl8.6");
    let value = serialise_result(&result);

    let dead: Vec<&Value> = value["cfgPostSsa"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|fn_| fn_["analysis"]["deadStores"].as_array().unwrap())
        .collect();
    assert_eq!(dead.len(), 1, "one dead store");
    assert_eq!(dead[0]["variable"], "x");
    assert_eq!(dead[0]["version"], 1);

    assert_eq!(value["stats"]["deadStores"], 1);
    assert!(
        value["annotations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["kind"] == "deadStore"),
        "a deadStore callout is emitted"
    );
}

#[test]
fn no_dead_stores_when_value_is_read() {
    let result = run_pipeline("proc f {} { set x 1; return $x }", "tcl8.6");
    let value = serialise_result(&result);
    assert_eq!(value["stats"]["deadStores"], 0);
}

#[test]
fn structural_index_reports_boundaries_and_balance() {
    // Two complete commands, balanced brackets/braces.
    let result = run_pipeline("set x {a b c}\nputs [llength $x]", "tcl8.6");
    let si = serialise_result(&result)["structuralIndex"].clone();
    assert_eq!(si["scriptComplete"], true);
    assert_eq!(si["commandBoundaries"].as_array().unwrap().len(), 2);
    assert_eq!(si["brackets"]["unterminated"], 0);
    assert_eq!(si["braces"]["unterminated"], 0);
}

#[test]
fn structural_index_flags_incomplete_script() {
    // An unclosed brace leaves the script incomplete with an unterminated
    // inert span.
    let result = run_pipeline("proc f {} {\n  set x 1", "tcl8.6");
    let si = serialise_result(&result)["structuralIndex"].clone();
    assert_eq!(si["scriptComplete"], false);
    assert!(si["braces"]["unterminated"].as_i64().unwrap() >= 1);
}

#[test]
fn source_map_maps_lines_to_byte_ranges() {
    let result = run_pipeline("set x 1\nputs $x\n", "tcl8.6");
    let sm = serialise_result(&result)["sourceMap"].clone();
    assert_eq!(sm["lineCount"], 3); // two lines + the trailing empty line
    let lines = sm["lines"].as_array().unwrap();
    assert_eq!(lines[0]["start"], 0);
    assert_eq!(lines[0]["text"], "set x 1");
    // Line 1 starts right after the first newline.
    assert_eq!(lines[1]["start"], 8);
    assert_eq!(lines[1]["text"], "puts $x");
    // Byte length covers the whole source.
    assert_eq!(sm["byteLength"], 16);
}

#[test]
fn optimiser_passes_lists_every_pass_in_order() {
    // The Rust-native pass-pipeline view: all 9 passes in execution
    // order, each with the optimisations it produced.
    let result = run_pipeline("set x 1\nset y [expr {1 + 2}]\nputs $x$y", "tcl8.6");
    let passes = serialise_result(&result)["optimiserPasses"].clone();
    let arr = passes.as_array().unwrap();
    assert_eq!(arr.len(), 9, "all nine passes are listed");
    // Execution order is fixed (propagation first).
    assert_eq!(arr[0]["id"], "propagation");
    assert_eq!(arr[1]["id"], "branch_folding");
    // Each entry carries a label, count, and the optimisations array.
    for p in arr {
        assert!(p["label"].is_string());
        let opts = p["optimisations"].as_array().unwrap();
        assert_eq!(p["count"].as_u64().unwrap(), opts.len() as u64);
    }
    // Propagation folds the constant var-ref / interpolation here.
    let prop = &arr[0];
    assert!(prop["count"].as_u64().unwrap() >= 1);
}

#[test]
fn gvn_abstains_without_exact_common_invocation_provenance() {
    let result = run_pipeline(
        "set x [list 1 2 3]\nset a [llength $x]\nset b [llength $x]\nputs $a$b",
        "tcl8.6",
    );
    let gvn = serialise_result(&result)["gvn"].clone();
    let arr = gvn.as_array().unwrap();
    // The executable sidecar does not yet retain exact invocation nodes
    // for commands nested inside bracket substitutions. Production GVN
    // therefore fails closed instead of re-enabling its legacy textual
    // scanner and claiming an optimisation without common world facts.
    assert!(arr.is_empty());
}

#[test]
fn irules_flow_empty_for_plain_tcl() {
    let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
    assert_eq!(serialise_result(&result)["irulesFlow"], json!([]));
}

#[test]
fn event_order_empty_for_plain_tcl() {
    let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
    assert_eq!(serialise_result(&result)["eventOrder"], json!([]));
}

#[test]
fn event_order_orders_when_handlers_by_firing_order() {
    // Two HTTP_REQUEST handlers (one with explicit priority) plus
    // RULE_INIT and HTTP_RESPONSE. Verified byte-for-byte for this snippet.
    let src = "when RULE_INIT { set ::x 1 }\n\
                   when HTTP_REQUEST priority 100 { log local0. a }\n\
                   when HTTP_REQUEST { log local0. b }\n\
                   when HTTP_RESPONSE { log local0. c }";
    let result = run_pipeline(src, "f5-irules");
    let order = serialise_result(&result)["eventOrder"].clone();
    let rows = order.as_array().expect("eventOrder is an array");
    assert_eq!(rows.len(), 4);

    // RULE_INIT fires first (init), then HTTP_REQUEST (priority 100 before
    // the default 500), then the default HTTP_REQUEST, then HTTP_RESPONSE.
    assert_eq!(rows[0]["event"], "RULE_INIT");
    assert_eq!(rows[0]["multiplicity"], "init");
    assert_eq!(rows[1]["event"], "HTTP_REQUEST");
    assert_eq!(rows[1]["base_priority"], 100);
    assert_eq!(rows[1]["multiplicity"], "per_request");
    assert_eq!(rows[2]["event"], "HTTP_REQUEST");
    assert_eq!(rows[2]["base_priority"], 500);
    assert_eq!(rows[3]["event"], "HTTP_RESPONSE");
    // The range points at the `EVENT` token, not `when`.
    assert_eq!(rows[0]["range"]["startOffset"], 5);
}

#[test]
fn event_order_tie_break_increments_offset() {
    // Two handlers for the same event at the same (default) priority: the
    // second gets priority_offset 1.
    let src = "when HTTP_REQUEST { log local0. a }\n\
                   when HTTP_REQUEST { log local0. b }";
    let result = run_pipeline(src, "f5-irules");
    let order = serialise_result(&result)["eventOrder"].clone();
    let rows = order.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["priority_offset"], 0);
    assert_eq!(rows[1]["priority_offset"], 1);
}

#[test]
fn event_order_applies_outer_priority_until_it_changes() {
    let src = "priority 700\n\
                   when HTTP_REQUEST { log local0. first }\n\
                   priority 400\n\
                   when CLIENT_ACCEPTED { log local0. second }\n\
                   when HTTP_REQUEST { log local0. third }\n\
                   when HTTP_REQUEST priority 200 { log local0. inline }";
    let value = serialise_result(&run_pipeline(src, "f5-irules"));
    let rows = value["eventOrder"].as_array().unwrap();
    assert_eq!(rows.len(), 4, "repeated events remain distinct");
    let request: Vec<_> = rows
        .iter()
        .filter(|row| row["event"] == "HTTP_REQUEST")
        .map(|row| row["base_priority"].as_i64().unwrap())
        .collect();
    assert_eq!(request, [200, 400, 700]);
    assert_eq!(rows[0]["event"], "CLIENT_ACCEPTED");
    assert_eq!(rows[0]["base_priority"], 400);
}

#[test]
fn event_order_uses_rooted_normalised_top_level_handlers_only() {
    let src = "::when http_request { if {1} { :::when client_data {} } }";
    let result = run_pipeline(src, "f5-irules");
    let value = serialise_result(&result);
    let rows = value["eventOrder"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["event"], "HTTP_REQUEST");
}

#[test]
fn event_order_ignores_non_braced_event_bodies() {
    let src = "set payload {when CLIENT_DATA {}}\n\
                   set q \"when SERVER_DATA {}\"\n\
                   when CLIENT_DATA bare_body\n\
                   when SERVER_DATA \"quoted body\"\n\
                   when HTTP_REQUEST priority 100 {}\n\
                   when HTTP_REQUEST {}";
    let value = serialise_result(&run_pipeline(src, "f5-irules"));
    let rows = value["eventOrder"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["event"], "HTTP_REQUEST");
    assert_eq!(rows[0]["base_priority"], 100);
    assert_eq!(rows[1]["event"], "HTTP_REQUEST");
    assert_eq!(rows[1]["base_priority"], 500);
}

#[test]
fn dataflow_emits_nodes_and_summary() {
    let result = run_pipeline("set x 1\nset y [expr {$x + 1}]\nputs $y", "tcl8.6");
    let df = serialise_result(&result)["dataflow"].clone();
    assert!(df["functions"].is_array());
    let summary = &df["summary"];
    assert!(summary["totalDefs"].as_u64().unwrap() >= 2);
    assert_eq!(
        summary["functionCount"],
        df["functions"].as_array().unwrap().len()
    );
    let f0 = &df["functions"].as_array().unwrap()[0];
    assert!(f0["nodes"].is_array());
    assert!(f0["edges"].is_array());
    assert!(f0["aliases"].is_array());
}

#[test]
fn loops_detects_for_loop_with_depth() {
    // `0 < 10` is statically true on entry, so analysis rotates the loop
    // (FP-RBS-18): the entry guard `for_header` moves out of the natural
    // loop and the back-edge target — the natural-loop header — becomes the
    // `for_body` block, with the `for_step` re-check as the latch.
    let result = run_pipeline("for {set i 0} {$i < 10} {incr i} { puts $i }", "tcl8.6");
    let loops = serialise_result(&result)["loops"].clone();
    let top = &loops.as_array().unwrap()[0];
    let lps = top["loops"].as_array().unwrap();
    assert_eq!(lps.len(), 1, "one natural loop for a single for-loop");
    let lp = &lps[0];
    assert_eq!(lp["depth"], 1);
    let header = lp["header"].as_str().unwrap();
    assert!(
        header.contains("for_body") || header.contains("header"),
        "rotated for-loop header is the body block, got {header}"
    );
    assert!(!lp["latches"].as_array().unwrap().is_empty());
    assert_eq!(
        lp["blockCount"].as_u64().unwrap(),
        lp["blocks"].as_array().unwrap().len() as u64
    );
}

#[test]
fn loops_empty_for_straightline_code() {
    let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
    let loops = serialise_result(&result)["loops"].clone();
    assert!(
        loops.as_array().unwrap()[0]["loops"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn bounds_emits_per_function_shape() {
    let result = run_pipeline("set l [list a b c]\nlindex $l 0", "tcl8.6");
    let bounds = serialise_result(&result)["bounds"].clone();
    let top = &bounds.as_array().unwrap()[0];
    assert_eq!(top["name"], "::top");
    assert!(top["findings"].is_array());
    // The Rust interval analysis emits no divide-by-zero findings.
    assert_eq!(top["divzero"], json!([]));
}

/// The SCCP view carries each statement's value-transfer route and its
/// answer at the fixed point, beside the lattice.
#[test]
fn sccp_reports_each_statements_route_and_answer() {
    let result = run_pipeline(
        "proc p {} {\n    set n 1\n    incr n\n    set s [string range abc 0 1]\n    return $n\n}\n",
        "tcl8.6",
    );
    let sccp = serialise_result(&result)["sccp"].clone();
    let proc_view = sccp
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "::p")
        .expect("the procedure's view");
    let routes = proc_view["routes"].as_array().expect("routes");
    let incr = routes
        .iter()
        .find(|r| r["command"] == "incr")
        .expect("the incr route");
    assert_eq!(incr["route"], "direct cell-increment (registry)");
    assert_eq!(incr["answer"], "evaluated");
    assert_eq!(incr["range"]["startLine"], 2);
    let range = routes
        .iter()
        .find(|r| r["command"] == "string")
        .expect("the string range route");
    assert_eq!(range["route"], "direct string-range (registry)");
    assert_eq!(range["answer"], "evaluated");
}

/// The SCCP view lists what each statement stores on its completion paths:
/// a `lassign` over an array stops after its first store, and over a
/// place of a kind the analysis does not know it may fail at any of them,
/// which its transfer says by path. A statement that stores on its normal
/// path alone lists that one.
#[test]
fn sccp_reports_each_statements_completion_paths() {
    let result = run_pipeline(
        "proc p {} {\n    array set c {k keep}\n    set a old\n    \
             lassign {new second} a c\n}\n\
             proc q {c} {\n    set a old\n    if {$c} {set b 1}\n    lassign {x y} a b\n}\n",
        "tcl8.6",
    );
    let sccp = serialise_result(&result)["sccp"].clone();
    let paths = |name: &str, line: u64| -> Vec<(String, Vec<String>)> {
        let proc_view = sccp
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == name)
            .expect("the procedure's view");
        let route = proc_view["routes"]
            .as_array()
            .expect("routes")
            .iter()
            .find(|r| r["command"] == "lassign" && r["range"]["startLine"] == line)
            .expect("the lassign route");
        route["paths"]
            .as_array()
            .expect("paths")
            .iter()
            .map(|path| {
                (
                    path["completion"].as_str().unwrap().to_owned(),
                    path["stores"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|store| store.as_str().unwrap().to_owned())
                        .collect(),
                )
            })
            .collect()
    };
    assert_eq!(
        paths("::p", 3),
        [(
            "error after 1 store".to_owned(),
            vec!["write a = new".to_owned()]
        )],
        "the store before the array ran, the one at it did not"
    );
    assert_eq!(
        paths("::q", 8),
        [
            (
                "normal".to_owned(),
                vec!["write a = x".to_owned(), "write b = y".to_owned()]
            ),
            (
                "error".to_owned(),
                vec![
                    "bind a as scalar".to_owned(),
                    "may-bind b as scalar".to_owned()
                ]
            ),
        ]
    );
}

/// The SCCP view's route tally counts every family's entries once,
/// nested ones included: two fused `expr` statements are two
/// expression entries, and the nested `string length` one of them
/// evaluates is a direct entry.
#[test]
fn sccp_reports_the_route_tally() {
    let result = run_pipeline(
        "proc p {} {set r [expr {\"x\"}]; \
             set n [expr {[string length abcdef] * 2}]}\n",
        "tcl8.6",
    );
    let sccp = serialise_result(&result)["sccp"].clone();
    let proc_view = sccp
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "::p")
        .expect("the procedure's view");
    let tally = &proc_view["routeTally"];
    assert_eq!(tally["direct"], 1);
    assert_eq!(tally["expression"], 2);
    assert_eq!(tally["implementation"], 0);
}

/// The SCCP view carries each opaque case-list statement's selection:
/// program (4)'s `-glob` form selects its final `default`,
/// which the view names as such, against the one pattern span the
/// statement keeps — `baz`'s.
#[test]
fn sccp_reports_the_selection() {
    let source = "proc p {} {\n    set acc \"\"; append acc foo; append acc bar\n    \
                      switch -glob -- $acc {\n        baz     { puts never }\n        \
                      default { puts always }\n    }\n}\n";
    let result = run_pipeline(source, "tcl8.6");
    let sccp = serialise_result(&result)["sccp"].clone();
    let proc_view = sccp
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "::p")
        .expect("the procedure's view");
    let selections = proc_view["selections"].as_array().expect("selections");
    assert_eq!(selections.len(), 1, "{proc_view:#}");
    let selection = &selections[0];
    assert_eq!(selection["selected"], json!(["default"]));
    assert_eq!(selection["bodies"], json!(["default"]));
    assert_eq!(selection["writes"], json!([0]));
    assert_eq!(selection["range"]["startLine"], 2);
    let patterns = selection["armPatterns"].as_array().expect("arm patterns");
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0]["startLine"], 3);
    assert_eq!(patterns[0]["startCol"], 8);
}

/// The SCCP view states each branch fact's kind, and an arm of an opaque
/// `switch` no member runs the body of is a `selected` fact: no target,
/// `false`, its pattern as the condition and its own range.
#[test]
fn sccp_reports_the_unreached_arms() {
    let source = "proc p {} {\n    set acc \"\"; append acc foo; append acc bar\n    \
                      switch -glob -- $acc {\n        baz     { puts never }\n        \
                      default { puts always }\n    }\n}\n";
    let result = run_pipeline(source, "tcl8.6");
    let sccp = serialise_result(&result)["sccp"].clone();
    let proc_view = sccp
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "::p")
        .expect("the procedure's view");
    let branches = proc_view["constantBranches"].as_array().expect("branches");
    assert_eq!(branches.len(), 1, "{proc_view:#}");
    let arm = &branches[0];
    assert_eq!(arm["kind"], "selected");
    assert_eq!(arm["condition"], "baz");
    assert_eq!(arm["value"], false);
    assert_eq!(arm["takenTarget"], "");
    assert_eq!(arm["range"]["startLine"], 3);
    assert_eq!(arm["range"]["startCol"], 8);
}

/// Each value the SCCP view lists carries the folded type the
/// evaluation that produced it states: a computed `string length` is an
/// int the route constructed, `list` a list, a copy shares its source's,
/// and a φ keeps what both arms state alike. A literal states nothing,
/// and neither does a φ over a literal arm.
#[test]
fn sccp_reports_folded_types() {
    let result = run_pipeline(
        "proc p {c} {set s abcdef; set n [string length $s]; set l [list a b]; \
             set m $n; if {$c} {set k [string length $s]} else {set k [llength $l]}; \
             if {$c} {set j 5} else {set j [llength $l]}; return $m$k$j}\n",
        "tcl8.6",
    );
    let sccp = serialise_result(&result)["sccp"].clone();
    let proc_view = sccp
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "::p")
        .expect("the procedure's view");
    let type_of = |variable: &str, version: u64| {
        proc_view["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["variable"] == variable && value["version"] == version)
            .unwrap_or_else(|| panic!("{variable}#{version}: {proc_view:#}"))
            .get("type")
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    assert_eq!(type_of("s", 1), None, "a literal states no type");
    assert_eq!(type_of("n", 1).as_deref(), Some("int (constructed)"));
    assert_eq!(type_of("l", 1).as_deref(), Some("list (constructed)"));
    assert_eq!(type_of("m", 1).as_deref(), Some("int (constructed)"));
    // The φ versions are numbered first: `k#1` joins `k#2` and `k#3`.
    assert_eq!(type_of("k", 1).as_deref(), Some("int (constructed)"));
    assert_eq!(type_of("j", 2), None, "a literal arm");
    assert_eq!(type_of("j", 3).as_deref(), Some("int (constructed)"));
    assert_eq!(type_of("j", 1), None, "one arm is a literal");
}

#[test]
fn cfg_post_ssa_has_phis_uses_defs_and_analysis() {
    let result = run_pipeline("set x 1\nset y [expr {$x + 1}]\nputs $y", "tcl8.6");
    let post = serialise_result(&result)["cfgPostSsa"].clone();
    let top = &post.as_array().unwrap()[0];
    assert_eq!(top["name"], "::top");
    assert!(top["analysis"]["constantBranches"].is_array());
    assert!(top["analysis"]["deadStores"].is_array());
    assert!(top["analysis"]["inferredTypes"].is_object());
    let block = &top["blocks"].as_array().unwrap()[0];
    assert!(block["phis"].is_array());
    assert!(block["isEntry"].as_bool().unwrap());
    // A statement carries SSA uses/defs maps.
    let stmt = &block["statements"].as_array().unwrap()[0];
    assert!(stmt["uses"].is_object());
    assert!(stmt["defs"].is_object());
}

#[test]
fn cfg_post_ssa_optimised_present_when_source_changes() {
    let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
    let v = serialise_result(&result);
    assert!(v["cfgPostSsaOptimised"].is_array());
}

#[test]
fn stats_reports_function_and_block_counts() {
    let result = run_pipeline("proc f {} { return 1 }\nf", "tcl8.6");
    let stats = serialise_result(&result)["stats"].clone();
    assert_eq!(stats["procedures"], 1);
    // top-level + ::f.
    assert_eq!(stats["functions"], 2);
    assert!(stats["blocks"].as_u64().unwrap() >= 1);
    assert!(stats["rewrites"].is_number());
    assert!(stats["shimmerWarnings"].is_number());
}

#[test]
fn annotations_collect_callouts_and_group_by_line() {
    let result = run_pipeline(
        "set a 5\nset b [expr {$a + 1}]\nset c [expr {$a + 1}]\nputs $b$c",
        "tcl8.6",
    );
    let v = serialise_result(&result);
    let anns = v["annotations"].as_array().unwrap();
    assert!(!anns.is_empty(), "expected source callouts");
    for a in anns {
        assert!(a["range"]["startOffset"].is_number());
        assert!(a["label"].is_string());
        assert!(a["kind"].is_string());
    }
    // byLine groups annotation indices under string line keys.
    let by_line = v["annotationsByLine"].as_object().unwrap();
    let total: usize = by_line.values().map(|v| v.as_array().unwrap().len()).sum();
    assert_eq!(total, anns.len());
}

#[test]
fn asm_emits_structured_instructions_for_top_level() {
    let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
    let asm = serialise_result(&result)["asm"].clone();
    let top = &asm.as_array().unwrap()[0];
    assert_eq!(top["name"], "::top");
    assert_eq!(top["kind"], "top");
    assert!(top["text"].as_str().unwrap().contains("ByteCode"));
    let ops: Vec<&str> = top["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "instr")
        .map(|r| r["op"].as_str().unwrap())
        .collect();
    // The store and the invoke are present; each instr has a fullText.
    assert!(ops.contains(&"storeStk"));
    assert!(ops.contains(&"invokeStk1"));
    assert!(ops.contains(&"done"));
    let first_instr = top["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "instr")
        .unwrap();
    assert!(
        first_instr["fullText"]
            .as_str()
            .unwrap()
            .starts_with("push1")
    );
}

#[test]
fn asm_resolves_jump_targets() {
    // A conditional produces a jump whose target resolves to a row idx.
    let result = run_pipeline("if {$x} { puts hi }", "tcl8.6");
    let asm = serialise_result(&result)["asm"].clone();
    let top = &asm.as_array().unwrap()[0];
    let has_jump = top["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["jumpTarget"].is_object());
    assert!(has_jump, "expected at least one resolved jump target");
}

#[test]
fn taint_tracking_marks_exec_tainted_vars() {
    let result = run_pipeline("set x [exec ls]\nset y $x", "tcl8.6");
    let tracking = serialise_result(&result)["taintTracking"].clone();
    let top = &tracking.as_array().unwrap()[0];
    let vars: Vec<&str> = top["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["variable"].as_str().unwrap())
        .collect();
    assert!(vars.contains(&"x"));
    assert!(vars.contains(&"y"));
    assert_eq!(top["entries"][0]["taint"], "tainted");
}

#[test]
fn taint_reports_eval_sink() {
    let result = run_pipeline("set x [gets stdin]\neval $x", "tcl8.6");
    let taint = serialise_result(&result)["taintWarnings"].clone();
    let arr = taint.as_array().unwrap();
    let t = arr
        .iter()
        .find(|w| w["code"] == "T100")
        .expect("T100 warning");
    assert_eq!(t["variable"], "x");
    assert_eq!(t["sinkCommand"], "eval");
    assert_eq!(t["severity"], "error");
}

#[test]
fn shimmer_reports_intrep_conversion() {
    let result = run_pipeline("set x [list 1 2 3]\nincr x", "tcl8.6");
    let shimmer = serialise_result(&result)["shimmer"].clone();
    let arr = shimmer.as_array().unwrap();
    let s100 = arr
        .iter()
        .find(|w| w["code"] == "S100")
        .expect("S100 warning");
    assert_eq!(s100["variable"], "x");
    assert_eq!(s100["fromType"], "list");
    assert_eq!(s100["toType"], "int");
    assert_eq!(s100["command"], "incr");
    assert_eq!(s100["severity"], "warning");
}

#[test]
fn optimisations_reports_constant_fold_with_range() {
    // A by-name read (`[set x]`) keeps the def alive, so the O101
    // constant-fold survives as its own reported rewrite rather than
    // being superseded by the dead-store coupling.
    let result = run_pipeline("set x [expr {1 + 2}]\nputs [set x]", "tcl8.6");
    let opts = serialise_result(&result)["optimisations"].clone();
    let arr = opts.as_array().unwrap();
    assert!(!arr.is_empty(), "expected at least one optimisation");
    // Every entry carries the contract fields.
    for o in arr {
        assert!(o["code"].is_string());
        assert!(o["message"].is_string());
        assert!(o["replacement"].is_string());
        assert!(o["range"]["startOffset"].is_number());
    }
    // The constant-fold rewrite is present.
    assert!(arr.iter().any(|o| o["code"] == "O101"));
}

#[test]
fn rendered_properties_flags_slash_and_dash() {
    let result = run_pipeline("set p /var/log\nset u -flag", "tcl8.6");
    let rp = serialise_result(&result)["renderedProperties"].clone();
    let entries = rp.as_array().unwrap()[0]["entries"].as_array().unwrap();
    let p = entries.iter().find(|e| e["variable"] == "p").unwrap();
    assert!(
        p["may"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "HAS_FORWARD_SLASH")
    );
    let u = entries.iter().find(|e| e["variable"] == "u").unwrap();
    assert!(
        u["must"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "STARTS_WITH_DASH")
    );
}

#[test]
fn intervals_reports_bounded_loop_counter() {
    let result = run_pipeline("for {set i 0} {$i < 10} {incr i} { puts $i }", "tcl8.6");
    let intervals = serialise_result(&result)["intervals"].clone();
    let top = &intervals.as_array().unwrap()[0];
    assert_eq!(top["name"], "::top");
    let entries = top["entries"].as_array().unwrap();
    // At least one bounded entry for `i` with a concrete lower bound.
    assert!(
        entries
            .iter()
            .any(|e| e["variable"] == "i" && e["lo"].is_number())
    );
}

#[test]
fn segments_reports_words_with_shape_flags() {
    let segs = serialise_segments("string length \"hi\"", LexerConfig::default());
    let arr = segs.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let cmd = &arr[0];
    assert_eq!(cmd["name"], "string");
    assert_eq!(cmd["subcommand"], Value::Null);
    let words = cmd["words"].as_array().unwrap();
    assert_eq!(words.len(), 3);
    assert_eq!(words[0]["text"], "string");
    assert_eq!(words[2]["quoted"], true);
    assert_eq!(words[2]["braced"], false);
}

#[test]
fn segments_marks_braced_words() {
    let segs = serialise_segments("if {$x} { puts hi }", LexerConfig::default());
    let words = segs[0]["words"].as_array().unwrap();
    // `{$x}` is a braced word.
    assert_eq!(words[1]["braced"], true);
    assert_eq!(words[1]["quoted"], false);
}

#[test]
fn types_reports_known_int_for_constant_assignments() {
    let result = run_pipeline("set x 1\nset y 2", "tcl8.6");
    let types = serialise_result(&result)["types"].clone();
    let top = types
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "::top")
        .unwrap();
    let x = top["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["variable"] == "x")
        .unwrap();
    assert_eq!(x["type"], "int");
    assert_eq!(x["kind"], "known");
}

#[test]
fn interproc_summarises_procs_with_arity_and_return_shape() {
    let result = run_pipeline("proc id {x} { return $x }\nid 5", "tcl8.6");
    let interproc = serialise_result(&result)["interprocedural"].clone();
    let add = interproc
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "::id")
        .expect("::id summary present");
    assert_eq!(add["arity"], "1..1");
    assert_eq!(add["returnShape"], "passthrough(x)");
    assert!(add["pure"].is_boolean());
    assert!(add["calls"].is_array());
}

/// The Unit Scope view must show *why* the interprocedural seed fired:
/// the registry-declared boundaries the file crosses, whether a
/// cross-file view was supplied, and the per-position verdict.
#[test]
fn unit_scope_reports_uniform_literals_and_no_boundary() {
    let result = run_pipeline(
        "proc helper {mode} { if {$mode eq \"prod\"} { set r 1 } }\nhelper prod\nhelper prod\n",
        "tcl8.6",
    );
    let scope = serialise_result(&result)["unitScope"].clone();
    assert_eq!(scope["boundaries"].as_array().unwrap().len(), 0);
    assert_eq!(scope["hasCrossFileEvidence"], false);
    assert!(
        scope["seeding"].as_str().unwrap().starts_with("allowed"),
        "{scope:?}"
    );
    let helper = scope["callees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "::helper")
        .expect("::helper evidence present");
    assert_eq!(
        helper["positions"][0]["verdict"],
        "uniform literal \"prod\""
    );
}

/// The same file with a `package provide` crosses a registry-declared
/// boundary, so the view must report the boundary and the declined seed.
#[test]
fn unit_scope_reports_a_registry_declared_boundary() {
    let result = run_pipeline(
        "package provide mylib 1.0\nproc helper {mode} { if {$mode eq \"prod\"} { set r 1 } }\nhelper prod\n",
        "tcl8.6",
    );
    let scope = serialise_result(&result)["unitScope"].clone();
    assert_eq!(
        scope["boundaries"].as_array().unwrap(),
        &vec![Value::String("PROVIDES_PACKAGE".to_owned())]
    );
    assert!(
        scope["seeding"].as_str().unwrap().starts_with("declined"),
        "{scope:?}"
    );
}

#[test]
fn cfg_pre_ssa_has_entry_block_and_branch_terminator() {
    let result = run_pipeline("if {$x} { puts a } else { puts b }", "tcl8.6");
    let cfg = serialise_result(&result)["cfgPreSsa"].clone();
    let top = &cfg[0];
    assert_eq!(top["name"], "::top");
    assert_eq!(top["entry"], "entry_1");
    let entry = top["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "entry_1")
        .unwrap();
    assert_eq!(entry["isEntry"], true);
    // The entry block branches on the `if` condition.
    assert_eq!(entry["terminator"]["type"], "branch");
    // Edges carry routing lanes from cfg_layout.
    assert!(
        top["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["lane"].is_u64())
    );
}

#[test]
fn ir_widens_braced_condition_range_to_closer() {
    // `{$x}` opens at the `{`; the range must cover the closing `}`.
    let src = "if {$x} { puts 1 }";
    let result = run_pipeline(src, "tcl8.6");
    let ir = serialise_result(&result)["ir"].clone();
    let cond_range = &ir["topLevel"][0]["children"][0]["range"];
    let start = usize::try_from(cond_range["startOffset"].as_u64().unwrap()).unwrap();
    let end = usize::try_from(cond_range["endOffset"].as_u64().unwrap()).unwrap();
    assert_eq!(&src[start..end], "{$x}");
}
