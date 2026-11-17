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
use crate::tools::dispatch;

fn check(source: &str, dialect: &str) -> Value {
    dispatch(
        "spectcl_check",
        &json!({ "source": source, "dialect": dialect }),
    )
    .expect("spectcl_check tool")
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

/// A pack whose every word the loader knows: no notices, the declared
/// fields show up as set, and the hook is reported with its family.
const VALID: &str = r"
speclib mylib 2.1 {
    command mylib::with_var {
        arity 2..3
        arg 0 -role VarWrite
        arg 1 -role Body
        return_type String
        arg_role_resolver -inputs {nwords} {ctx} {
            if {[dict get $ctx nwords] >= 2} {
                role 1 Body
            }
        }
        arg_role_resolver_roles Body
    }
}
";

#[test]
fn a_valid_pack_reports_its_commands_and_no_notices() {
    let result = check(VALID, "tcl9.0");
    assert_eq!(result["pack"], "mylib", "{result}");
    assert_eq!(result["dsl_version"], "2.1");
    assert_eq!(result["dialect"], "tcl9.0");
    assert_eq!(result["notices"], json!([]), "{result}");
    assert_eq!(result["summary"]["commands"], 1);
    assert_eq!(result["collisions"], json!([]));

    let command = &result["commands"][0];
    assert_eq!(command["name"], "mylib::with_var");
    let set = strings(&command["fields_set"]);
    for field in [
        "arity",
        "arg_roles",
        "return_type",
        "arg_role_resolver",
        "arg_role_resolver_roles",
    ] {
        assert!(set.contains(&field), "{field} missing from {command}");
    }
    // A field the pack never mentioned stays at its default and is absent.
    assert!(!set.contains(&"traits"), "{command}");
}

/// The one hook declares `nwords` and reads it out of `ctx`: every input
/// it named is carried by the shape key, so the engine caches it.
#[test]
fn a_shape_only_resolver_body_is_shape_cacheable() {
    let result = check(VALID, "tcl9.0");
    let hook = &result["commands"][0]["hooks"][0];
    assert_eq!(hook["family"], "arg_role_resolver", "{hook}");
    assert_eq!(hook["owner"], "command");
    assert_eq!(hook["source"], "body");
    assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
    assert_eq!(hook["declaration_conflict"], Value::Null, "{hook}");
    assert_eq!(hook["verbs"], json!(["role"]));
    assert_eq!(result["summary"]["uncacheable_hooks"], 0);
    assert_eq!(result["summary"]["declaration_conflicts"], 0);
    assert_eq!(
        strings(&result["commands"][0]["hook_families"]),
        vec!["arg_role_resolver"]
    );
}

#[test]
fn script_timing_resolver_is_reported_by_its_schema_family() {
    let source = r"
speclib timed 1.2 {
    command timed::schedule {
        arity 1
        arg 0 -role Body
        script_timing_resolver {words ctx} { timing 0 Deferred }
    }
}
";
    let result = check(source, "tcl9.0");
    assert_eq!(result["notices"], json!([]), "{result}");
    assert_eq!(
        strings(&result["commands"][0]["hook_families"]),
        vec!["script_timing_resolver"]
    );
    assert_eq!(
        result["commands"][0]["hooks"][0]["family"],
        "script_timing_resolver"
    );
}

/// A pack-authored `state_transitions` resolver body is a hook like any
/// other family's: reported under the family's field spelling, with the
/// two verbs its sandbox defines and what its silence means.
#[test]
fn a_state_transition_resolver_is_reported_as_a_hook() {
    let source = r"
speclib linked 2.1 {
    command linked::alias {
        arity 2
        state_transitions {
            argument_shape Positional
            resolver {words ctx} { alias 1 0 }
        }
    }
}
";
    let result = check(source, "tcl9.0");
    assert_eq!(result["notices"], json!([]), "{result}");
    assert_eq!(
        strings(&result["commands"][0]["hook_families"]),
        vec!["state_transitions.resolver"]
    );
    let hook = &result["commands"][0]["hooks"][0];
    assert_eq!(hook["family"], "state_transitions.resolver", "{hook}");
    assert_eq!(hook["field"], "state_transitions.resolver", "{hook}");
    assert_eq!(hook["source"], "body", "{hook}");
    assert_eq!(hook["verbs"], json!(["alias", "namespace-variable"]));
    assert_eq!(hook["silence_means"], "no transitions");
}

/// A body that declares nothing may read anything, and is never cached —
/// the safe default, and the reason a pack pays 28 µs per call site until
/// it opts in. The keys it reads are still reported.
#[test]
fn an_undeclared_body_is_uncacheable_whatever_it_reads() {
    let source = r"
speclib mylib 1.0 {
    command mylib::guarded {
        arity 1
        context_gate {words ctx} {
            if {![dict get $ctx in-event-body]} return
            reject {mylib::guarded is only valid in an event body}
        }
    }
}
";
    let result = check(source, "tcl9.0");
    let hook = &result["commands"][0]["hooks"][0];
    assert_eq!(hook["family"], "context_gate", "{hook}");
    assert_eq!(hook["ctx_keys"], json!(["in-event-body"]), "{hook}");
    assert_eq!(hook["shape_cacheable"], json!(false), "{hook}");
    assert!(
        hook["cache_reason"]
            .as_str()
            .unwrap_or_default()
            .contains("-inputs"),
        "{hook}"
    );
    // Nothing was promised, so nothing is contradicted.
    assert_eq!(hook["declaration_conflict"], Value::Null, "{hook}");
    assert_eq!(hook["silence_means"], "the call is allowed");
    assert_eq!(result["summary"]["uncacheable_hooks"], 1);
}

/// `in-event-body` and `tcl-version` are declared shape-only inputs,
/// retained by `ShapeKey`, so declaring and reading them is cacheable.
#[test]
fn the_shape_key_extends_past_the_words() {
    let source = r"
speclib mylib 1.0 {
    command mylib::guarded {
        arity 1
        context_gate -inputs {in-event-body tcl-version} {ctx} {
            if {[dict get $ctx in-event-body]} return
            if {[dict get $ctx tcl-version] eq {8.4}} return
            reject {mylib::guarded is only valid in an event body}
        }
    }
}
";
    let result = check(source, "tcl9.0");
    let hook = &result["commands"][0]["hooks"][0];
    assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
    assert_eq!(hook["declaration_conflict"], Value::Null, "{hook}");
    assert_eq!(result["summary"]["uncacheable_hooks"], 0);
}

/// `dialect` is not a declared shape-only input. Reading it under a
/// `nwords` declaration is a conflict, even though `ShapeKey` retains the
/// call's dialect independently to keep cache entries dialect-specific.
#[test]
fn a_body_reading_past_its_declaration_is_flagged_as_a_conflict() {
    let source = r"
speclib mylib 1.0 {
    command mylib::guarded {
        arity 1
        context_gate -inputs {nwords} {ctx} {
            if {[dict get $ctx dialect] eq {irules}} return
            reject {mylib::guarded is iRules-only}
        }
    }
}
";
    let result = check(source, "tcl9.0");
    let hook = &result["commands"][0]["hooks"][0];
    // The engine caches it — saying otherwise would describe an engine
    // that does not exist.
    assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
    let conflict = hook["declaration_conflict"].as_str().unwrap_or_default();
    assert!(conflict.contains("dialect"), "{hook}");
    assert_eq!(result["summary"]["declaration_conflicts"], 1, "{result}");
}

/// A declared implementation reading a target's incoming value that the
/// structural plan never named as one of its own targets: almost always
/// a typo for `arg N exact`, or a `stores -targets` row an edit forgot
/// to update.
#[test]
fn an_evaluator_reading_an_undeclared_target_is_flagged() {
    let source = r"
speclib probe 2.2 {
    command probe::bad_input {
        arity 1..3
        semantics {
            stores -targets {1} -outcome write
        }
        evaluate -implementation probe.bad_input.v1 -host bounded_tcl {
            inputs {arg 0 exact target 2 incoming}
            budget {-commands 100}
            body {a b} { write 1 $a }
        }
    }
}
";
    let result = check(source, "tcl9.0");
    assert_eq!(result["notices"], json!([]), "{result}");
    let hook = &result["commands"][0]["hooks"][0];
    let findings = strings(&hook["evaluate_findings"]);
    assert_eq!(findings.len(), 1, "{hook}");
    assert!(
        findings[0].contains("target 2 incoming") && findings[0].contains("stores"),
        "{hook}"
    );
    assert_eq!(result["summary"]["evaluate_findings"], 1, "{result}");
}

/// A body that answers something but never names one of its own declared
/// targets: rule 3 declines the whole answer, so every fact the body did
/// establish for the targets it did name is lost with it.
#[test]
fn an_evaluator_silent_on_a_declared_target_is_flagged() {
    let source = r"
speclib probe 2.2 {
    command probe::partial_split {
        arity 1
        semantics {
            stores -targets {1 2} -outcome write_or_preserve
        }
        evaluate -implementation probe.partial_split.v1 -host bounded_tcl {
            inputs {arg 0 exact}
            body {value} { write 1 $value }
        }
    }
}
";
    let result = check(source, "tcl9.0");
    assert_eq!(result["notices"], json!([]), "{result}");
    let hook = &result["commands"][0]["hooks"][0];
    let findings = strings(&hook["evaluate_findings"]);
    assert_eq!(findings.len(), 1, "{hook}");
    assert!(
        findings[0].contains("target 2") && findings[0].contains("silence"),
        "{hook}"
    );
    assert_eq!(result["summary"]["evaluate_findings"], 1, "{result}");
}

/// A `write` naming an index outside the structural plan's declared
/// targets: it raises at query time, a decline the pack can avoid by
/// fixing the typo at load time instead.
#[test]
fn a_write_naming_a_non_target_is_flagged() {
    let source = r"
speclib probe 2.2 {
    command probe::stray_write {
        arity 1
        semantics {
            stores -targets {1} -outcome write
        }
        evaluate -implementation probe.stray_write.v1 -host bounded_tcl {
            inputs {arg 0 exact}
            body {value} { write 1 $value; write 2 $value }
        }
    }
}
";
    let result = check(source, "tcl9.0");
    assert_eq!(result["notices"], json!([]), "{result}");
    let hook = &result["commands"][0]["hooks"][0];
    let findings = strings(&hook["evaluate_findings"]);
    assert_eq!(findings.len(), 1, "{hook}");
    assert!(
        findings[0].contains("target 2") && findings[0].contains("outside"),
        "{hook}"
    );
    assert_eq!(result["summary"]["evaluate_findings"], 1, "{result}");
}

/// The other side of the previous three: a declaration whose inputs,
/// targets, and body all agree raises none of the three findings, and a
/// hook of a different family always reads `[]` regardless of what its
/// body does.
#[test]
fn a_consistent_declared_implementation_has_no_evaluate_findings() {
    let source = r#"
speclib probe 2.2 {
    command tenant::label {
        arity 1
        evaluate -implementation tenant.label.v1 -host bounded_tcl {
            inputs {arg 0 exact}
            body {name} { fold [string cat "tenant:" $name] }
        }
    }
    command probe::split3 {
        arity 4
        semantics {
            stores -targets {1 2 3} -outcome write_or_preserve
        }
        evaluate -implementation probe.split3.v1 -host bounded_tcl {
            inputs {arg 0 exact target 1 incoming}
            body {sep incoming} {
                write 1 $incoming
                preserve 2
                preserve 3
            }
        }
    }
    command mylib::with_var {
        arity 2
        const_fold {words ctx} { fold ok }
    }
}
"#;
    let result = check(source, "tcl9.0");
    assert_eq!(result["notices"], json!([]), "{result}");
    for command in result["commands"].as_array().expect("commands") {
        for hook in command["hooks"].as_array().expect("hooks") {
            assert_eq!(
                hook["evaluate_findings"],
                json!([]),
                "{}: {hook}",
                command["name"]
            );
        }
    }
    assert_eq!(result["summary"]["evaluate_findings"], 0, "{result}");
}

/// A `-native` hook has no VM body, so cacheability does not apply to it.
#[test]
fn a_native_hook_is_not_a_cacheability_question() {
    let source = r"
speclib mylib 1.0 {
    command mylib::native {
        arity 1
        const_fold -native Concat
    }
}
";
    let result = check(source, "tcl9.0");
    let hook = &result["commands"][0]["hooks"][0];
    assert_eq!(hook["source"], "native", "{hook}");
    assert_eq!(hook["detail"], "Concat");
    assert_eq!(hook["shape_cacheable"], Value::Null, "{hook}");
    assert_eq!(result["summary"]["uncacheable_hooks"], 0);
}

/// Unknown words are dropped with a notice each, and the rest of the
/// command still loads — the tolerance rule, made visible.
#[test]
fn unknown_words_surface_as_notices_without_losing_the_command() {
    let source = r"
speclib mylib 1.0 {
    command mylib::typo {
        arity 1..2
        traits {NOT_PROC_FACTORY WOBBLY_TRAIT}
        frobnicate yes
        return_type String
    }
}
";
    let result = check(source, "tcl9.0");
    let notices = result["notices"].as_array().expect("notices");
    assert!(notices.len() >= 2, "{result}");
    for notice in notices {
        assert_eq!(notice["context"], "command mylib::typo", "{notice}");
        assert!(notice["line"].as_u64().unwrap_or(0) > 0, "{notice}");
        assert!(
            !notice["reason"].as_str().unwrap_or_default().is_empty(),
            "{notice}"
        );
    }
    let reasons: Vec<&str> = notices
        .iter()
        .filter_map(|n| n["reason"].as_str())
        .collect();
    assert!(
        reasons.iter().any(|r| r.contains("WOBBLY_TRAIT")),
        "{reasons:?}"
    );
    assert!(
        reasons.iter().any(|r| r.contains("frobnicate")),
        "{reasons:?}"
    );

    // The command survived, with its known fields intact.
    let command = &result["commands"][0];
    assert_eq!(command["name"], "mylib::typo");
    let set = strings(&command["fields_set"]);
    assert!(set.contains(&"arity"), "{command}");
    assert!(set.contains(&"return_type"), "{command}");
    assert!(set.contains(&"traits"), "{command}");
}

#[test]
fn contradictory_option_semantics_surface_as_degradation_notices() {
    let source = r"
speclib mylib 1.2 {
    command mylib::bad {
        option -timing -takes value -role Value -script-timing Deferred
        option -input -takes script -role Body -script-timing SameInvocation \
            -callback-taint-inputs {%P}
        option -scope -takes value -role Value -variable-scope Global
        option -taint -takes variable -role VarRead -taints-var-write
    }
}
";
    let result = check(source, "tcl9.0");
    let reasons: Vec<&str> = result["notices"]
        .as_array()
        .expect("notices")
        .iter()
        .filter_map(|notice| notice["reason"].as_str())
        .collect();
    for fragment in [
        "non-executable value",
        "not a deferred executable",
        "non-variable value",
        "without a VarWrite role",
    ] {
        assert!(
            reasons.iter().any(|reason| reason.contains(fragment)),
            "missing `{fragment}`: {result}"
        );
    }
    assert_eq!(result["summary"]["notices"], 4, "{result}");
    assert_eq!(result["summary"]["commands"], 1, "{result}");
}

/// Redeclaring a shipped name without `-override` is the silent-no-op
/// case, and the report says so.
#[test]
fn a_shipped_name_collides_and_the_pack_spec_loses() {
    let source = r"
speclib mylib 1.0 {
    command lsort {
        arity 1..
    }
}
";
    let result = check(source, "tcl9.0");
    let collision = &result["collisions"][0];
    assert_eq!(collision["command"], "lsort", "{result}");
    assert_eq!(collision["dialect"], "tcl9.0");
    assert_eq!(collision["override"], json!(false));
    assert_eq!(collision["effect"], "shipped-spec-wins");
    assert!(
        collision["message"]
            .as_str()
            .unwrap_or_default()
            .contains("-override"),
        "{collision}"
    );
    assert_eq!(result["summary"]["shadowed_commands"], 1);
}

/// The same collision with `-override` is a deliberate replacement, not a
/// dead declaration.
#[test]
fn an_override_collision_says_the_pack_spec_wins() {
    let source = r"
speclib mylib 1.0 {
    command lsort -override {
        arity 1..
    }
}
";
    let result = check(source, "tcl9.0");
    let collision = &result["collisions"][0];
    assert_eq!(collision["effect"], "pack-spec-wins", "{result}");
    assert_eq!(collision["override"], json!(true));
    assert_eq!(result["summary"]["shadowed_commands"], 0);
    assert_eq!(result["summary"]["collisions"], 1);
}

/// A dialect-only command collides under its own dialect and not under
/// plain Tcl — the collision check is per target dialect, not global.
#[test]
fn collisions_are_scoped_to_the_target_dialect() {
    let source = r"
speclib mylib 1.0 {
    command HTTP::uri {
        arity 0..1
    }
}
";
    let irules = check(source, "f5-irules");
    assert_eq!(irules["summary"]["collisions"], 1, "{irules}");
    let plain = check(source, "tcl9.0");
    assert_eq!(plain["summary"]["collisions"], 0, "{plain}");
}

/// Subcommands are reported with their own set-field lists.
#[test]
fn subcommands_report_their_own_fields() {
    let source = r"
speclib mylib 1.0 {
    command mylib::ensemble {
        arity 1..
        subcommand indices {
            arity 1
            return_type List
        }
    }
}
";
    let result = check(source, "tcl9.0");
    let sub = &result["commands"][0]["subcommands"][0];
    assert_eq!(sub["name"], "indices", "{result}");
    let set = strings(&sub["fields_set"]);
    assert!(set.contains(&"arity"), "{sub}");
    assert!(set.contains(&"return_type"), "{sub}");
}

#[test]
fn hook_context_references_use_the_selected_original_c_and_jim_grammar() {
    let c = tcl_lexer::LexerConfig::for_profile(Some(crate::environment::profile_for_dialect(
        "tcl9.0",
    )));
    let jim = tcl_lexer::LexerConfig::for_profile(Some(crate::environment::profile_for_dialect(
        "jimtcl",
    )));
    assert!(ctx_reads("$ctxé", c).unattributed);
    assert!(!ctx_reads("$ctxé", jim).unattributed);
    for config in [c, jim] {
        assert!(!ctx_reads("$ctxfoo ${ctxé}", config).unattributed);
        assert!(ctx_reads("$ctx(key)", config).unattributed);
        assert!(ctx_reads("${ctx}", config).unattributed);
        let scan = ctx_reads("dict get ${ctx} nwords", config);
        assert_eq!(scan.keys, BTreeSet::from(["nwords".to_owned()]));
        assert!(!scan.unattributed);
    }
}

#[test]
fn hook_context_report_threads_the_requested_target_grammar() {
    let source = r"speclib contextgrammar 1.0 {
            command contextgrammar::sample {
                arity 1
                arg_role_resolver -inputs {nwords} {ctx} {
                    set sample $ctxé
                    role 0 Body
                }
            }
        }";
    let c = check(source, "tcl9.0");
    let jim = check(source, "jimtcl");
    assert!(c["commands"][0]["hooks"][0]["declaration_conflict"].is_string());
    assert_eq!(
        jim["commands"][0]["hooks"][0]["declaration_conflict"],
        Value::Null
    );
    assert_eq!(c["commands"][0]["hooks"][0]["shape_cacheable"], json!(true));
    assert_eq!(
        jim["commands"][0]["hooks"][0]["shape_cacheable"],
        json!(true)
    );
}

