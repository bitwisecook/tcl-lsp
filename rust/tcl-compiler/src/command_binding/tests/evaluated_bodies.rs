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

//! Readable evaluated bodies: constructed argv, `apply`, `namespace eval`, `uplevel` and `TclOO`.

use super::*;

#[test]
fn constructed_builder_argv_preserves_retained_lexer_overlays() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = current.commands();
    let unit = CompilationUnit::build_for_dialect("set x 1", registry, false, "tcl8.6");
    let bindings = ModuleCommandBindings::analyse(&unit.ir_module, registry);
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let mut config = input.lexer_config();
    config.escapes = tcl_dialect::EscapeSyntax::Tcl84;
    let overlay = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    let spelling = r"[list \U00000041]";
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        registry, &overlay,
    )
    .unwrap();
    assert_eq!(
        constructed_script_words_with_metadata_context(
            spelling,
            registry,
            &bindings,
            "::",
            Some(metadata),
        ),
        Some(vec!["U00000041".to_owned()]),
    );
    assert_eq!(
        constructed_script_words_with_metadata_context(
            spelling,
            registry,
            &bindings,
            "::",
            Some(current.into()),
        ),
        Some(vec!["A".to_owned()]),
    );
    assert_eq!(
        constructed_script_words(spelling, registry, &bindings, "::"),
        Some(vec!["A".to_owned()]),
    );
    assert_eq!(
        constructed_script_words_with_metadata_context(spelling, registry, &bindings, "::", None,),
        None,
    );
}

#[test]
fn static_apply_replays_binding_changes_in_default_and_explicit_namespaces() {
    for (source, moved_name) in [
        ("apply {{} {rename set saved_set}}", "::saved_set"),
        (
            "namespace eval ::n {}\napply {{} {rename set saved_set} ::n}",
            "::n::saved_set",
        ),
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a literal apply lambda is exactly replayable: {source}"
        );
        assert!(
            bindings
                .targets(moved_name, "::")
                .iter()
                .any(|target| target.command == "::set"),
            "apply body did not move set to {moved_name}: {source}"
        );
    }
}

#[test]
fn dynamic_apply_lambda_keeps_the_command_domain_opaque() {
    for (source, opaque) in [
        (
            "set lambda {{} {rename set saved_set}}; apply $lambda",
            false,
        ),
        ("set lambda [read stdin]; apply $lambda", true),
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
        if !opaque {
            assert!(
                bindings
                    .targets("::saved_set", "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                "{source}"
            );
        }
    }
}

#[test]
fn alias_chain_prefix_replays_apply_body_after_arity_validation() {
    let source = "interp alias {} apply_saved {} apply {{} {rename set saved_set}}\n\
                      interp alias {} run_saved {} apply_saved\n\
                      run_saved";
    let (cu, reg) = analyse(source);
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(!bindings.has_opaque_domain());
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the alias chain must supply apply's lambda"
    );

    for source in [
        "interp alias {} bad {} apply {{x} {rename set saved_set}}\nbad",
        "interp alias {} bad {} apply {{{x y z}} {rename set saved_set}}\nbad",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(!bindings.has_opaque_domain(), "known apply error: {source}");
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "an arity or formal-list error must precede the body: {source}"
        );
    }
}

#[test]
fn static_namespace_eval_replays_binding_changes_in_target_namespace() {
    let (cu, reg) = analyse("namespace eval n { rename set saved_set }");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        !bindings.has_opaque_domain(),
        "a literal namespace and body are exactly replayable"
    );
    assert!(
        bindings
            .targets("::n::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "namespace eval body must run in ::n"
    );
}

#[test]
fn readable_nested_body_retains_intermediate_binding_history() {
    let (cu, reg) =
        analyse("namespace eval ::n { rename ::set ::saved_set; rename ::saved_set ::set }");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the transient binding is observable while the evaluated body runs"
    );
    assert!(
        !bindings
            .root_boundary_bindings
            .get("::saved_set")
            .is_some_and(|states| states.iter().any(|state| matches!(
                state,
                MayBinding::Target(target) if target.command == "::set"
            ))),
        "transient history must not become a replayable root boundary"
    );
}

#[test]
fn dynamic_namespace_eval_target_or_body_keeps_the_domain_opaque() {
    for source in [
        "proc change {ns} {namespace eval $ns {rename set saved_set}}",
        "proc change {body} {namespace eval ::n $body}",
        "proc change {old} {namespace eval ::n [list rename $old saved_set]}",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "dynamic namespace eval input must not be replayed as source: {source}"
        );
    }
}

#[test]
fn frozen_namespace_eval_inputs_retain_the_actual_entered_rename() {
    for source in [
        "set ns ::n\nnamespace eval $ns {rename set saved_set}",
        "set body {rename set saved_set}\nnamespace eval ::n $body",
        "set old set\nnamespace eval ::n [list rename $old saved_set]",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| { target.command == "::set" }),
            "{source}"
        );
    }
}

