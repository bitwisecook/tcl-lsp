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

//! Procedure targets: seeded parameters, parity with a static `proc` across profiles and constructed namespaces.

use super::*;

#[test]
fn procedure_target_seeds_params_and_supports_both_dispatch_modes() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let parameters = vec!["value".to_owned(), "suffix".to_owned()];
    let target = ProcedureCompileTarget {
        source: "append value $suffix; return $value",
        parameters: &parameters,
        namespace: "example",
    };
    let fast = service
        .compile_procedure_for_profile(target, profile, ProcedureDispatch::Optimised)
        .unwrap();
    let plain = service
        .compile_procedure_for_profile(target, profile, ProcedureDispatch::Plain)
        .unwrap();

    assert_eq!(&fast.top_level.lvt.entries()[..2], ["value", "suffix"]);
    assert!(std::ptr::eq(fast.profile, profile));
    assert!(!fast.top_level.plain_command_dispatch);
    assert!(
        fast.top_level
            .command_bindings
            .iter()
            .any(|binding| binding.name == "append" && binding.identity == "append")
    );
    assert!(plain.top_level.plain_command_dispatch);
    assert_eq!(
        plain.top_level.command_bindings,
        [] as [tcl_runtime_api::CommandBindingIdentity; 0]
    );

    let static_proc = service
        .compile("proc p {} {mutate; if {1} {return yes}}")
        .unwrap();
    assert_eq!(
        static_proc.procedure_provenance[&fixture_id(&static_proc.procedures, "::p")],
        tcl_bytecode::ProcedureProvenance {
            name: "::p".to_owned(),
            namespace_context: None,
            parameters: String::new(),
            body: "mutate; if {1} {return yes}".to_owned(),
        }
    );
}

#[test]
fn same_name_projected_profiles_keep_cache_policies_and_artifact_handles() {
    use tcl_dialect::model::{DialectPoint, Release};
    let early = Box::leak(Box::new(DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        DialectPoint::canonical(Release::JIM_0_80),
    )));
    let late = Box::leak(Box::new(DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        DialectPoint::canonical(Release::JIM_0_84),
    )));
    for service in [
        BytecodeCompileService::default(),
        BytecodeCompileService::for_profile(early),
    ] {
        for profile in [early as &'static DialectProfile, late] {
            let view = service
                .registry
                .registry_for_profile(profile)
                .expect("an installed standalone profile generation");
            assert_eq!(
                view.as_ref().profile().unwrap().cache_key(),
                profile.cache_key()
            );
            let fast = service.compile_for_profile("set x 1", profile).unwrap();
            let plain = service
                .compile_plain_script_for_profile(
                    ScriptCompileTarget {
                        source: "set x 1",
                        namespace: "",
                        evaluation: CommandEvaluation::Compiled,
                    },
                    profile,
                )
                .unwrap();
            assert!(std::ptr::eq(fast.profile, profile));
            assert!(std::ptr::eq(plain.profile, profile));
            let parameters = Vec::new();
            let target = ProcedureCompileTarget {
                source: "return 1",
                namespace: "",
                parameters: &parameters,
            };
            let procedure = service
                .compile_procedure_for_profile(target, profile, ProcedureDispatch::Optimised)
                .unwrap();
            assert!(std::ptr::eq(procedure.profile, profile));
        }
    }
}

