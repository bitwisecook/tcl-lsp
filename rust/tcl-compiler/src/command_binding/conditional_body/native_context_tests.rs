// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::command_binding::{SourceAnalysisOptions, SourceNamespaceKey};
use crate::var_resolve::{ResolveContext, VariableExecutionFrame};

fn native_bindings(source: &str) -> (tcl_vm::Vm, SourceCommandBindings) {
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = owner.commands();
    let profile = registry.profile().unwrap();
    let (runtime, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        registry,
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                loop_depth: 0,
                catch_depth: Some(0),
            },
            ..Default::default()
        },
    );
    (runtime, bindings)
}

#[test]
fn conditional_native_body_scope_matches_original_declaration_context() {
    let source = "namespace eval N {proc :f {} {set local 1}}";
    let (_owner, bindings) = native_bindings(source);
    let offset = u32::try_from(source.find("set local").unwrap()).unwrap();
    let origin = bindings.source_origin().unwrap();
    let entry = bindings.conditional_body_entry_at(origin, offset).unwrap();
    let original = bindings
        .procedure_implementation_bodies()
        .find(|body| body.source == entry.source())
        .unwrap();
    assert_eq!(entry.namespace_context(), Some(original.namespace_key));
    assert_eq!(
        entry.frame().namespace_identity(),
        Some(original.namespace_key)
    );
    assert_eq!(original.namespace_key.display().as_deref(), Some("::N"));
    let invocation = bindings.conditional_invocation_at_source(&entry, offset);
    assert!(invocation.invocation_site().is_some());
    assert!(entry.owns_invocation(&invocation));
    assert!(bindings.original_body_owns_context(&entry, &invocation.variable_context));
    assert!(!bindings.has_actual_procedure_entry(entry.source()));
}

#[test]
fn conditional_native_body_context_rejects_equal_display_incarnation() {
    let source = "namespace eval a: {namespace eval b {proc p {} {set local FIRST}}}; \
        namespace eval a {namespace eval :b {proc p {} {set local SECOND}}}";
    let (_owner, bindings) = native_bindings(source);
    let origin = bindings.source_origin().unwrap();
    let entries = ["set local FIRST", "set local SECOND"].map(|text| {
        bindings
            .conditional_body_entry_at(origin, u32::try_from(source.find(text).unwrap()).unwrap())
            .unwrap()
    });
    let keys = entries
        .each_ref()
        .map(|entry| entry.namespace_context().unwrap());
    assert_eq!(keys[0].display(), keys[1].display());
    assert_ne!(keys[0], keys[1]);
    let mut wrong = ResolveContext::default().in_frame(entries[0].frame());
    assert!(bindings.original_body_owns_context(&entries[0], &wrong));
    wrong.namespace_identity = Some(keys[1].clone());
    assert!(!bindings.original_body_owns_context(&entries[0], &wrong));
}

#[test]
fn receiver_declaration_key_does_not_supply_runtime_namespace() {
    let source = "namespace eval N {oo::class create C {method p {} {set local 1}}}";
    let (_owner, bindings) = native_bindings(source);
    let offset = u32::try_from(source.find("set local").unwrap()).unwrap();
    let origin = bindings.source_origin().unwrap();
    let entry = bindings
        .declared_receiver_body_entry_at(origin, offset)
        .unwrap();
    let original = bindings
        .deferred
        .values()
        .find(|body| {
            body.receiver_method
                && body
                    .executed_script
                    .as_ref()
                    .is_some_and(|(_, _, body_source)| body_source == entry.source())
        })
        .unwrap();
    assert_eq!(
        entry.declaration_namespace_context(),
        &original.namespace_key
    );
    assert!(entry.preview_frame().namespace_identity().is_none());
    let preview = ResolveContext::default().in_frame(entry.preview_frame());
    assert!(!preview.namespace_known);
    let binding = bindings.invocation_at_source("set", offset);
    assert_eq!(&binding.variable_frame, entry.preview_frame());
    assert!(!binding.variable_context.namespace_known);
    assert!(binding.variable_context.namespace_identity.is_none());
    assert!(bindings.declaration_layouts.values().flatten().any(|observation| {
        matches!(observation.entry.as_ref(), super::super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredReceiver(body) if body.as_ref() == entry.as_ref())
            && &observation.namespace == entry.declaration_namespace_context()
    }));
}