#[test]
fn alias_chain_prefix_replays_namespace_eval_in_normalised_namespace() {
    let source = "interp alias {} in_n {} namespace eval n::\n\
                      interp alias {} run_in_n {} in_n\n\
                      run_in_n {rename set saved_set}";
    let (cu, reg) = analyse(source);
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(!bindings.has_opaque_domain());
    assert!(
        bindings
            .targets("::n::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the alias chain must supply namespace eval's normalised target"
    );
}

#[test]
fn empty_namespace_eval_prefix_enters_the_global_body() {
    // Pinned C 8.4–9.1 accepts the empty namespace selector as global.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(profile).unit_profile();
        let reg = CommandRegistry::build_default().project_for_profile(profile);
        for (body, moved) in [("rename set saved_set", true), ("\"unterminated", false)] {
            let source =
                format!("interp alias {{}} in_empty {{}} namespace eval {{}}\nin_empty {{{body}}}");
            let cu = CompilationUnit::build_for_profile(&source, &reg, false, profile);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(!bindings.has_opaque_domain(), "{}: {body}", profile.name);
            assert_eq!(
                bindings
                    .targets("::saved_set", "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                moved,
                "{}: {body}",
                profile.name,
            );
        }
    }
}

#[test]
fn static_delete_and_recreate_does_not_widen_unrelated_commands() {
    let (cu, reg) = analyse(
        "proc p {} {}\n\
             rename p {}\n\
             proc p {} { set local 1 }",
    );
    assert!(cu.ir_module.redefined_procedures.contains("::p"));
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        !bindings.has_opaque_domain(),
        "a readable discarded procedure body must not poison the command domain"
    );
    assert!(
        !bindings.target_may_be_unknown("set", "::"),
        "the unrelated set command remains exactly known"
    );
}

#[test]
fn absolute_heads_keep_relative_transition_operands_in_the_current_namespace() {
    let (cu, reg) = analyse("namespace eval ::n {::rename ::set saved_set}");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::n::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "absolute head lookup must not replace the command's current namespace"
    );
    assert!(
        !bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the relative destination must not be moved into the global namespace"
    );
}

#[test]
fn absolute_proc_head_keeps_current_namespace_for_discarded_body() {
    let (cu, reg) = analyse(
        "namespace eval ::n {\n\
                 ::proc p {} {::rename ::set saved_set}\n\
                 ::proc p {} {}\n\
             }",
    );
    assert!(cu.ir_module.redefined_procedures.contains("::n::p"));
    let history = discarded_procedure_history(&cu.ir_module, &reg);
    assert!(!history.opaque);
    assert!(
        history
            .modules
            .iter()
            .any(|module| module.top_level_namespace == "::n")
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::n::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set")
    );
    assert!(
        !bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set")
    );
}

#[test]
fn consumed_typed_head_is_revalidated_in_future_roots() {
    let (cu, reg) = analyse(
        "proc ::uses_set {} { set x 1 }\n\
             rename ::set ::saved_set",
    );
    assert!(
        cu.ir_module
            .procedures
            .get(&fixture_id(&cu.ir_module.procedures, "::uses_set"))
            .expect("authored fixture body")
            .body
            .command_binding_sites
            .iter()
            .next()
            .is_some(),
        "structured lowering must retain the consumed set dependency"
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.has_opaque_domain(),
        "the callable procedure may execute after set no longer denotes the compiled identity"
    );
}

#[test]
fn uplevel_global_proc_body_is_recovered_in_its_runtime_namespace() {
    for wrapper in [
        "uplevel #0 {proc p {} {rename expr saved_expr}}",
        "uplevel #0 {eval {proc p {} {rename expr saved_expr}}}",
    ] {
        let source = format!(
            "proc p {{}} {{}}\n\
                 namespace eval ::n {{\n\
                     proc installer {{}} {{ {wrapper} }}\n\
                 }}"
        );
        let (cu, reg) = analyse(&source);
        assert!(
            cu.ir_module.redefined_procedures.contains("::p"),
            "the #0 body is lowered in the same global namespace Tcl selects"
        );
        let history = discarded_procedure_history(&cu.ir_module, &reg);
        assert!(!history.opaque, "literal runtime body is exact: {wrapper}");
        assert!(
            history
                .modules
                .iter()
                .any(|module| module.top_level_namespace == "::"),
            "misqualified retained bodies must be re-lowered globally: {wrapper}"
        );
        assert!(history.rebound_names.contains("::p"));

        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "literal bodies should stay closed: {bindings:#?}"
        );
        assert!(
            bindings
                .targets("::saved_expr", "::")
                .iter()
                .any(|target| target.command == "::expr"),
            "the recovered body must mutate global expr, not ::n::expr: {wrapper}"
        );
        assert!(
            !scan_module_command_mutations(&cu.ir_module, &reg).trusts_proc_binding("::p"),
            "both runtime definitions of ::p make its identity unstable"
        );
    }
}

#[test]
fn unreadable_uplevel_global_proc_body_is_opaque() {
    let (cu, reg) = analyse(
        "namespace eval ::n {\n\
                 proc installer {body} { uplevel #0 {proc p {} $body} }\n\
             }",
    );
    let history = discarded_procedure_history(&cu.ir_module, &reg);
    assert!(
        history.opaque,
        "a misqualified retained procedure cannot borrow an unreadable lexical body"
    );
}