#[test]
fn procedure_target_matches_static_proc_across_profiles_and_dispatches() {
    // Keep the cases together: each exercises module state a
    // procedure-target lowering must assemble the same way a static
    // `proc` body does (procedure frame/LVT, namespace resolution, nested and
    // const-materialised procedures, command aliases and rename, and
    // namespace directives).
    let body = PROCEDURE_TARGET_MATRIX_BODY;
    let parameters = vec!["value".to_owned(), "suffix".to_owned()];

    for environment in ["tcl8.4", "tcl9.0"] {
        let profile =
            tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        // `proc ::matrix::p` needs its namespace: tclsh 8.4.20/9.0.4 refuse it
        // otherwise (`can't create procedure "::matrix::p": unknown namespace`).
        let static_source =
            format!("namespace eval ::matrix {{}}; proc ::matrix::p {{value suffix}} {{{body}}}");
        let static_module = service
            .compile_for_profile(&static_source, profile)
            .unwrap();
        let static_ir = lower_to_ir_for_bytecode_with_dialect(
            &static_source,
            profile_registry(&service, profile).as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
        );

        for dispatch in [ProcedureDispatch::Optimised, ProcedureDispatch::Plain] {
            let direct_ir = lower_proc_body_module_for_bytecode(
                body,
                "matrix",
                profile_registry(&service, profile).as_ref(),
                LexerConfig::from_grammar(profile.grammar),
                Some(profile),
                dispatch == ProcedureDispatch::Plain,
            );
            let direct_module = service
                .compile_procedure_for_profile(
                    ProcedureCompileTarget {
                        source: body,
                        parameters: &parameters,
                        namespace: "matrix",
                    },
                    profile,
                    dispatch,
                )
                .unwrap();
            let static_proc = static_module
                .procedures
                .get(&fixture_id(&static_module.procedures, "::matrix::p"))
                .expect("authored fixture body");
            let direct_proc = &direct_module.top_level;

            if dispatch == ProcedureDispatch::Optimised {
                assert_eq!(
                    static_proc.lvt.entries(),
                    direct_proc.lvt.entries(),
                    "{environment:?} procedure parameters/LVT"
                );
                assert_eq!(
                    static_proc.literals.entries(),
                    direct_proc.literals.entries(),
                    "{environment:?} literal materialisation"
                );
                let static_instructions =
                    instructions_without_source_spans(&static_proc.instructions);
                let direct_instructions =
                    instructions_without_source_spans(&direct_proc.instructions);
                assert_eq!(
                    static_instructions, direct_instructions,
                    "{environment:?} procedure instruction shape"
                );
                assert_eq!(
                    static_proc.labels, direct_proc.labels,
                    "{environment:?} procedure labels"
                );
                assert_eq!(
                    static_proc.loop_targets, direct_proc.loop_targets,
                    "{environment:?} loop targets"
                );
                assert_eq!(
                    static_proc.command_bindings, direct_proc.command_bindings,
                    "{environment:?} command bindings"
                );
                assert!(
                    direct_module.procedures.is_empty(),
                    "{environment:?} runtime target compiled an unentered nested body"
                );
                assert!(direct_module.procedure_provenance.is_empty());
            } else {
                assert!(direct_proc.plain_command_dispatch);
                assert_eq!(
                    direct_proc.command_bindings,
                    [] as [tcl_runtime_api::CommandBindingIdentity; 0]
                );
            }
            assert_eq!(
                static_ir.namespace_imports, direct_ir.namespace_imports,
                "{environment:?} {dispatch:?} namespace imports"
            );
            assert_eq!(
                static_ir.namespace_exports, direct_ir.namespace_exports,
                "{environment:?} {dispatch:?} namespace exports"
            );
        }
    }
}

#[test]
fn procedure_target_roots_literal_colon_constructed_namespace_once() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let registry = profile_registry(&service, profile);
    let config = LexerConfig::from_grammar(profile.grammar);
    let body = "namespace export exposed\n\
                    proc child {} {return [namespace current]}\n\
                    return [namespace current]";
    let direct_ir = lower_proc_body_module_for_bytecode(
        body,
        ":",
        registry.as_ref(),
        config,
        Some(profile),
        false,
    );
    let direct_module = service
        .compile_procedure_for_profile(
            ProcedureCompileTarget {
                source: body,
                parameters: &[],
                namespace: ":",
            },
            profile,
            ProcedureDispatch::Optimised,
        )
        .expect("literal-colon procedure target compiles");

    assert_eq!(
        direct_ir.namespace_exports,
        [(":::".to_owned(), "exposed".to_owned())]
    );
    assert!(fixture_id_if_present(&direct_ir.procedures, ":::::child").is_some());
    assert!(direct_module.procedures.is_empty());
    assert!(direct_module.procedure_provenance.is_empty());

    let static_source = "namespace eval : {\
            proc p {} {\
                namespace export exposed\n\
                proc child {} {return [namespace current]}\n\
                return [namespace current]\
            }\
        }";
    let static_ir = lower_to_ir_for_bytecode_with_dialect(
        static_source,
        registry.as_ref(),
        config,
        Some(profile),
    );
    let namespace_proofs: Vec<_> = fixture_id_if_present(&static_ir.procedures, ":::::p")
        .and_then(|id| static_ir.procedures.get(&id))
        .into_iter()
        .flat_map(|procedure| &procedure.body.statements)
        .filter_map(|statement| statement.tokens())
        .map(|tokens| {
            let binding = tokens.source_binding.as_ref();
            (
                &tokens.argv_texts,
                binding.map(|binding| &binding.lookup_namespace),
                binding.and_then(|binding| binding.proved_handler_target()),
                binding.and_then(|binding| binding.proved_execution_target()),
                binding.map(|binding| &binding.evaluated_argument_values),
                tcl_compiler::registry_invocation::namespace_directive_footprint(
                    registry.as_ref(),
                    None,
                    tokens,
                ),
            )
        })
        .collect();
    assert_eq!(
        static_ir.namespace_exports,
        direct_ir.namespace_exports,
        "literal-colon procedures: {:?}; body units: {:?}; namespace proofs: {namespace_proofs:?}",
        static_ir.procedures.keys().collect::<Vec<_>>(),
        static_ir.body_units.keys().collect::<Vec<_>>()
    );
    assert!(fixture_id_if_present(&static_ir.procedures, ":::::p").is_some());
}
