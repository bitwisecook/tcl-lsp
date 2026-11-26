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

//! Procedure bodies the compiler inlined: invalidation after redefinition, and revalidation and replay in the defining namespace.

use super::*;

/// Test-only executable pipeline which opts into the public IR inliner before
/// CFG/codegen. Production `BytecodeCompileService` deliberately does not.
struct InliningCompilerSvc(BytecodeCompileService);

impl InliningCompilerSvc {
    fn compile_script_with_ir(
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<(tcl_bytecode::ModuleAsm, tcl_compiler::ir::Module), CompileError> {
        // The document a fresh interpreter of `profile` runs, as the service
        // lowers one: a module lowered with no entry has no source purpose,
        // and the inliner proves nothing about it.
        let context = tcl_registry::model::ingress::resolve_environment(profile.name)
            .default_context_registry();
        let registry = context.commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
            registry,
            &input,
            config,
            Some(profile),
        );
        let mut options = entry.options();
        options.compilation_scope = tcl_runtime_api::SourceCompilationScope::WholeModule;
        options.native_compilation.mode =
            tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
        let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(registry, config)
            .with_dialect(Some(profile))
            .for_bytecode_backend()
            .with_resolved_analysis_input(input.clone());
        lowerer.set_source_analysis_options(options);
        lowerer.lower_script_target(target.source, target.namespace);
        let ir = lowerer.finish_module(target.source);
        let ir = inline_module(ir, registry);
        let cfg = build_cfg_codegen(&ir, false);
        let module = codegen_module(&cfg, &ir, registry)?;
        Ok((module, ir))
    }
}

fn compiled_body_at_declaration<'a>(
    ir: &tcl_compiler::ir::Module,
    module: &'a tcl_bytecode::ModuleAsm,
    declaration: usize,
    report_name: &str,
) -> &'a tcl_bytecode::FunctionAsm {
    let span = ir.top_level.statements[declaration].span();
    let mut bodies = ir
        .procedures
        .iter()
        .filter(|(_, procedure)| procedure.span == span);
    let (id, procedure) = bodies.next().expect("body at the original declaration");
    assert!(
        bodies.next().is_none(),
        "one body at the original declaration"
    );
    assert_eq!(
        procedure.qualified_name, report_name,
        "fixture reporting name"
    );
    module
        .procedures
        .get(id)
        .expect("same-module compiled body")
}

impl CompileService for InliningCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        let ir = tcl_compiler::lowering::lower_script_bytes_module_for_bytecode_with_options(
            target,
            registry,
            config,
            Some(profile),
            false,
            None,
        )
        .map_err(|error| match error {
            tcl_lexer::NativeWordError::Parse(message) => {
                tcl_runtime_api::CompileError::Message(message.into())
            }
            error => tcl_runtime_api::CompileError::Unsupported(format!(
                "original native word unavailable: {error:?}"
            )),
        })?;
        let ir = inline_module(ir, registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry).expect("original source namespace"))
    }
    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        BytecodeCompileService::for_profile(profile)
            .compile_substitution_with_entry(target, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        let ir = tcl_compiler::lowering::lower_script_bytes_module_for_bytecode_with_options(
            target,
            registry,
            config,
            Some(profile),
            false,
            Some(BytecodeCompileService::native_entry_options(
                entry,
                Some(profile),
            )),
        )
        .map_err(|error| match error {
            tcl_lexer::NativeWordError::Parse(message) => {
                tcl_runtime_api::CompileError::Message(message.into())
            }
            error => tcl_runtime_api::CompileError::Unsupported(format!(
                "original native word unavailable: {error:?}"
            )),
        })?;
        let ir = inline_module(ir, registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry).expect("original source namespace"))
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_plain_script_bytes_for_profile(target, profile)
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_procedure_bytes_for_profile(target, profile, dispatch)
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
    }
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        self.0
            .script_command_plan_bytes_for_profile(source, profile)
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        self.0
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile_script_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        let ir = tcl_compiler::lowering::lower_script_module_for_bytecode_with_options(
            target.source,
            target.namespace,
            registry,
            config,
            Some(profile),
            false,
            Some(BytecodeCompileService::native_entry_options(
                entry,
                Some(profile),
            )),
        );
        let ir = inline_module(ir, registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry).expect("original source namespace"))
    }

    fn compile_procedure_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        self.0
            .compile_procedure_with_entry(target, profile, entry, dispatch)
    }

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.0.compile(src)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.compile_script_for_profile(
            ScriptCompileTarget {
                source: src,
                namespace: "",
                evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
            },
            profile,
        )
    }

    fn compile_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        Self::compile_script_with_ir(target, profile).map(|(module, _)| module)
    }

    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        let ir = lower_to_ir_traced_with_dialect(src, registry, config, Some(profile));
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry).expect("original source namespace"))
    }

    fn compile_plain_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.0.compile_plain_script_for_profile(target, profile)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        self.0
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