#[test]
fn unavailable_discarded_redefinition_body_keeps_the_domain_opaque() {
    let (cu, reg) = analyse(
        "proc install {replacement} {\n\
                 set p_name {p}\n\
                 proc p {} {}\n\
                 rename p {}\n\
                 proc $p_name {} $replacement\n\
             }\n\
             install {rename set saved_set}\n\
             p",
    );
    assert!(cu.ir_module.redefined_procedures.contains("::p"));
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.has_opaque_domain(),
        "a runtime-supplied discarded body may mutate any command binding"
    );
}

#[test]
fn direct_and_alias_prefixed_proc_bodies_are_recovered_before_binding() {
    for source in [
        "proc p {} {rename expr saved_expr}\np",
        "interp alias {} makep {} proc p; makep {} {rename expr saved_expr}; p",
        "interp alias {} makep_target {} proc p\n\
             interp alias {} makep {} makep_target\n\
             makep {} {rename expr saved_expr}\n\
             p",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a literal typed proc body is exactly recoverable: {source}"
        );
        assert!(
            bindings
                .targets("::saved_expr", "::")
                .iter()
                .any(|target| target.command == "::expr"),
            "the recovered procedure body must contribute its expr rename: {source}"
        );
        let mutations = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !mutations.trusts("expr"),
            "compiler consumers must not trust expr after the recovered body: {source}"
        );
    }
}

#[test]
fn constructed_namespace_eval_defers_procedure_body_effects_to_the_root_fixpoint() {
    let (cu, reg) = analyse("namespace eval ::n [list proc mutate {} {rename ::set ::saved_set}]");
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        !bindings.has_opaque_domain(),
        "the constructed script and procedure body are both exact"
    );
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "the constructed procedure must become a future executable root"
    );
}

#[test]
fn constructed_procedure_roots_discovered_by_a_root_reach_the_next_round() {
    let (cu, reg) = analyse(
        "namespace eval ::n [list proc installer {} {\
                 proc late {} {rename ::set ::saved_set}\
             }]",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "a nested definition discovered while replaying installer must schedule late"
    );
}

#[test]
fn constructed_procedure_redefinitions_retain_every_possible_body() {
    let (cu, reg) = analyse(
        "namespace eval ::n [list proc mutate {} {rename ::set ::saved_set}]\n\
             namespace eval ::n [list proc mutate {} {rename ::expr ::saved_expr}]",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    for (alias, target) in [("::saved_set", "::set"), ("::saved_expr", "::expr")] {
        assert!(
            bindings
                .targets(alias, "::")
                .iter()
                .any(|resolved| resolved.command == target),
            "every exact replacement body remains a possible future root: {alias}"
        );
    }
}

#[test]
fn constructed_namespaced_tcloo_method_uses_runtime_receiver_namespace() {
    let (cu, reg) = analyse(
        "namespace eval ::n { oo::class create C {\
                 method mutate {} {set x 1}\
             } }",
    );
    assert_eq!(
        cu.ir_module
            .methods
            .get(&fixture_id(&cu.ir_module.methods, "::n::C::mutate"))
            .expect("authored fixture body")
            .execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.has_opaque_domain(),
        "an object-local set may shadow the typed-lowered global builtin"
    );
}

#[test]
fn unretained_tcloo_body_keeps_the_command_domain_opaque() {
    let (cu, reg) = analyse(
        "set ::body {::rename ::set ::saved_set}\n\
             oo::class create C {method mutate {} $::body}",
    );
    assert!(cu.ir_module.oo_evidence.unretained_executable_roots);
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings.has_opaque_domain(),
        "a runtime-callable body absent from the retained root inventory must fail closed"
    );
    assert!(
        scan_module_command_mutations(&cu.ir_module, &reg).has_dynamic_mutation(),
        "the optimiser projection must retain closed-lattice opacity"
    );
}

#[test]
fn constructed_tcloo_method_keeps_absolute_command_heads_exact() {
    let (cu, reg) = analyse(
        "namespace eval ::n [list oo::class create C {\
                 method mutate {} {::rename ::set ::saved_set}\
             }]",
    );
    let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
    assert!(
        bindings
            .targets("::saved_set", "::")
            .iter()
            .any(|target| target.command == "::set"),
        "an absolute command and target remain namespace-invariant"
    );
}

#[test]
fn unavailable_direct_or_alias_prefixed_proc_body_is_opaque() {
    for source in [
        "set body {rename expr saved_expr}\nproc p {} $body\np",
        "set body {rename expr saved_expr}\n\
             interp alias {} makep {} proc p\n\
             makep {} $body\n\
             p",
    ] {
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a procedure target without a retained or recovered body is opaque: {source}"
        );
        assert!(
            bindings.target_may_be_unknown("expr", "::"),
            "the unavailable procedure body must make expr untrusted: {source}"
        );
        assert!(
            !scan_module_command_mutations(&cu.ir_module, &reg).trusts("expr"),
            "the unavailable body makes the whole command-trust projection dynamic: {source}"
        );
    }
}
