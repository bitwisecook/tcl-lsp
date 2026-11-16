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

use tcl_compiler::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

use super::*;
use tcl_compiler::lowering::{
    lower_proc_body_module_for_bytecode, lower_to_ir_for_bytecode_with_dialect,
};

mod command_boundaries;
mod command_plans;
mod dispatch_modes;
mod entered_source;
mod entry_policy;
mod procedure_targets;
mod profiles;
mod retained_bindings;
mod word_compilation;

fn policy_entry(
    profile: &'static DialectProfile,
    point: tcl_dialect::model::DialectPoint,
) -> tcl_runtime_api::NativeCompilationEntry {
    use tcl_runtime_api::native_compilation::{
        NativeCompilationEntry, NativeCompilationFrame, NativeInterpreterIdentity,
    };
    NativeCompilationEntry {
        interpreter: NativeInterpreterIdentity {
            owner: 31,
            interpreter: 0,
        },
        epoch: 4,
        profile: profile.cache_key(),
        invocation_policy: Some(profile.cache_key()),
        expression_policy:
            tcl_registry::native_expression_program::native_expression_evaluation_policy(
                profile, point,
            ),
        execution_point: Some(point),
        execution_name_policy: None,
        name_protocol: tcl_syntax::naming::NamePolicyProtocol::for_native_point(point),
        compiled_variable_protocol:
            tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(point),
        compiled_local_layout: None,
        oo_classes: None,
        ensemble_target_objects: None,
        source_string_protocol: tcl_registry::InvocationDialect::of_profile(profile)
            .native_source_string_protocol(),
        lexer_grammar: None,
        inline_compilation_disabled: false,
        direct_evaluation: false,
        authored_tmm_static: None,
        namespace_variable_tables: None,
        empty_literal_world: None,
        compiler_pass_environment: None,
        command_resolvers: None,
        variable_observers:
            tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Unknown,
        math_functions: None,
        closed: false,
        commands: Vec::new().into(),
        namespaces: Vec::new(),
        current_namespace: 0,
        frame: NativeCompilationFrame::Unknown,
    }
}

fn opaque_set_namespace_entry(
    profile: &tcl_dialect::DialectProfile,
    namespace: tcl_core_types::ByteNamespacePath,
) -> tcl_runtime_api::native_compilation::NativeCompilationEntry {
    use tcl_runtime_api::native_compilation::{
        NativeCommandImplementation, NativeCompilationBinding, NativeCompilationEntry,
        NativeCompilationFrame, NativeCompilationNamespace, NativeInterpreterIdentity,
    };
    NativeCompilationEntry {
        interpreter: NativeInterpreterIdentity {
            owner: 31,
            interpreter: 0,
        },
        epoch: 4,
        profile: profile.cache_key(),
        invocation_policy: Some(profile.cache_key()),
        expression_policy: None,
        execution_point: tcl_registry::InvocationDialect::of_profile(profile).core_point,
        execution_name_policy: None,
        name_protocol: tcl_registry::InvocationDialect::of_profile(profile)
            .core_point
            .and_then(tcl_syntax::naming::NamePolicyProtocol::for_native_point),
        compiled_variable_protocol: tcl_registry::InvocationDialect::of_profile(profile)
            .native_compiled_variable_protocol(),
        compiled_local_layout: None,
        oo_classes: None,
        ensemble_target_objects: None,
        source_string_protocol: tcl_registry::InvocationDialect::of_profile(profile)
            .native_source_string_protocol(),
        lexer_grammar: None,
        inline_compilation_disabled: false,
        direct_evaluation: false,
        authored_tmm_static: None,
        namespace_variable_tables: None,
        empty_literal_world: None,
        compiler_pass_environment: None,
        command_resolvers: None,
        variable_observers:
            tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Unknown,
        math_functions: None,
        closed: true,
        commands: vec![NativeCompilationBinding {
            slot: tcl_core_types::NativeByteCommandSlot::new(namespace.clone(), "set".into()),
            namespace_token: 1,
            token: 19,
            implementation_generation: 19,
            implementation: NativeCommandImplementation::Opaque,
            compiler_hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
            has_execution_trace: false,
            compiler: None,
            procedure_header: None,
        }]
        .into(),
        namespaces: vec![NativeCompilationNamespace {
            path: namespace,
            jim_namespace_object: None,
            token: 1,
            visible: true,
            exports: Vec::new(),
            command_path: Vec::new(),
            unknown_handler: None,
        }],
        current_namespace: 1,
        frame: NativeCompilationFrame::Namespace,
    }
}

/// The registry a profile-backed service compiles `profile` against.
fn profile_registry<'a>(
    service: &'a BytecodeCompileService,
    profile: &'static DialectProfile,
) -> ProfileRegistry<'a> {
    service
        .registry
        .registry_for_profile(profile)
        .expect("a profile-backed registry")
}

fn registry_with_custom_list_expr_hook() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    let mut custom = registry.get("list").expect("list spec").clone();
    // SpecTcl overrides are commonly profile-independent authored rows.
    // They must still replace the core row after an explicit-profile
    // compiler projection.
    custom.surface = None;
    custom.lowering_hook = Some(tcl_registry::hooks::LoweringHookId::Expr);
    custom.inline_codegen_hook = Some(tcl_registry::hooks::InlineCodegenHookId::Expr);
    registry.insert(custom);
    registry
}

fn instructions_without_source_spans(
    instructions: &[tcl_bytecode::Instruction],
) -> Vec<tcl_bytecode::Instruction> {
    // Direct procedure targets start at zero; enclosing modules use source
    // coordinates. All other codegen facts must remain equivalent.
    let mut instructions = instructions.to_vec();
    for instruction in &mut instructions {
        instruction.source_span = None;
        for operation in &mut instruction.native_operation_selections {
            operation.span = tcl_lexer::Span::new(0, operation.span.len());
        }
    }
    instructions
}

const PROCEDURE_TARGET_MATRIX_BODY: &str = "namespace import ::source::*\n\
                    namespace export exposed\n\
                    interp alias {} mirror {} append\n\
                    mirror value $suffix\n\
                    rename mirror appended\n\
                    appended value !\n\
                    set child_body {return child}\n\
                    proc child {} $child_body\n\
                    return $value";