#[test]
fn explicit_inline_codegen_pipeline_invalidates_after_user_proc_redefinition() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = "proc target {} {return OLD}; proc caller {} {target}";
    let _compiler_owner = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let (module, ir) = InliningCompilerSvc::compile_script_with_ir(
        ScriptCompileTarget {
            source,
            namespace: "",
            evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
        },
        profile,
    )
    .expect("explicit inline pipeline compiles");
    assert_eq!(
        compiled_body_at_declaration(&ir, &module, ir.top_level.statements.len() - 1, "::caller")
            .procedure_bindings,
        [tcl_runtime_api::ProcedureBindingIdentity::new(
            "target",
            "::target",
            "",
            "return OLD",
        )],
        "the consumed target call must survive codegen as an exact typed dependency",
    );

    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");
    assert_eq!(eval_ok(&mut vm, "caller"), "OLD");

    eval_ok(&mut vm, "rename target {}; proc target {} {return NEW}");
    assert_eq!(
        eval_ok(&mut vm, "caller"),
        "NEW",
        "the stale inlined body must deoptimise to the live procedure binding",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "{source}; puts [caller]; rename target {{}}; proc target {{}} {{return NEW}}; puts [caller]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            "OLD\nNEW",
            "Tcl 9.0.4 observes the redefined procedure",
        );
    }
}

#[test]
fn inlined_bare_procedure_call_revalidates_resolution_after_local_shadow() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = concat!(
        "namespace eval ::N {}\n",
        "proc target {} {return GLOBAL}\n",
        "proc ::N::caller {} {target}",
    );
    let _compiler_owner = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let (module, ir) = InliningCompilerSvc::compile_script_with_ir(
        ScriptCompileTarget {
            source,
            namespace: "",
            evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
        },
        profile,
    )
    .expect("explicit inline pipeline compiles");
    assert_eq!(
        compiled_body_at_declaration(
            &ir,
            &module,
            ir.top_level.statements.len() - 1,
            "::N::caller"
        )
        .procedure_bindings,
        [tcl_runtime_api::ProcedureBindingIdentity::in_namespace(
            "N",
            "target",
            "::target",
            "",
            "return GLOBAL",
        )],
        "the erased call must retain its source resolution and selected definition",
    );

    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");
    assert_eq!(eval_ok(&mut vm, "N::caller"), "GLOBAL");

    eval_ok(&mut vm, "proc ::N::target {} {return LOCAL}");
    assert_eq!(
        eval_ok(&mut vm, "N::caller"),
        "LOCAL",
        "a local shadow must invalidate the caller's inlined global target",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "{source}; puts [N::caller]; proc ::N::target {{}} {{return LOCAL}}; puts [N::caller]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            "GLOBAL\nLOCAL",
            "Tcl 9.0.4 resolves the newly-created namespace-local procedure",
        );
    }
}

