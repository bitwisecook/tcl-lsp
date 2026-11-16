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

//! Entered source scopes: deferred procedure bodies and namespace children, and whole-module artifacts.

use super::*;

#[test]
fn runtime_targets_leave_unentered_procedure_bodies_in_original_source() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile =
            tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let source =
            tcl_runtime_api::SourceImage::native(&b"proc child {} {\"}; set marker ENTERED"[..]);
        let namespace = tcl_runtime_api::ByteNamespacePath::root();
        for plain in [false, true] {
            let target = ScriptCompileTargetBytes {
                source: &source,
                namespace: &namespace,
            };
            let script = if plain {
                service.compile_plain_script_bytes_for_profile(target, profile)
            } else {
                service.compile_script_bytes_for_profile(target, profile)
            }
            .expect("unentered malformed child does not invalidate entered script");
            assert!(script.procedures.is_empty(), "{environment}");
            assert!(script.procedure_provenance.is_empty());
            assert_eq!(script.source, source, "{environment} original image");
            assert!(
                script.top_level.instructions.iter().any(|instruction| {
                    instruction.source_cmd_text.bytes() == b"proc child {} {\"}"
                }),
                "{environment} original declaration invocation"
            );
            let procedure = service
                .compile_procedure_bytes_for_profile(
                    ProcedureCompileTargetBytes {
                        source: &source,
                        parameters: &[],
                        namespace: &namespace,
                    },
                    profile,
                    if plain {
                        ProcedureDispatch::Plain
                    } else {
                        ProcedureDispatch::Optimised
                    },
                )
                .expect("entered procedure leaves nested body lazy");
            assert!(procedure.procedures.is_empty(), "{environment}");
            assert!(procedure.procedure_provenance.is_empty());
            assert_eq!(procedure.source, source);
        }
        let unicode = service
            .compile_script_for_profile(
                ScriptCompileTarget {
                    source: "proc child {} {\"}; set marker ENTERED",
                    namespace: "",
                    evaluation: CommandEvaluation::Compiled,
                },
                profile,
            )
            .expect("Unicode target uses the same entered-source policy");
        assert!(unicode.procedures.is_empty());
    }
}

#[test]
fn explicit_whole_module_artifact_keeps_procedure_inventory() {
    let source = "proc child {} {return KEPT}; set marker ENTERED";
    let module = BytecodeCompileService::default().compile(source).unwrap();
    assert!(fixture_id_if_present(&module.procedures, "::child").is_some());
    assert_eq!(
        module.procedure_provenance[&fixture_id(&module.procedures, "::child")].body,
        "return KEPT"
    );
}

#[test]
fn entered_source_scope_excludes_deferred_bodies_before_cfg_preparation() {
    let source = "proc child {} {proc grand {} {puts NEVER}; return KEPT}; set marker ENTERED";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = LexerConfig::from_grammar(profile.grammar);
    for scope in [
        ModuleEmissionScope::WholeModule,
        ModuleEmissionScope::EnteredSource,
    ] {
        let options = BytecodeCompileService::source_analysis_options(
            CompileSourcePolicy { entry: None, scope },
            registry,
            config,
            Some(profile),
        );
        let ir = lower_script_module_for_bytecode_with_options(
            source,
            "",
            registry,
            config,
            Some(profile),
            false,
            options,
        );
        assert_eq!(ir.source_entry.compilation_scope, scope);
        let entered = scope == ModuleEmissionScope::EnteredSource;
        assert_eq!(ir.procedures.is_empty(), entered);
        assert!(ir.top_level.statements.iter().any(|statement| {
            statement.tokens().is_some_and(|tokens| {
                tokens.argv_texts.first().is_some_and(|head| head == "proc")
                    && tokens
                        .argv_texts
                        .last()
                        .is_some_and(|body| body.contains("puts NEVER"))
            })
        }));
        if entered {
            assert_eq!(
                ir.future_call_sites,
                [] as [tcl_compiler::command_binding::SourceFutureCallSite; 0]
            );
            assert!(ir.body_units.is_empty());
            assert!(ir.methods.is_empty());
        }
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        assert_eq!(cfg.procedures.is_empty(), entered);
    }
}

#[test]
fn entered_source_defers_generic_namespace_children_before_cfg_preparation() {
    let source = "namespace eval ::outer {namespace eval ::inner {proc lazy {} {\"}; set child UNENTERED}}; set marker ENTERED";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = LexerConfig::from_grammar(profile.grammar);
    for scope in [
        ModuleEmissionScope::WholeModule,
        ModuleEmissionScope::EnteredSource,
    ] {
        let options = BytecodeCompileService::source_analysis_options(
            CompileSourcePolicy { entry: None, scope },
            registry,
            config,
            Some(profile),
        );
        let ir = lower_script_module_for_bytecode_with_options(
            source,
            "",
            registry,
            config,
            Some(profile),
            false,
            options,
        );
        let entered = scope == ModuleEmissionScope::EnteredSource;
        assert_eq!(ir.procedures.is_empty(), entered);
        assert_eq!(ir.body_units.is_empty(), entered);
        let namespace_call = ir.top_level.statements.iter().find_map(|statement| {
            statement.tokens().filter(|tokens| {
                tokens
                    .argv_texts
                    .first()
                    .is_some_and(|head| head == "namespace")
            })
        });
        let tokens = namespace_call.expect("the original namespace invocation is retained");
        assert_eq!(
            tokens.argv_texts[3],
            "namespace eval ::inner {proc lazy {} {\"}; set child UNENTERED}"
        );
        if entered {
            assert!(tokens.evaluated_body().is_none());
            assert_eq!(
                ir.future_call_sites,
                [] as [tcl_compiler::command_binding::SourceFutureCallSite; 0]
            );
            assert!(ir.methods.is_empty());
        }
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        assert_eq!(cfg.procedures.is_empty(), entered);
    }
}

#[test]
fn entered_source_keeps_native_structured_body_preflight() {
    let source = tcl_lexer::SourceImage::native(b"if {1} {puts \"}".as_slice());
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile =
            tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
        let entry: crate::test_support::CapturedNativeEntry =
            crate::test_support::captured_native_entry(profile);
        assert!(entry.closed, "{environment}: actual compilation entry");
        let selected = entry
            .lookup_command_bytes(entry.current_namespace, b"if")
            .expect("actual original command lookup")
            .expect("registered native if command");
        assert_eq!(
            selected.compiler_hook,
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present,
            "{environment}: actual if compiler hook",
        );
        assert!(
            selected.compiler.is_some(),
            "{environment}: selected compiler"
        );
        let script = BytecodeCompileService::for_profile(profile)
            .compile_script_bytes_with_entry(
                ScriptCompileTargetBytes {
                    source: &source,
                    namespace: &tcl_runtime_api::ByteNamespacePath::root(),
                },
                profile,
                &entry,
            )
            .expect("native syntax failure remains a presentable guest failure");
        let failure = script
            .top_level
            .native_compilation_failure
            .as_ref()
            .expect("the selected compiler visits the literal if body");
        assert_eq!(failure.message, "missing \"", "{environment}");
    }
}
