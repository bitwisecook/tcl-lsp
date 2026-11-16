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

//! Parameter traits read from the original source: authored alias and call reports, formal references, conditions and forwarding.

use super::*;

fn original_reference_facts(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<ProcedureId, LocalFacts> {
    let module = &unit.ir_module;
    let identities = crate::realm::document_realm_bindings_for_module(module, registry).unwrap();
    let bodies = summary_procedure_bodies(module, registry);
    let known = bodies.keys().copied().collect();
    scan_all_procs(
        module,
        SummaryBodyInputs {
            bodies: &bodies,
            known: &known,
            unit: Some(unit),
        },
        registry,
        module.dialect_profile,
        ObjectTypeMap::none().candidates,
        &identities,
        None,
    )
}

fn assert_authored_alias_variant_refusals() {
    for body in [
        "upvar #0 $n x; set x 1",
        "upvar 0 $n x; set x 1",
        "upvar 2 $n x; set x 1",
        "upvar $level $n x; set x 1",
        "upvar 1 $n x; unknownCommand; set x 1",
        "upvar 1 $n x; global x; set x 1",
        "if {1} {upvar 1 $n x}; set x 1",
    ] {
        let source = format!("proc p {{n level}} {{{body}}}");
        let (context, unit) = original_reference_unit(&source);
        let (&id, _) = unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let facts = original_reference_facts(&unit, context.commands());
        assert!(
            !facts[&id]
                .param_trait_flags
                .get("n")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite)),
            "{body}"
        );
        assert!(facts[&id].upvar_aliases.is_empty(), "{body}");
    }
    let (_, rejected) = original_reference_unit("proc p {::n} {upvar 1 $::n x; set x 1}");
    assert!(
        rejected
            .ir_module
            .original_authored_declaration_body_units
            .is_empty()
    );
    let (context, unit) = original_reference_unit("proc p {n} {upvar 1 $::n x; set x 1}");
    let (&id, _) = unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let facts = original_reference_facts(&unit, context.commands());
    assert!(
        !facts[&id]
            .param_trait_flags
            .get("::n")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite))
    );
    assert!(
        !facts[&id]
            .param_trait_flags
            .get("n")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite))
    );
    assert!(facts[&id].upvar_aliases.is_empty());
    let (context, unit) =
        original_reference_unit("proc p {n other} {upvar 1 $n x; upvar 1 $other x; set x 1}");
    let (&id, _) = unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let facts = original_reference_facts(&unit, context.commands());
    assert!(facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::VarRead));
    assert!(!facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::VarWrite));
    assert!(facts[&id].param_trait_flags["other"].contains(&ProcArgTrait::VarWrite));
}

#[test]
fn original_authored_alias_reports_keep_source_formals_frames_and_unknown_debts() {
    let source = "proc p {n v spare} {upvar 1 $n x; set x $v}";
    let (context, unit) = original_reference_unit(source);
    let registry = context.commands();
    let module = &unit.ir_module;
    let (&id, _) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let procedure = &module.body_units[&id];
    let tokens = procedure
        .body
        .retained_source_tokens_for_statement(&procedure.body.statements[0])
        .unwrap();
    let command =
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens, module, registry,
        )
        .expect("actual original alias source row");
    assert_eq!(command.body(), id);
    assert!(command.has_unproved_applicability());
    let [alias] = command.aliases() else {
        panic!("one Registry alias declaration");
    };
    let held = command
        .caller_alias(alias)
        .expect("conditional caller alias incidence");
    assert_eq!(held.parameter(), "n");
    let write = procedure
        .body
        .retained_source_tokens_for_statement(&procedure.body.statements[1])
        .unwrap();
    let write = crate::registry_invocation::original_authored_parameter_source_command_for_module(
        write, module, registry,
    )
    .expect("actual retained source successor");
    assert!(held.belongs_to(&write));
    let facts = original_reference_facts(&unit, registry);
    let observed = &facts[&id];
    assert!(observed.param_trait_flags["n"].contains(&ProcArgTrait::VarRead));
    assert!(observed.param_trait_flags["n"].contains(&ProcArgTrait::VarWrite));
    assert!(observed.param_observations_incomplete);
    assert!(observed.has_unknown_calls);
    assert!(observed.upvar_aliases.is_empty());
    assert!(observed.global_aliases.is_empty());
    assert_eq!(observed.name_params, [] as [usize; 0]);
    assert!(observed.source_parameter_aliases.is_empty());
    assert!(
        observed
            .effect_writes
            .intersects(EffectRegion::UNKNOWN_STATE)
    );
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .proved_execution_target()
            .is_none()
    );
    let cloned = tokens.clone();
    assert!(
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            &cloned, module, registry
        )
        .is_none()
    );
    let mut changed = module.clone();
    changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
    assert!(
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens, &changed, registry
        )
        .is_none()
    );
    let mut changed = module.clone();
    changed.body_units.get_mut(&id).unwrap().params_raw = "other".to_owned();
    assert!(
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens, &changed, registry
        )
        .is_none()
    );
    assert!(
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens,
            module,
            tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
        )
        .is_none()
    );

    assert_authored_alias_variant_refusals();
}