/// `$ctx` the scan cannot attribute to a literal key leaves a declaration
/// unconfirmable. The scan is textual, so it reports the doubt rather than
/// claiming a fault — and never touches the cacheability answer, which the
/// declaration alone decides.
#[test]
fn an_unattributable_ctx_use_leaves_the_declaration_unconfirmed() {
    let source = r"
speclib mylib 1.0 {
    command mylib::dynamic {
        arity 1
        arg_role_resolver -inputs {nwords} {ctx} {
            set key command
            if {[dict get $ctx $key] eq {}} return
            role 0 Body
        }
    }
}
";
    let result = check(source, "tcl9.0");
    let hook = &result["commands"][0]["hooks"][0];
    assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
    assert!(
        hook["declaration_conflict"]
            .as_str()
            .unwrap_or_default()
            .contains("attribute"),
        "{hook}"
    );
}

/// A `ctx` key the calling convention does not define reads as empty
/// forever. The loader carries a body verbatim and never looks inside it,
/// so this report is the only place that typo shows up.
#[test]
fn a_ctx_key_outside_the_calling_convention_is_flagged() {
    let source = r"
speclib mylib 1.0 {
    command mylib::typo {
        arity 1
        arg_role_resolver -inputs {nwords} {ctx} {
            if {[dict get $ctx subcomand] eq {}} return
            role 0 Body
        }
    }
}
";
    let result = check(source, "tcl9.0");
    let hook = &result["commands"][0]["hooks"][0];
    assert_eq!(hook["unknown_ctx_keys"], json!(["subcomand"]), "{hook}");
    // Reading an undefined key is a pack bug, not a caching cost: an
    // always-empty read is a function of nothing at all, so it neither
    // blocks the cache nor contradicts the declaration.
    assert_eq!(hook["shape_cacheable"], json!(true), "{hook}");
    assert_eq!(hook["declaration_conflict"], Value::Null, "{hook}");
}