#[test]
fn inlined_namespaced_body_revalidates_commands_in_its_defining_namespace() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = concat!(
        "namespace eval ::n {}\n",
        "proc ::n::callee {} {expr {40 + 2}}\n",
        "proc caller {} {::n::callee}\n",
    );
    let _compiler_owner = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let (module, ir) = InliningCompilerSvc::compile_script_with_ir(
        ScriptCompileTarget {
            source,
            namespace: "",
            evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
        },
        profile,
    )
    .expect("explicit inline pipeline compiles");
    let caller =
        compiled_body_at_declaration(&ir, &module, ir.top_level.statements.len() - 1, "::caller");
    assert!(
        caller
            .procedure_bindings
            .iter()
            .any(|binding| binding.name == "::n::callee"),
        "the caller must contain the callee's inlined body",
    );
    assert!(
        caller.command_bindings.iter().any(|binding| {
            binding.resolution_namespace == "n"
                && binding.name == "expr"
                && binding.identity == "expr"
        }),
        "the copied expr specialisation must retain the callee's defining namespace",
    );

    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");
    assert_eq!(eval_ok(&mut vm, "caller"), "42");

    eval_ok(&mut vm, "proc ::n::expr args {return NAMESPACE_OVERRIDE}");
    assert_eq!(
        eval_ok(&mut vm, "caller"),
        "NAMESPACE_OVERRIDE",
        "a local command shadow must invalidate the copied namespaced binding",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "{source}; puts [caller]; proc ::n::expr args {{return NAMESPACE_OVERRIDE}}; puts [caller]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            "42\nNAMESPACE_OVERRIDE",
            "Tcl 9.0.4 resolves the newly-created local expr binding",
        );
    }
}

const INLINED_NAMESPACED_BOUNDARY_SOURCE: &str = concat!(
    "namespace eval ::n {}\nnamespace eval ::m {}\nnamespace eval ::e {}\n",
    "proc mutate_n {name1 name2 op} {",
    "proc ::n::expr args {return N_OVERRIDE}}\n",
    "proc mutate_m {name1 name2 op} {",
    "proc ::m::expr args {return M_OVERRIDE}}\n",
    "proc mutate_e {name1 name2 op} {",
    "proc ::e::expr args {error E_FAILURE}}\n",
    "set ::trigger_n {}; trace add variable ::trigger_n read mutate_n\n",
    "set ::trigger_m {}; trace add variable ::trigger_m read mutate_m\n",
    "set ::trigger_e {}; trace add variable ::trigger_e read mutate_e\n",
    "proc ::n::callee {} {",
    "::llength $::trigger_n; ::puts -nonewline {}; expr {40 + 2}}\n",
    "proc ::m::callee {} {",
    "::llength $::trigger_m; ::puts -nonewline {}; expr {40 + 2}}\n",
    "proc ::e::callee {} {",
    "::llength $::trigger_e; ::puts -nonewline {}; expr {40 + 2}}\n",
    "proc caller_n {} {::n::callee}\n",
    "proc caller_m {} {::m::callee}\n",
    "proc caller_e {} {::e::callee}\n",
);

#[test]
fn active_inlined_namespaced_boundary_replays_in_its_defining_namespace() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = INLINED_NAMESPACED_BOUNDARY_SOURCE;
    let _compiler_owner = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let (module, ir) = InliningCompilerSvc::compile_script_with_ir(
        ScriptCompileTarget {
            source,
            namespace: "",
            evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
        },
        profile,
    )
    .expect("explicit inline pipeline compiles");
    for (index, (caller_name, callee_name, namespace)) in [
        ("::caller_n", "::n::callee", "n"),
        ("::caller_m", "::m::callee", "m"),
        ("::caller_e", "::e::callee", "e"),
    ]
    .into_iter()
    .enumerate()
    {
        let caller = compiled_body_at_declaration(
            &ir,
            &module,
            ir.top_level.statements.len() - 3 + index,
            caller_name,
        );
        assert!(
            caller
                .procedure_bindings
                .iter()
                .any(|binding| binding.name == callee_name),
            "{caller_name} must contain the callee's inlined body",
        );
        let expr_boundary = caller
            .instructions
            .iter()
            .find(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.source_cmd_text == "expr {40 + 2}"
            })
            .expect("the inlined expr has a stale-command replay boundary");
        assert_eq!(
            expr_boundary.source_command_namespace.as_segments(),
            [tcl_core_types::NameBytes::from(namespace)].as_slice(),
            "the replay target must retain the callee's constructed namespace",
        );
    }

    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");
    assert_eq!(eval_ok(&mut vm, "caller_n"), "N_OVERRIDE");
    assert_eq!(
        eval_ok(&mut vm, "namespace current"),
        "::",
        "successful replay must restore the caller's namespace",
    );
    assert_eq!(
        eval_ok(&mut vm, "caller_m"),
        "M_OVERRIDE",
        "same-source replays in distinct inlined namespaces must not share a plain cache entry",
    );
    assert_eq!(
        eval_ok(&mut vm, "catch {caller_e} replay_error; set replay_error"),
        "E_FAILURE",
    );
    assert_eq!(
        eval_ok(&mut vm, "namespace current"),
        "::",
        "error unwind from a replay must restore the caller's namespace",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "{source}; puts [caller_n]; puts [caller_m]; catch {{caller_e}} replay_error; puts $replay_error; puts [namespace current]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            "N_OVERRIDE\nM_OVERRIDE\nE_FAILURE\n::",
            "Tcl 9.0.4 re-resolves expr in the callee's defining namespace",
        );
    }
}