fn assert_authored_call_variant_refusals() {
    // #2427: an autoload index entry keeps `unknownCommand` unbounded.
    for body in [
        "set ::auto_index(unknownCommand) {source lib.tcl}; unknownCommand; ::helper $x",
        "$name $x",
        "::helper",
    ] {
        let source = format!("proc ::helper {{v}} {{return $v}}\nproc ::f {{x name}} {{{body}}}");
        let (context, unit) = original_reference_unit(&source);
        let module = &unit.ir_module;
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "::f")
            .unwrap();
        let procedure = &module.body_units[&id];
        let statement = procedure.body.statements.last().unwrap();
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let calls = crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens,
            module,
            context.commands(),
        );
        assert!(
            !calls.is_some_and(|calls| !calls.calls().is_empty()
                && calls.calls().iter().all(
                    crate::registry_invocation::OriginalSourceProcedureCall::accepts_arguments
                )),
            "{body}"
        );
        assert!(
            !original_reference_facts(&unit, context.commands())[&id]
                .param_trait_flags
                .values()
                .any(|traits| traits.contains(&ProcArgTrait::ForwardedToCallee)),
            "{body}"
        );
    }
}

fn assert_authored_call_keeps_its_original_target(
    call: &crate::registry_invocation::OriginalSourceProcedureCall<'_>,
    module: &crate::ir::Module,
    tokens: &crate::ir::CommandTokens,
) {
    assert!(call.accepts_arguments());
    assert_eq!(call.argument_count(), Some(1));
    assert!(std::ptr::eq(
        call.procedure(),
        std::ptr::from_ref(&module.body_units[&call.id()])
    ));
    assert_eq!(
        module.original_authored_declaration_body_units[&call.id()].name(),
        "::helper"
    );
    assert!(
        crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots::from_module(
            module,
            call.procedure()
        )
        .is_none()
    );
    assert!(
        !tokens
            .source_binding
            .as_ref()
            .unwrap()
            .has_original_source_layout_owner()
    );
}

#[test]
fn original_authored_call_reports_keep_actual_source_target_formals_and_debts() {
    let source = "proc ::helper {v} {return $v}\nproc ::f {x spare} {::helper $x}";
    let (context, unit) = original_reference_unit(source);
    let registry = context.commands();
    let module = &unit.ir_module;
    let (&id, _) = module
        .original_authored_declaration_body_units
        .iter()
        .find(|(_, card)| card.name() == "::f")
        .unwrap();
    let procedure = &module.body_units[&id];
    let [statement] = procedure.body.statements.as_slice() else {
        panic!("one actual source call");
    };
    assert!(matches!(statement, crate::ir::Statement::Call { .. }));
    let tokens = procedure
        .body
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(!binding.has_original_source_layout_owner());
    assert!(binding.proved_execution_target().is_none());
    assert!(
        crate::registry_invocation::original_authored_procedure_calls_for_module(
            tokens, module, registry
        )
        .is_none()
    );
    let calls = crate::registry_invocation::original_authored_body_procedure_calls_for_module(
        tokens, module, registry,
    )
    .expect("actual retained source calling template");
    assert_eq!(calls.obligations().len(), 3);
    let [call] = calls.calls() else {
        panic!("one actual original callee allocation");
    };
    assert_authored_call_keeps_its_original_target(call, module, tokens);
    let observed = original_reference_facts(&unit, registry);
    assert!(observed[&id].param_trait_flags["x"].contains(&ProcArgTrait::ForwardedToCallee));
    assert!(observed[&id].param_observations_incomplete);
    assert!(observed[&id].has_unknown_calls);
    assert!(
        observed[&id]
            .effect_writes
            .intersects(crate::side_effects::EffectRegion::UNKNOWN_STATE)
    );
    let mut changed = module.clone();
    changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
    assert!(
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens, &changed, registry
        )
        .is_none()
    );
    let mut changed = module.clone();
    changed.body_units.get_mut(&call.id()).unwrap().params_raw = "other".to_owned();
    assert!(
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens, &changed, registry
        )
        .is_none()
    );
    let cloned = tokens.clone();
    assert!(
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            &cloned, module, registry
        )
        .is_none()
    );
    assert!(
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens,
            module,
            tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
        )
        .is_none()
    );

    assert_authored_call_variant_refusals();
}