// The evaluation-only classes.

/// A pack that reaches for a clock fails closed, and the report names
/// the determinism axis rather than describing a half-loaded pack.
#[test]
fn a_determinism_denial_is_a_load_error_naming_its_axis() {
    let result = check(
        "speclib clocky 2.0 {\n    command fine { arity 1 }\n    \
             set now [clock seconds]\n}\n",
        "tcl9.0",
    );
    let error = result["load_error"].as_str().unwrap_or_default();
    assert!(error.contains("determinism axis: clock/time"), "{result}");
    // Registration is transactional: the command declared *before* the
    // denial is not loaded either.
    assert_eq!(result["summary"]["commands"], 0, "{result}");
    let reasons: Vec<&str> = result["notices"]
        .as_array()
        .expect("notices")
        .iter()
        .filter_map(|n| n["reason"].as_str())
        .collect();
    assert!(
        reasons.iter().any(|r| r.contains("transactional")),
        "{reasons:?}"
    );
}

/// A runaway loop is a notice, not a hung tool — which is the whole
/// reason an AI-generated program can be checked at all.
#[test]
fn a_runaway_loop_is_a_budget_notice_naming_its_axis() {
    let result = check(
        "speclib hungry 2.0 {\n    command fine { arity 1 }\n    \
             set i 0\n    while {1} { incr i }\n}\n",
        "tcl9.0",
    );
    let error = result["load_error"].as_str().unwrap_or_default();
    assert!(
        error.contains("budget")
            && (error.contains("command steps") || error.contains("wall clock")),
        "{result}"
    );
    assert_eq!(result["summary"]["commands"], 0, "{result}");
}

