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

#[test]
fn inline_lowering_declines_unavailable_scope_and_retained_unknown_source() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["target"], &registry);
    assert_eq!(
        ctx.inline_lowering_hook("set", &["target", "value"])
            .unwrap()
            .0,
        tcl_registry::hooks::LoweringHookId::Set
    );
    ctx.resolution_namespace.key = None;
    assert!(
        ctx.inline_lowering_hook("set", &["target", "value"])
            .is_none()
    );
    let mut ctx = CodegenCtx::new(true, &["target"], &registry);
    let mut tokens = ctx.original_inline_syntax("set target value").unwrap();
    tokens.source_binding = Some(tcl_compiler::command_binding::SourceInvocationBinding::unknown());
    ctx.with_invocation_tokens(Some(&tokens), |ctx| {
        assert!(
            ctx.inline_lowering_hook("set", &["target", "value"])
                .is_none()
        );
        assert!(
            ctx.inline_lowering_hook("set", &["other", "value"])
                .is_none()
        );
        assert!(ctx.command_binding_requirements.is_empty());
    });
}

fn fixed_preparation(
    generation: u64,
) -> tcl_compiler::command_binding::SourceExpressionPreparation {
    use std::sync::Arc;
    use tcl_runtime_api::native_compilation::{
        NativeInterpreterIdentity, NativeMathFunctionBinding, NativeMathFunctionPrerequisite,
        NativeMathFunctionTable,
    };
    let source = "0 && abs(1)";
    let origin = Arc::new(tcl_compiler::command_binding::SourceOriginId::authored(
        &Arc::from(source),
    ));
    let source = Arc::new(
        tcl_compiler::command_binding::ExecutedScriptSource::contiguous(
            Arc::clone(&origin),
            source,
            0,
        )
        .unwrap(),
    );
    let table = NativeMathFunctionPrerequisite {
        interpreter: NativeInterpreterIdentity {
            owner: 17,
            interpreter: 3,
        },
        table: NativeMathFunctionTable {
            closed: true,
            generation,
            functions: vec![NativeMathFunctionBinding {
                name: "abs".into(),
                token: 1,
                implementation_generation: 1,
                registry_identity: Some("abs".into()),
                arity: Some(1),
            }],
        },
    };
    let profile = tcl_registry::model::ingress::static_context_for("jim")
        .commands()
        .profile()
        .unwrap();
    let context = tcl_native_model::InvocationDialect::of_profile(profile)
        .expression_parse_context(Some(profile));
    let tcl_registry::runtime_expr_validation::ExpressionPreparationProof::Prepared(witness) =
        tcl_registry::runtime_expr_validation::prepare_expression_witness(
            source.try_text().unwrap(),
            &context,
            Some(&table),
        )
    else {
        panic!("actual closed fixed table must prepare expression");
    };
    tcl_compiler::command_binding::SourceExpressionPreparation {
        namespace_key: tcl_compiler::command_binding::SourceNamespaceKey::authored("::"),
        invocation: tcl_compiler::command_binding::CommandAllocationSite {
            source: origin,
            offset: 0,
        },
        source,
        witness: Arc::from(witness),
        script_compilation: None,
        executed_expression: None,
    }
}

#[test]
fn preparation_guard_survives_without_reached_math_calls() {
    let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let mut ctx = CodegenCtx::new(false, &[], registry);
    ctx.dialect = registry.profile();
    ctx.numbers = registry.numbers();
    let proof = fixed_preparation(4);
    assert!(ctx.retain_expression_preparations(std::slice::from_ref(&proof)));
    assert_eq!(
        ctx.math_table_prerequisite.as_ref(),
        proof.witness.fixed_functions()
    );
    assert!(ctx.command_binding_requirements.is_empty());
    let incompatible = fixed_preparation(5);
    assert!(!ctx.retain_expression_preparations(&[incompatible]));
    assert_eq!(
        ctx.math_table_prerequisite.as_ref(),
        proof.witness.fixed_functions()
    );
}

#[test]
fn preparation_guard_declines_a_different_native_engine() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut ctx = CodegenCtx::new(false, &[], registry);
    ctx.dialect = registry.profile();
    ctx.numbers = registry.numbers();
    assert!(!ctx.retain_expression_preparations(&[fixed_preparation(4)]));
    assert!(ctx.math_table_prerequisite.is_none());
    assert!(ctx.command_binding_requirements.is_empty());
}