#[test]
fn conditional_authored_declaration_reports_do_not_close_unused_parameters() {
    let (context, unit) =
        original_reference_unit("rename $old decl\nproc kept {unused} {return untouched}");
    let module = &unit.ir_module;
    let (&id, card) = module
        .original_authored_declaration_body_units
        .iter()
        .find(|(_, card)| card.name() == "kept")
        .expect("genuine conditional source body");
    assert!(card.matches_procedure(module, &module.body_units[&id]));
    let facts = original_reference_facts(&unit, context.commands());
    let observed = &facts[&id];
    assert!(observed.param_observations_incomplete);
    let traits = finalise_param_traits(
        &module.body_units[&id].params,
        &observed.param_trait_flags,
        None,
        &observed.param_references,
        !observed.param_observations_incomplete,
    );
    assert!(
        !traits
            .get("unused")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Unused))
    );
    assert!(observed.upvar_aliases.is_empty());
    assert!(observed.global_aliases.is_empty());
}

#[test]
fn original_formal_reference_traits_keep_actual_body_function_and_cfg_owners() {
    let (context, unit) = original_reference_unit("proc p {used spare} {return $used}");
    let module = &unit.ir_module;
    let (&id, _) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let procedure = &module.body_units[&id];
    assert!(procedure.body.executed_source.is_none());
    let facts = original_reference_facts(&unit, context.commands());
    let observed = &facts[&id];
    assert!(!observed.param_observations_incomplete);
    let traits = finalise_param_traits(
        &procedure.params,
        &observed.param_trait_flags,
        None,
        &observed.param_references,
        !observed.param_observations_incomplete,
    );
    assert!(traits["used"].contains(&ProcArgTrait::Passthrough));
    assert!(traits["spare"].contains(&ProcArgTrait::Unused));
    for &installed in module.installed_procedure_body_units.keys() {
        assert!(!facts.contains_key(&installed));
        assert!(unit.original_procedure_cfg_for_body(installed).is_none());
        assert!(unit.installed_procedure_cfg_for_body(installed).is_some());
    }
    let mut changed = unit.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .cfg
        .name
        .push_str(" changed");
    let changed_facts = original_reference_facts(&changed, context.commands());
    let missing = &changed_facts[&id];
    assert!(missing.param_observations_incomplete);
    let traits = finalise_param_traits(
        &procedure.params,
        &missing.param_trait_flags,
        None,
        &missing.param_references,
        !missing.param_observations_incomplete,
    );
    assert!(
        traits
            .values()
            .all(|traits| !traits.contains(&ProcArgTrait::Unused))
    );
    let mut changed = unit.clone();
    changed.body_units.get_mut(&id).unwrap().body_id = None;
    assert!(
        original_reference_facts(&changed, context.commands())[&id].param_observations_incomplete
    );
    let mut changed = unit.clone();
    let profile = context.commands().profile().unwrap();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        module.native_lexer_config(),
    ));
    assert_eq!(changed.body_units[&id].cfg, unit.body_units[&id].cfg);
    assert!(
        original_reference_facts(&changed, context.commands())[&id].param_observations_incomplete
    );
    let mut report = unit.clone();
    report
        .ir_module
        .body_units
        .get_mut(&id)
        .unwrap()
        .qualified_name = "unrelated display".into();
    assert!(
        !original_reference_facts(&report, context.commands())[&id].param_observations_incomplete
    );
}