/// `available?` makes the answer depend on the analysis target, and the
/// report says so — the E-R1 trap, named where an author will see it.
#[test]
fn a_target_dependent_pack_is_flagged_with_its_notice() {
    let result = check(
        "speclib trap 2.0 {\n    default available {tcl 8.6-}\n    \
             command base { arity 1 }\n    if {[available? {tcl 8.6-}]} {\n        \
             command extra { arity 1 }\n    }\n}\n",
        "tcl9.0",
    );
    assert_eq!(result["target_dependent"], json!(true), "{result}");
    assert_eq!(result["summary"]["commands"], 2, "{result}");
    let reasons: Vec<&str> = result["notices"]
        .as_array()
        .expect("notices")
        .iter()
        .filter_map(|n| n["reason"].as_str())
        .collect();
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("target-dependent registration")),
        "{reasons:?}"
    );
}

/// The provenance class: the pack loads for its author and would be
/// refused from a workspace, and the report says which and why.
#[test]
fn a_workspace_tier_refusal_is_reported_without_failing_the_check() {
    let result = check(
        "speclib sneaky 2.0 {\n    command lsort -override {\n        arity 1..\n    }\n}\n",
        "tcl9.0",
    );
    // The check itself still reports the command and its collision.
    assert_eq!(result["summary"]["commands"], 1, "{result}");
    assert_eq!(result["load_error"], Value::Null, "{result}");
    let refusal = result["untrusted_tier_refusal"]
        .as_str()
        .unwrap_or_default();
    assert!(refusal.contains("design E-R2"), "{result}");
    assert!(refusal.contains("workspace"), "{result}");
    let classes: Vec<&str> = result["notices"]
        .as_array()
        .expect("notices")
        .iter()
        .filter_map(|n| n["class"].as_str())
        .collect();
    assert!(classes.contains(&"provenance"), "{result}");
}