#[test]
fn root_declaration_scope_requires_actual_retained_global_context() {
    let (_owner, bindings) = native_bindings("set x 1");
    let root = bindings.final_state.source_root_namespace_key().unwrap();
    let mut state = bindings.final_state.as_ref().clone();
    state.variable_frame = VariableExecutionFrame::Global.with_namespace_identity(root.clone());
    assert_eq!(
        super::super::declaration_layout::root_diagnostic_namespace(&state, &root),
        Some(root.clone())
    );
    assert!(
        super::super::declaration_layout::root_diagnostic_namespace(
            &state,
            &SourceNamespaceKey::authored("::")
        )
        .is_none()
    );
    state.variable_frame = VariableExecutionFrame::Global;
    assert!(super::super::declaration_layout::root_diagnostic_namespace(&state, &root).is_none());
    assert!(bindings.declaration_layouts.values().flatten().any(|observation| {
        matches!(observation.entry.as_ref(), super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { frame, namespace, .. } if frame.namespace_identity() == Some(&root) && namespace == &root)
    }));
}

#[test]
fn restored_body_receipts_stay_in_original_namespace_context() {
    let source = "namespace eval a {expr {1 + 2}}; namespace eval b {expr {3 + 4}}";
    let bindings = SourceCommandBindings::analyse(
        source,
        tcl_lexer::LexerConfig::default(),
        &tcl_registry::CommandRegistry::build_default(),
    );
    let original =
        ExecutedScriptSource::contiguous(Arc::clone(bindings.source_origin().unwrap()), source, 0)
            .unwrap();
    let proofs = bindings.expression_preparations_for_script(&original);
    assert_eq!(proofs.len(), 2);
    assert_ne!(proofs[0].namespace_key, proofs[1].namespace_key);
    for proof in proofs {
        let mut script = crate::ir::Script::new();
        script.executed_source = Some(Arc::new(original.clone()));
        script.namespace_context = Some(Box::new(proof.namespace_key.clone()));
        assert!(bindings.restore_script_bindings(&mut script));
        assert_eq!(script.expression_preparations, vec![proof.clone()]);
        let selected = bindings
            .selected_source_in_context(&original, &proof.namespace_key)
            .unwrap();
        assert_eq!(
            script.implicit_math_invocations,
            selected.implicit_math_invocations_for_script(&original)
        );
        assert_eq!(
            script.namespace_context.as_deref(),
            Some(&proof.namespace_key)
        );
    }
}

#[test]
fn lambda_original_namespace_overrides_callers_context_key() {
    let source = "namespace eval other {proc helper {} {return OTHER}}; \
        proc helper {} {return ROOT}; apply {{} {helper} ::other}";
    let (_owner, bindings) = native_bindings(source);
    let offset = u32::try_from(source.find("apply").unwrap()).unwrap();
    let invocation = bindings.invocation_at_source("apply", offset);
    let site = invocation.invocation_site().unwrap();
    let body = bindings.executed_script_at(site, 0).unwrap();
    let namespace = bindings
        .executed_script_entry_namespace_context_at(site, 0, body)
        .unwrap();
    assert_eq!(namespace.display().as_deref(), Some("::other"));
    assert_ne!(
        Some(namespace),
        invocation.variable_frame.namespace_identity()
    );
    let entered = bindings.possible_entered_body_invocations(site);
    assert!(entered.iter().any(|binding| {
        binding
            .execution_targets()
            .any(|target| !target.registry_backed && target.command == "::other::helper")
    }));
    assert!(!entered.iter().any(|binding| {
        binding
            .execution_targets()
            .any(|target| !target.registry_backed && target.command == "::helper")
    }));
}