#[test]
fn script_preparation_retains_chunk_entry_guards_atomically() {
    use tcl_compiler::command_binding::{ExecutedScriptSource, SourceCommandBindings};
    use tcl_runtime_api::CommandBindingGuard;

    let registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
    let source = "set x 4; expr {[set x] + 0}";
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        registry,
        tcl_compiler::command_binding::SourceAnalysisOptions {
            invocation_dialect: registry
                .profile()
                .map(tcl_native_model::InvocationDialect::of_profile),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let script = ExecutedScriptSource::contiguous(
        std::sync::Arc::clone(bindings.source_origin().unwrap()),
        source,
        0,
    )
    .unwrap();
    let proofs = bindings.expression_preparations_for_script(&script);
    assert_eq!(proofs.len(), 1);
    assert_ne!(proofs[0].witness.compiled_scripts(), []);
    let mut ctx = CodegenCtx::new(false, &[], registry);
    ctx.dialect = registry.profile();
    ctx.numbers = registry.numbers();
    assert!(ctx.retain_expression_preparations(&proofs));
    assert!(ctx.command_binding_requirements.iter().any(|binding| {
        binding.identity == "set" && binding.guard == CommandBindingGuard::ChunkEntry
    }));
    let retained = ctx.command_binding_requirements.clone();
    let mut missing = proofs[0].clone();
    missing.script_compilation = None;
    assert!(!ctx.retain_expression_preparations(&[missing]));
    assert_eq!(ctx.command_binding_requirements, retained);
}

#[test]
fn native_source_boundaries_keep_opaque_bytes_and_constructed_namespace() {
    let registry = CommandRegistry::build_default();
    let namespace =
        tcl_runtime_api::ByteNamespacePath::from_segments([b"\xff".as_slice(), b":".as_slice()]);
    let source = tcl_lexer::SourceImage::native(b"set \xfe V".as_slice());
    let mut context = CodegenCtx::with_native_parameters(
        true,
        &[tcl_runtime_api::NameBytes::from(b"\xfe")],
        &registry,
    );
    context.set_source_image(source.clone());
    context.set_resolution_namespace_path(namespace.clone());
    assert!(context.resolution_namespace().is_none());
    assert_eq!(context.lvt.entries()[0].as_bytes(), b"\xfe");
    context.set_command_source_span(Span::new(0, 7));
    let index = context.emit(Op::NOP, Vec::new());
    assert_eq!(context.instructions[index].source_cmd_text, source);
    assert_eq!(
        context.instructions[index].source_command_namespace,
        namespace
    );
}

#[test]
fn source_lines_are_indexed_at_byte_boundaries() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.set_source("alpha\nβeta\n\ngamma");

    for (offset, expected) in [(0, 1), (5, 1), (6, 2), (11, 2), (12, 3), (13, 4)] {
        assert_eq!(ctx.source_line(Span::empty(offset)), expected, "{offset}");
    }

    assert_eq!(ctx.source_line(Span::empty(99)), 1);

    ctx.current_span = Some(Span::empty(13));
    let instruction = ctx.emit(Op::NOP, Vec::new());
    assert_eq!(ctx.instructions[instruction].source_line, 4);

    let mut empty = CodegenCtx::new(false, &[], &registry);
    assert_eq!(empty.source_line(Span::empty(0)), 0);
    empty.current_span = Some(Span::empty(0));
    let instruction = empty.emit(Op::NOP, Vec::new());
    assert_eq!(empty.instructions[instruction].source_line, 1);

    let source = tcl_lexer::SourceImage::document("shared\nsource");
    let source_clone = source.shared_bytes();
    let line_index = tcl_lexer::LineIndex::from_bytes(source.bytes());
    let line_index_clone = line_index.clone();
    let mut shared = CodegenCtx::new(false, &[], &registry);
    shared.set_indexed_source(source, line_index);
    assert!(std::sync::Arc::ptr_eq(
        &shared.source.shared_bytes(),
        &source_clone
    ));
    assert!(
        shared
            .line_index
            .as_ref()
            .is_some_and(|index| index.shares_storage_with(&line_index_clone)),
    );
}