/// The bundled and user tiers are trusted whatever the editor says, and
/// the load never refuses them, so a preview at either draws no refusal
/// — even for an `-override` of a compiled command the workspace tier
/// would refuse — while the workspace default still does.
#[test]
fn a_bundled_or_user_tier_draws_no_refusal() {
    let source =
        "speclib sneaky 2.0 {\n    command lsort -override {\n        arity 1..\n    }\n}\n";
    for (tier, provenance) in [("bundled", "bundled"), ("user", "user")] {
        let result = dispatch(
            "spectcl_check",
            &json!({ "source": source, "dialect": "tcl9.0", "tier": tier }),
        )
        .expect("spectcl_check tool");
        assert_eq!(
            result["untrusted_tier_refusal"],
            Value::Null,
            "{tier}: {result}"
        );
        assert_eq!(result["provenance"], provenance, "{tier}: {result}");
        let classes: Vec<&str> = result["notices"]
            .as_array()
            .expect("notices")
            .iter()
            .filter_map(|n| n["class"].as_str())
            .collect();
        assert!(!classes.contains(&"provenance"), "{tier}: {result}");
        assert_eq!(result["summary"]["commands"], 1, "{tier}: {result}");
    }
    let workspace = check(source, "tcl9.0");
    assert!(
        workspace["untrusted_tier_refusal"].is_string(),
        "the workspace default still previews the refusal: {workspace}"
    );
}

