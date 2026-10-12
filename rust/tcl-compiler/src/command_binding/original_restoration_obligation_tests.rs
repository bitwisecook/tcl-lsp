// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source restoration retains compiler obligations, never admission.

use super::*;
use crate::analyser::ResolvedAnalysisInput;
use crate::ir::Module;
use crate::native_compilation_admission::{
    NativeCompilationAdmissionPlan, NativeCompilationAdmissionScope,
    script_admission_plan,
};

fn native_module(
    name: &str,
    source: &str,
) -> (crate::environment_ingress::CapturedNativeEntry, ResolvedAnalysisInput, Module) {
    let environment = tcl_registry::model::ingress::resolve_known_environment(name).unwrap();
    let profile = environment.unit_profile();
    let input = ResolvedAnalysisInput::new(
        profile, profile, environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let held = crate::environment_ingress::captured_native_entry(profile);
    let mut lowerer = crate::lowering::Lowerer::with_config(
        input.borrowed_context_registry().commands(), input.lexer_config(),
    ).with_resolved_analysis_input(input.clone());
    lowerer.set_source_analysis_options(SourceAnalysisOptions {
        native_entry: Some(held.as_entry()),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
            loop_depth: 0,
            catch_depth: Some(0),
        },
        ..Default::default()
    });
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    (held, input, module)
}

fn positioned<'a>(module: &'a Module, input: &ResolvedAnalysisInput) -> &'a SourceCommandBindings {
    module.retained_source_bindings.as_ref().unwrap()
        .positioned_for_module(module, input.borrowed_context_registry().commands()).unwrap()
}

#[test]
fn original_native_restoration_retains_required_script_and_procedure_provider_plans() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    // Original held source/metadata only; no compiler or successful execution grant.
    let (_held, input, module) = native_module(
        "tcl8.6", "puts [list VALUE]; foreach item [list VALUE] {}",
    );
    let owner = positioned(&module, &input);
    let mut restored = module.top_level.clone();
    let original = restored.native_compilation_admission.as_ref().unwrap().clone();
    assert!(original.requires_native_provider());
    let source = original.source.as_ref().unwrap();
    assert!(owner.restore_script_bindings(&mut restored));
    assert!(Arc::ptr_eq(restored.native_compilation_admission.as_ref().unwrap(), &original));
    assert_eq!(restored.executed_source.as_deref(), Some(source));
    assert_eq!(
        script_admission_plan(&restored, NativeCompilationAdmissionScope::Script),
        NativeCompilationAdmissionPlan::HostScript(source),
    );
    assert_eq!(
        script_admission_plan(&restored, NativeCompilationAdmissionScope::ProcedureBody),
        NativeCompilationAdmissionPlan::HostProcedure(source),
    );
    assert!(crate::native_compilation_admission::script_requires_admission(&restored));
    assert!(Arc::ptr_eq(
        &crate::native_compilation_admission::retained_script_admission(&restored).unwrap(),
        &original,
    ));
}

#[test]
fn original_native_restoration_refuses_missing_changed_and_foreign_obligations_without_repair() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let (_held, input, module) = native_module(
        "tcl8.6", "puts [list VALUE]; foreach item [list VALUE] {}",
    );
    let owner = positioned(&module, &input);
    let (_foreign_held, _foreign_input, foreign) = native_module(
        "tcl8.6", "puts [list FOREIGN]; foreach item [list FOREIGN] {}",
    );
    for axis in 0..4 {
        let mut restored = module.top_level.clone();
        match axis {
            0 => restored.native_compilation_admission = None,
            1 => Arc::make_mut(restored.native_compilation_admission.as_mut().unwrap()).source = None,
            2 => Arc::make_mut(restored.native_compilation_admission.as_mut().unwrap()).provider_required = false,
            3 => Arc::make_mut(restored.native_compilation_admission.as_mut().unwrap()).source =
                foreign.top_level.native_compilation_admission.as_ref().and_then(|admission| admission.source.clone()),
            _ => unreachable!(),
        }
        let obligation = restored.native_compilation_admission.clone();
        assert!(!owner.restore_script_bindings(&mut restored), "axis {axis}");
        assert_eq!(restored.native_compilation_admission, obligation, "axis {axis}");
    }
}

#[test]
fn original_native_restoration_requires_full_failure_contexts_and_dependency_generations() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    // A native compiler failure stays an entry obligation after metadata restoration.
    let (_held, input, module) = native_module(
        "tcl8.4", "set flag 0; set before 1; if {$flag} {set}",
    );
    let owner = positioned(&module, &input);
    let original = module.top_level.native_compilation_admission.as_ref().unwrap();
    let failure = original.failure.as_ref().unwrap();
    assert!(!failure.dependencies.is_empty());
    assert!(!failure.contexts.is_empty());
    let mut unchanged = module.top_level.clone();
    assert!(owner.restore_script_bindings(&mut unchanged));
    assert_eq!(unchanged.native_compilation_admission.as_ref(), Some(original));
    assert!(crate::native_compilation_admission::script_requires_admission(&unchanged));
    for axis in 0..4 {
        let mut restored = module.top_level.clone();
        let admission = Arc::make_mut(restored.native_compilation_admission.as_mut().unwrap());
        match axis {
            0 => admission.failure = None,
            1 => Arc::make_mut(admission.failure.as_mut().unwrap()).dependencies.clear(),
            2 => Arc::make_mut(admission.failure.as_mut().unwrap()).contexts[0].line_in_chunk = Some(u32::MAX),
            3 => restored.native_compilation_failure = None,
            _ => unreachable!(),
        }
        let obligation = restored.native_compilation_admission.clone();
        let old_failure = restored.native_compilation_failure.clone();
        assert!(!owner.restore_script_bindings(&mut restored), "axis {axis}");
        assert_eq!(restored.native_compilation_admission, obligation);
        assert_eq!(restored.native_compilation_failure, old_failure);
    }
}
