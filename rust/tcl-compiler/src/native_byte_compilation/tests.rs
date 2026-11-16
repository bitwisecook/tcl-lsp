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
use tcl_core_types::{ByteNamespacePath, NameBytes, NativeByteCommandSlot};
use tcl_lexer::{Lexer, LexerConfig, group_commands_bytes};
use tcl_runtime_api::native_compilation::{
    NativeCommandImplementation, NativeCompilationBinding, NativeCompilationFrame,
    NativeCompilationNamespace, NativeCompilerHookPresence, NativeInterpreterIdentity,
    NativeVariableObserverPresence,
};

mod ensemble_workers;
mod generic_certificate;
mod namespace_workers;
mod private_declines;
mod registered_commands;
mod worker_selection;

fn entry(version: tcl_dialect::TclVersion) -> NativeCompilationEntry {
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    let point = tcl_dialect::model::DialectPoint::for_tcl_version(version);
    NativeCompilationEntry {
        interpreter: NativeInterpreterIdentity {
            owner: 7,
            interpreter: 0,
        },
        epoch: 3,
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
        source_string_protocol: tcl_registry::InvocationDialect::of_point(point)
            .native_source_string_protocol(),
        lexer_grammar: Some(profile.grammar),
        inline_compilation_disabled: false,
        direct_evaluation: false,
        authored_tmm_static: None,
        namespace_variable_tables: None,
        empty_literal_world: None,
        compiler_pass_environment: None,
        command_resolvers: None,
        variable_observers: NativeVariableObserverPresence::Unknown,
        math_functions: None,
        closed: true,
        commands: vec![].into(),
        namespaces: vec![namespace(1, ByteNamespacePath::root(), true)],
        current_namespace: 1,
        frame: NativeCompilationFrame::Global,
    }
}

fn namespace(token: u64, path: ByteNamespacePath, visible: bool) -> NativeCompilationNamespace {
    NativeCompilationNamespace {
        path,
        token,
        visible,
        jim_namespace_object: None,
        exports: vec![],
        command_path: vec![],
        unknown_handler: None,
    }
}

fn binding(
    namespace: &NativeCompilationNamespace,
    name: &[u8],
    hook: NativeCompilerHookPresence,
) -> NativeCompilationBinding {
    NativeCompilationBinding {
        slot: NativeByteCommandSlot::new(namespace.path.clone(), NameBytes::from(name)),
        namespace_token: namespace.token,
        token: 11,
        implementation_generation: 4,
        implementation: NativeCommandImplementation::Opaque,
        compiler_hook: hook,
        compiler: None,
        procedure_header: None,
        has_execution_trace: false,
    }
}

fn install_original_target_fixture(entry: &mut NativeCompilationEntry) {
    let mut identity = 1;
    entry.ensemble_target_objects = Some(entry.commands.iter().flat_map(|command| {
            command.compiler.as_ref().and_then(|compiler| compiler.ensemble.as_ref()).into_iter().flat_map(move |configuration| {
                configuration.map.iter().flat_map(move |(member, prefix)| prefix.iter().enumerate().map(move |(position, name)| (command.token, member.clone(), position, name.clone())))
            })
        }).map(|(token, member, position, name)| {
            let observation = tcl_runtime_api::native_compilation::NativeEnsembleTargetObservation {
                ensemble_token: token, member, prefix_index: position, object_identity: identity,
                resident_name: name.clone(), primary: if name.is_some() { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::StockString } else { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::Unavailable },
            };
            identity += 1; observation
        }).collect());
}

fn source_module(source: &[u8], entry: &NativeCompilationEntry) -> crate::ir::Module {
    let image = SourceImage::native(source);
    let config = LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
    let tokens = Lexer::with_source_image(&image, config)
        .tokenise_all()
        .unwrap();
    let statements = group_commands_bytes(&tokens, image.bytes(), config)
        .into_iter()
        .map(|group| crate::ir::Statement::NativeCall {
            span: group.span,
            words: group
                .words
                .iter()
                .map(|word| NativeWord::from_group(image.clone(), config, &tokens, word).unwrap())
                .collect(),
        })
        .collect();
    let mut module = crate::ir::Module {
        source: image,
        native_namespace: Some(
            entry
                .namespaces
                .iter()
                .find(|row| row.token == entry.current_namespace)
                .unwrap()
                .path
                .clone(),
        ),
        ..crate::ir::Module::default()
    };
    module.top_level.statements = statements;
    module
}