/// A pack that touches nothing reserved carries no refusal.
#[test]
fn an_ordinary_pack_carries_no_tier_refusal() {
    let result = check(VALID, "tcl9.0");
    assert_eq!(result["untrusted_tier_refusal"], Value::Null, "{result}");
    assert_eq!(result["target_dependent"], json!(false), "{result}");
    assert_eq!(result["load_error"], Value::Null, "{result}");
}

/// `tier`/`trust` default to `workspace`/`trusted` — the tier a
/// workspace `.tclspec` file actually installs at, previewed as if the
/// editor already trusts it — so a call naming neither answers exactly
/// as an explicit `workspace`/`trusted` call does, and the two fields
/// this item adds are present but vacuous at that default: no hook is
/// dormant, and the provenance names a trusted workspace.
#[test]
fn the_default_tier_and_trust_equal_an_explicit_workspace_trusted_call() {
    let default = dispatch(
        "spectcl_check",
        &json!({ "source": VALID, "dialect": "tcl9.0" }),
    )
    .expect("spectcl_check tool");
    let explicit = dispatch(
        "spectcl_check",
        &json!({
            "source": VALID,
            "dialect": "tcl9.0",
            "tier": "workspace",
            "trust": "trusted",
        }),
    )
    .expect("spectcl_check tool");
    assert_eq!(
        default, explicit,
        "default must equal an explicit workspace/trusted call"
    );
    assert_eq!(default["provenance"], "trusted workspace", "{default}");
    assert_eq!(default["dormant_hooks"], json!([]), "{default}");
    assert_eq!(default["summary"]["dormant_hooks"], 0, "{default}");
}