#[test]
fn original_formal_reference_traits_keep_authored_conditions_and_unknown_obligations() {
    let (context, unit) =
        original_reference_unit("proc p {n spare} {if {$n > 0} {return ONE} else {return TWO}}");
    let module = &unit.ir_module;
    let (&id, _) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    assert!(matches!(
        module.body_units[&id].body.statements[0],
        crate::ir::Statement::If { .. }
    ));
    let facts = original_reference_facts(&unit, context.commands());
    assert!(facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::UsedInCondition));
    assert!(!facts[&id].param_references.contains("spare"));
    let (mutation_context, mutation_unit) = original_reference_unit(
        "proc p {first other} {if {$first > 0} {return ONE} else {return TWO}}",
    );
    let (&mutation_id, _) = mutation_unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    assert!(
        original_reference_facts(&mutation_unit, mutation_context.commands())[&mutation_id]
            .param_trait_flags["first"]
            .contains(&ProcArgTrait::UsedInCondition)
    );
    let mut changed = mutation_unit.clone();
    let crate::ir::Statement::If { clauses, .. } = &mut changed
        .ir_module
        .body_units
        .get_mut(&mutation_id)
        .unwrap()
        .body
        .statements[0]
    else {
        unreachable!()
    };
    let crate::expr_ast::ExprNode::Binary { left, .. } = &mut clauses[0].condition else {
        unreachable!()
    };
    let crate::expr_ast::ExprNode::Var { text, .. } = left.as_mut() else {
        unreachable!()
    };
    *text = "$other".into();
    assert_eq!(
        changed.body_units[&mutation_id].cfg,
        mutation_unit.body_units[&mutation_id].cfg
    );
    assert!(
        !changed.ir_module.original_authored_declaration_body_units[&mutation_id]
            .matches_procedure(
                &changed.ir_module,
                &changed.ir_module.body_units[&mutation_id]
            )
    );
    assert!(
        !original_reference_facts(&changed, mutation_context.commands()).contains_key(&mutation_id)
    );
    assert!(module.source_entry.native_entry.is_none());
    let body = &module.body_units[&id].body;
    let tokens = body
        .retained_source_tokens_for_statement(&body.statements[0])
        .unwrap();
    assert!(
        crate::registry_invocation::normal_transfer_invocation(context.commands(), None, tokens)
            .is_none()
    );

    let (context, unit) = original_reference_unit("proc p {used spare} {unknownCommand $used}");
    let (&id, _) = unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let facts = original_reference_facts(&unit, context.commands());
    let facts = &facts[&id];
    assert!(facts.param_observations_incomplete);
    assert!(facts.param_references.contains("used"));
    let traits = finalise_param_traits(
        &unit.ir_module.body_units[&id].params,
        &facts.param_trait_flags,
        None,
        &facts.param_references,
        !facts.param_observations_incomplete,
    );
    assert!(!traits["spare"].contains(&ProcArgTrait::Unused));
}