#[test]
fn namespaced_boundary_replay_preserves_the_frame_namespace_pairing() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = concat!(
        "namespace eval ::n {}\n",
        "proc pick {} {return GLOBAL}\n",
        "proc ::n::pick {} {return NAMESPACE}\n",
        "proc replay_worker args {uplevel 1 {pick}}\n",
        "proc mutate {name1 name2 op} {",
        "interp alias {} ::n::expr {} ::replay_worker}\n",
        "set ::trigger {}; trace add variable ::trigger read mutate\n",
        "proc ::n::callee {} {",
        "llength $::trigger; puts -nonewline {}; expr {40 + 2}}\n",
        "proc caller {} {::n::callee}\n",
    );
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");

    // The stale inlined boundary runs in ::n without adding a Tcl call frame.
    // Its alias target's `uplevel 1` must therefore observe ::n, just as Tcl
    // 9.0.4 does for the non-inlined ::n::callee frame.
    assert_eq!(eval_ok(&mut vm, "caller"), "NAMESPACE");
    assert_eq!(eval_ok(&mut vm, "namespace current"), "::");
}

#[test]
fn top_level_cross_namespace_call_keeps_its_proc_frame_and_matches_tcl90() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let body = "llength $::trigger; puts -nonewline {}; expr {40 + 2}";
    let source = format!("proc ::n::callee {{}} {{{body}}}\n::n::callee\n");
    let service = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let module = service
        .compile_for_profile(&source, profile)
        .expect("top-level inline pipeline compiles");
    assert!(
        module
            .top_level
            .procedure_bindings
            .iter()
            .all(|binding| binding.name != "::n::callee"),
        "a cross-namespace top-level call must keep its Tcl procedure frame",
    );

    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(
        eval_ok(
            &mut vm,
            &format!(
                "namespace eval ::n {{}}\n\
                 proc ::n::callee {{}} {{{body}}}\n\
                 proc replay_worker args {{\
                     set ::replay_observed [uplevel #0 {{namespace current}}]; c\
                 }}\n\
                 proc mutate {{name1 name2 op}} {{\
                     interp alias {{}} ::n::expr {{}} ::replay_worker\
                 }}\n\
                 set ::replay_observed UNTOUCHED\n\
                 set ::trigger {{}}\n\
                 trace add variable ::trigger read mutate\n\
                 coroutine c eval {{yield READY; namespace current}}"
            ),
        ),
        "READY",
    );

    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(
        completion.result.to_str().as_ref(),
        "::",
        "the replacement command runs from ::n::callee's retained procedure frame",
    );
    assert_eq!(
        eval_ok(
            &mut vm,
            "list $::replay_observed [info commands c] [uplevel #0 {namespace current}]",
        ),
        ":: {} ::",
        "the replacement observes the global root, consumes the coroutine, and restores global context",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "namespace eval ::n {{}}\n\
             proc ::n::callee {{}} {{{body}}}\n\
             proc replay_worker args {{\
                 set ::replay_observed [uplevel #0 {{namespace current}}]; c\
             }}\n\
             proc mutate {{name1 name2 op}} {{\
                 interp alias {{}} ::n::expr {{}} ::replay_worker\
             }}\n\
             set ::replay_observed UNTOUCHED\n\
             set ::trigger {{}}\n\
             trace add variable ::trigger read mutate\n\
             coroutine c eval {{yield READY; namespace current}}\n\
             set result [::n::callee]\n\
             puts [list $result $::replay_observed [info commands c] \
                 [uplevel #0 {{namespace current}}]]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            ":: :: {} ::",
            "Tcl 9.0.4 retains the callee frame and global root",
        );
    }
}