/// A `const_fold` body checked as `trust: untrusted` (tier still the
/// `workspace` default): the pack loads there, its body is held dormant,
/// and its command is still fully reported — holding a body dormant
/// costs no analysis fact.
#[test]
fn trust_untrusted_lists_the_hook_as_dormant() {
    let result = dispatch(
        "spectcl_check",
        &json!({ "source": STRLEN_FOLD, "dialect": "tcl9.0", "trust": "untrusted" }),
    )
    .expect("spectcl_check tool");
    assert_eq!(result["provenance"], "untrusted workspace", "{result}");
    assert_eq!(result["untrusted_tier_refusal"], Value::Null, "{result}");
    let dormant = result["dormant_hooks"].as_array().expect("dormant_hooks");
    assert_eq!(dormant.len(), 1, "{result}");
    assert_eq!(dormant[0]["command"], "trustfold::strlen", "{result}");
    assert_eq!(dormant[0]["field"], "const_fold", "{result}");
    assert_eq!(result["summary"]["dormant_hooks"], 1, "{result}");
    assert_eq!(result["summary"]["commands"], 1, "{result}");
    assert_eq!(result["load_error"], Value::Null, "{result}");
}

/// The same body beside an `-override` of a compiled command, checked as
/// `trust: untrusted`: that install refuses the whole pack, so the
/// report names the refusal and no dormant body — a pack that does not
/// load holds nothing dormant. The author's own check still reports both
/// commands (the authority ruling: the preview costs no analysis fact).
#[test]
fn a_pack_the_untrusted_install_refuses_holds_no_dormant_hook() {
    let source = STRLEN_FOLD.replace(
        "\n}\n",
        "\n    command lsort -override {\n        arity 1..\n    }\n}\n",
    );
    let result = dispatch(
        "spectcl_check",
        &json!({ "source": source, "dialect": "tcl9.0", "trust": "untrusted" }),
    )
    .expect("spectcl_check tool");
    assert_eq!(result["provenance"], "untrusted workspace", "{result}");
    let refusal = result["untrusted_tier_refusal"]
        .as_str()
        .unwrap_or_default();
    assert!(refusal.contains("design E-R2"), "{result}");
    assert!(
        refusal.contains("would not be loaded from the untrusted workspace tier"),
        "{result}"
    );
    assert_eq!(result["dormant_hooks"], json!([]), "{result}");
    assert_eq!(result["summary"]["dormant_hooks"], 0, "{result}");
    assert_eq!(result["summary"]["commands"], 2, "{result}");
    assert_eq!(result["load_error"], Value::Null, "{result}");

    // Negative control: the trusted install loads the pack, so the
    // refusal is only the preview's, and still no body is dormant there
    // because a trusted workspace runs it.
    let trusted = check(&source, "tcl9.0");
    assert!(trusted["untrusted_tier_refusal"].is_string(), "{trusted}");
    assert_eq!(trusted["dormant_hooks"], json!([]), "{trusted}");
}

/// The stamp rejection rule, previewed: a workspace install drops a
/// codegen-axis stamp even on `alias_of lassign`, a bundled one admits
/// it there, and a bundled one naming a target that does not carry it
/// drops it and names the target that does.
#[test]
fn stamp_refusals_preview_the_install_the_pair_describes() {
    let unpack = |target: &str| {
        format!(
            "speclib vendor 2.0 {{\n    command vendor::unpack {{\n        arity 2..\n        \
                 alias_of {target}\n        codegen_hook -native Lassign\n    }}\n}}\n"
        )
    };
    let at = |source: &str, tier: &str| {
        dispatch(
            "spectcl_check",
            &json!({ "source": source, "dialect": "tcl9.0", "tier": tier }),
        )
        .expect("spectcl_check tool")
    };

    let workspace = at(&unpack("lassign"), "workspace");
    let refusals = workspace["stamp_refusals"]
        .as_array()
        .expect("stamp_refusals");
    assert_eq!(refusals.len(), 1, "{workspace}");
    assert_eq!(refusals[0]["command"], "vendor::unpack", "{workspace}");
    assert_eq!(refusals[0]["line"], 2, "{workspace}");
    assert_eq!(
        refusals[0]["message"],
        "`codegen_hook Lassign` refused for `vendor::unpack`: a trusted workspace pack may not \
             name a codegen catalogue member; only a bundled pack may carry `alias_of lassign`'s \
             own stamp",
        "{workspace}"
    );
    assert_eq!(workspace["summary"]["stamp_refusals"], 1, "{workspace}");
    // The author's own check still reports the stamp as a field set.
    assert_eq!(workspace["summary"]["commands"], 1, "{workspace}");

    let bundled = at(&unpack("lassign"), "bundled");
    assert_eq!(bundled["stamp_refusals"], json!([]), "{bundled}");

    let wrong = at(&unpack("lsort"), "bundled");
    let refusals = wrong["stamp_refusals"].as_array().expect("stamp_refusals");
    assert_eq!(refusals.len(), 1, "{wrong}");
    let message = refusals[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("`alias_of lsort` names a shipped command that does not carry it")
            && message.ends_with("the stamp would have to sit on `alias_of lassign`"),
        "{wrong}"
    );
}