#[test]
fn original_formal_reference_parts_keep_lexical_names_and_complete_grammar() {
    for (dialect, unicode, nested) in [
        ("tcl8.6", false, false),
        ("tcl9.1", false, true),
        ("jim", true, false),
    ] {
        let profile = tcl_registry::model::ingress::resolve_known_environment(dialect)
            .unwrap()
            .unit_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let source = r"puts $café ${p($k)} $arr($k) \$escaped {$braced} [list ${nested name}]";
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        let names = original_word_reference_names(&plan.commands[0].words).unwrap();
        let expected: HashSet<String> = [
            if unicode { "café" } else { "caf" },
            "p($k)",
            "arr",
            "k",
            "nested name",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(names, expected, "{dialect}");
        let source = "puts ${a{b}c}";
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        let names = original_word_reference_names(&plan.commands[0].words).unwrap();
        assert_eq!(
            names,
            HashSet::from([if nested { "a{b}c" } else { "a{b" }.to_owned()])
        );
    }
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let node = crate::expr_ast::ExprNode::Var {
        text: "${actual name}".into(),
        name: "unrelated report".into(),
        start: 0,
        end: 14,
    };
    let params = HashSet::from(["actual name".to_owned(), "unrelated report".to_owned()]);
    let mut facts = LocalFacts::default();
    note_params_in_expr(&node, &params, &mut facts, config, 0);
    assert!(facts.param_trait_flags.contains_key("actual name"));
    assert!(!facts.param_trait_flags.contains_key("unrelated report"));
    let mut names = Vec::new();
    assert!(walk_collect_param_refs(
        &node,
        &params,
        &mut names,
        Some(config),
        0
    ));
    assert_eq!(names, ["actual name"]);
    assert!(!walk_collect_param_refs(
        &node, &params, &mut names, None, 0
    ));
    let mut facts = LocalFacts {
        param_observations_incomplete: false,
        ..LocalFacts::default()
    };
    note_params_in_expr(&node, &params, &mut facts, config, u32::MAX);
    assert!(facts.param_observations_incomplete);
}

#[test]
fn param_traits_inferred() {
    // `x` is returned → Passthrough; `y` never read → Unused.
    let ia = build("proc ::f {x y} { return $x }");
    let s = fixture_proc(&ia, "::f").unwrap();
    let x_traits = s.param_traits.get("x").expect("x traits");
    assert!(
        x_traits.contains(&ProcArgTrait::Passthrough),
        "expected Passthrough for x, got {x_traits:?}",
    );
    let y_traits = s.param_traits.get("y").expect("y traits");
    assert!(
        y_traits.contains(&ProcArgTrait::Unused),
        "expected Unused for y, got {y_traits:?}",
    );
}

#[test]
fn call_by_name_param_traits_inferred() {
    // `upvar 1 $n x; set x ...` → n is VarWrite;
    // `upvar 1 $n x; return $x` (read only) → n is VarRead.
    let w = build("proc ::setvar {n v} { upvar 1 $n x\nset x $v }");
    let s = fixture_proc(&w, "::setvar").unwrap();
    let nt = s.param_traits.get("n").expect("n traits");
    assert!(
        nt.contains(&ProcArgTrait::VarWrite),
        "expected VarWrite for n (write through upvar alias), got {nt:?}",
    );

    let r = build("proc ::getvar {n} { upvar 1 $n x\nreturn $x }");
    let s2 = fixture_proc(&r, "::getvar").unwrap();
    let nt2 = s2.param_traits.get("n").expect("n traits");
    assert!(
        nt2.contains(&ProcArgTrait::VarRead) && !nt2.contains(&ProcArgTrait::VarWrite),
        "expected VarRead (not VarWrite) for read-only upvar alias, got {nt2:?}",
    );

    // upvar #0 (global frame, not the caller) → VarRead only, no
    // write-back to the caller's passed variable.
    let g = build("proc ::gw {p} { upvar #0 $p x\nset x 1 }");
    let s3 = fixture_proc(&g, "::gw").unwrap();
    let pt = s3.param_traits.get("p").expect("p traits");
    assert!(
        pt.contains(&ProcArgTrait::VarRead) && !pt.contains(&ProcArgTrait::VarWrite),
        "upvar #0 is not a caller-frame write-back, got {pt:?}",
    );
}

/// `upvar $lvl $a b` has three words after the command, so `$lvl` is the
/// level and `($a, b)` the pair, by the argument-count parity
/// `Tcl_UpvarObjCmd` decides on; the text-sniffing reading took `$lvl`
/// for the other variable and paired `($lvl, $a)`. A computed level is no
/// known frame, so `a` names a variable the alias reads and no write-back
/// is claimed; at level 1 the same pair is the caller's write-back.
#[test]
fn upvar_level_word_is_read_by_argument_parity() {
    let ia = build("proc ::q {lvl a} { upvar $lvl $a b\nset b 1 }");
    let s = fixture_proc(&ia, "::q").unwrap();
    let a = s.param_traits.get("a").expect("a traits");
    assert!(
        a.contains(&ProcArgTrait::VarRead) && !a.contains(&ProcArgTrait::VarWrite),
        "{a:?}"
    );
    assert!(
        s.param_traits.get("lvl").is_none_or(|t| {
            !t.contains(&ProcArgTrait::VarRead) && !t.contains(&ProcArgTrait::VarWrite)
        }),
        "{:?}",
        s.param_traits.get("lvl")
    );
    let ia = build("proc ::q {lvl a} { upvar 1 $a b\nset b 1 }");
    let a = ia
        .procedures
        .get(&fixture_id(&ia.procedures, "::q"))
        .expect("authored fixture body")
        .param_traits
        .get("a")
        .expect("a traits");
    assert!(a.contains(&ProcArgTrait::VarWrite), "{a:?}");
}

#[test]
fn used_in_condition_detected() {
    let ia = build("proc ::f {n} { if {$n > 0} { return 1 } else { return 0 } }");
    let s = fixture_proc(&ia, "::f").unwrap();
    let traits = s.param_traits.get("n").expect("n traits");
    assert!(
        traits.contains(&ProcArgTrait::UsedInCondition),
        "expected UsedInCondition for n, got {traits:?}",
    );
}

#[test]
fn forwarded_to_callee_detected() {
    let ia = build("proc ::helper {v} { return $v }\nproc ::f {x} { ::helper $x }");
    let s = fixture_proc(&ia, "::f").unwrap();
    let traits = s.param_traits.get("x").expect("x traits");
    assert!(
        traits.contains(&ProcArgTrait::ForwardedToCallee),
        "expected ForwardedToCallee for x, got {traits:?}",
    );
}