/// One `const_fold` body — the hook the dormancy previews are about.
const STRLEN_FOLD: &str = "speclib trustfold 2.2 {\n    \
         command trustfold::strlen {\n        \
             arity 1\n        \
             arg 0 -role Value\n        \
             const_fold -inputs {words} {words ctx} {\n            \
                 fold [string length [lindex $words 0]]\n        \
             }\n    \
         }\n}\n";

// ── spectcl_expand ────────────────────────────────────────────────

fn expand(source: &str) -> Value {
    dispatch("spectcl_expand", &json!({ "source": source })).expect("spectcl_expand tool")
}

const PROGRAM: &str = r"speclib fleet 2.0 {
    proc fleet-command {name arity} {
        command math::fleet::$name {
            arity $arity
            traits {CSE_CANDIDATE}
            option -verbose -detail {Report each step as it runs.}
        }
    }
    foreach {name arity} {alpha 2 beta 1 gamma 3} {
        fleet-command $name $arity
    }
}
";

/// The affordance: a loop in, one literal declaration per iteration out.
#[test]
fn expand_writes_a_programmed_packs_registrations_as_canonical_source() {
    let result = expand(PROGRAM);
    assert_eq!(result["pack"], "fleet", "{result}");
    assert_eq!(
        result["commands"],
        json!([
            "math::fleet::alpha",
            "math::fleet::beta",
            "math::fleet::gamma"
        ]),
        "{result}"
    );
    assert_eq!(result["target_dependent"], json!(false));
    assert_eq!(result["load_error"], Value::Null);
    let canonical = result["canonical_source"].as_str().expect("source");
    assert!(
        canonical.contains("command math::fleet::alpha {"),
        "{canonical}"
    );
    assert!(canonical.contains("arity 3"), "{canonical}");
    for word in ["proc ", "foreach ", "$name", "$arity"] {
        assert!(!canonical.contains(word), "{canonical}");
    }
    // And the expansion is a pack the checker accepts, with the same
    // commands — the loop the author is meant to close.
    let checked = check(canonical, "tcl9.0");
    assert_eq!(checked["summary"]["commands"], 3, "{checked}");
    assert_eq!(checked["notices"], json!([]), "{checked}");
}

/// A pack that is already canonical expands to itself, modulo layout:
/// the same commands, the same notices, nothing invented.
#[test]
fn expand_is_a_no_op_on_a_canonical_pack() {
    let once = expand(VALID);
    let source = once["canonical_source"].as_str().expect("source");
    let twice = expand(source);
    assert_eq!(
        twice["canonical_source"], once["canonical_source"],
        "expansion is not idempotent"
    );
    assert_eq!(once["commands"], json!(["mylib::with_var"]), "{once}");
    // The hook body survives verbatim — a `CommandSpec` could not carry
    // it, and the record does.
    assert!(source.contains("arg_role_resolver -inputs"), "{source}");
    assert!(source.contains("role 1 Body"), "{source}");
}

/// A pack whose evaluation failed expands to nothing, and says why.
#[test]
fn expand_reports_a_pack_that_registered_nothing() {
    let result = expand("speclib clocky 2.0 {\n    set now [clock seconds]\n}\n");
    assert!(
        result["load_error"]
            .as_str()
            .unwrap_or_default()
            .contains("determinism axis"),
        "{result}"
    );
    assert_eq!(result["commands"], json!([]), "{result}");
}

/// A source with no `speclib` wrapper loads nothing and says why, rather
/// than erroring.
#[test]
fn a_pack_with_no_speclib_wrapper_reports_a_notice() {
    let result = check("command foo { arity 1 }\n", "tcl9.0");
    assert_eq!(result["summary"]["commands"], 0, "{result}");
    assert!(
        result["notices"].as_array().is_some_and(|n| !n.is_empty()),
        "{result}"
    );
}
