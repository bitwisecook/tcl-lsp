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

//! Lowering one script or procedure body, from text or bytes, to a module for the bytecode backend.

use super::Lowerer;
use crate::command_binding::SourceAnalysisOptions;
use crate::ir::{Module, Statement};
use tcl_registry::CommandRegistry;

/// Lower one runtime procedure body as a complete IR module.
///
/// Unlike the ordinary module entry point, the outer source runs at procedure
/// depth in `namespace`. This preserves procedure-only lowering semantics and
/// retains any nested procedure definitions and whole-unit command-mutation
/// facts for bytecode emission.
#[must_use]
pub fn lower_proc_body_module_for_bytecode(
    body_text: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
) -> Module {
    lower_proc_body_module_for_bytecode_with_options(
        body_text,
        namespace,
        registry,
        config,
        dialect,
        plain_command_dispatch,
        None,
    )
}

/// Runtime lowering with an explicit live interpreter entry contract.
#[must_use]
pub fn lower_proc_body_module_for_bytecode_with_options(
    body_text: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Module {
    lower_procedure_target_module_for_bytecode_with_options(
        tcl_runtime_api::ProcedureCompileTarget {
            source: body_text,
            namespace,
            parameters: &[],
        },
        registry,
        config,
        dialect,
        plain_command_dispatch,
        options,
    )
}

/// Lower a runtime procedure with its actual formal input contract.
/// C formal names are ordinary slots. Name-only Jim headers cannot prove
/// reference/default binding kind, so they retain the live entry's uncertainty.
#[must_use]
pub fn lower_procedure_target_module_for_bytecode_with_options(
    target: tcl_runtime_api::ProcedureCompileTarget<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Module {
    let mut lowerer = Lowerer::with_config(registry, config)
        .with_dialect(dialect)
        .for_bytecode_backend();
    if let Some(options) = options {
        lowerer.set_source_analysis_options(options);
    }
    if plain_command_dispatch {
        lowerer = lowerer.trace_visible();
    }
    if lowerer
        .invocation_dialect
        .and_then(tcl_native_model::InvocationDialect::parameter_grammar)
        == Some(tcl_dialect::ParameterGrammar::Tcl)
    {
        lowerer.incoming_formals = target.parameters.to_vec();
    }
    lowerer.lower_procedure_target(target.source, target.namespace);
    lowerer.finish_module(target.source)
}

/// Lower one runtime script as a complete bytecode IR module in its exact
/// constructed command-resolution namespace.
#[must_use]
pub fn lower_script_module_for_bytecode(
    source: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
) -> Module {
    lower_script_module_for_bytecode_with_options(
        source,
        namespace,
        registry,
        config,
        dialect,
        plain_command_dispatch,
        None,
    )
}

/// Runtime lowering with an explicit live interpreter entry contract.
#[must_use]
pub fn lower_script_module_for_bytecode_with_options(
    source: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Module {
    let mut lowerer = Lowerer::with_config(registry, config)
        .with_dialect(dialect)
        .for_bytecode_backend();
    if let Some(options) = options {
        lowerer.set_source_analysis_options(options);
    }
    if plain_command_dispatch {
        lowerer = lowerer.trace_visible();
    }
    lowerer.lower_script_target(source, namespace);
    lowerer.finish_module(source)
}

/// Lower original native script operands through the shared byte lexical owner.
/// No Unicode projection supplies execution or compiler-admission authority.
///
/// # Errors
/// Returns the exact lexical operand capability failure.
pub fn lower_script_bytes_module_for_bytecode_with_options(
    target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    lower_native_target(
        NativeLoweringTarget {
            source: target.source,
            namespace: target.namespace,
            kind: crate::ir::TopLevelKind::Script,
            ordinary_parameters: None,
            original_request: None,
        },
        registry,
        config,
        dialect,
        plain_command_dispatch,
        options,
    )
}

/// Lower a native procedure body while retaining its exact namespace and source.
/// Formal binding is owned by the caller's original-object activation contract.
///
/// # Errors
/// Returns the exact lexical operand capability failure.
pub fn lower_procedure_bytes_module_for_bytecode_with_options(
    target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    lower_native_target(
        NativeLoweringTarget {
            source: target.source,
            namespace: target.namespace,
            kind: crate::ir::TopLevelKind::ProcedureBody,
            ordinary_parameters: None,
            original_request: None,
        },
        registry,
        config,
        dialect,
        plain_command_dispatch,
        options,
    )
}

/// Lower the immutable body request with its independently captured binding plan.
/// Unsupported formal bindings keep the legacy conservative variable treatment.
///
/// # Errors
/// Returns the original lexical operand capability failure.
pub fn lower_original_procedure_bytes_module_for_bytecode_with_options(
    request: &tcl_runtime_api::OriginalProcedureCompileRequest,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    mut options: Option<SourceAnalysisOptions<'_>>,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    let ordinary_parameters = options
        .as_ref()
        .and_then(|options| options.native_entry)
        .filter(|entry| request.matches_entry(entry))
        .and_then(|_| request.ordinary_formals());
    if let Some(options) = &mut options {
        options.incoming_formals = &[];
    }
    let target = request.target();
    lower_native_target(
        NativeLoweringTarget {
            source: target.source,
            namespace: target.namespace,
            kind: crate::ir::TopLevelKind::ProcedureBody,
            ordinary_parameters,
            original_request: Some(request),
        },
        registry,
        config,
        dialect,
        plain_command_dispatch,
        options,
    )
}

#[derive(Clone, Copy)]
struct NativeLoweringTarget<'a> {
    pub(super) source: &'a tcl_lexer::SourceImage,
    pub(super) namespace: &'a tcl_core_types::ByteNamespacePath,
    pub(super) kind: crate::ir::TopLevelKind,
    ordinary_parameters: Option<&'a [tcl_core_types::NameBytes]>,
    original_request: Option<&'a tcl_runtime_api::OriginalProcedureCompileRequest>,
}

fn lower_native_target(
    target: NativeLoweringTarget<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    let NativeLoweringTarget {
        source,
        namespace,
        kind,
        ordinary_parameters,
        original_request,
    } = target;
    let mut lowerer = Lowerer::with_config(registry, config)
        .with_dialect(dialect)
        .for_bytecode_backend();
    if let Some(options) = options {
        lowerer.set_source_analysis_options(options);
    }
    if let (Some(request), Some(entry)) = (original_request, lowerer.native_entry.as_ref()) {
        lowerer.original_native_compiler_formals =
            crate::command_binding::OriginalNativeCompilerFormals::capture(
                request,
                std::sync::Arc::clone(entry),
                lowerer.config,
            );
    }
    if plain_command_dispatch {
        lowerer = lowerer.trace_visible();
    }
    if kind == crate::ir::TopLevelKind::ProcedureBody
        && lowerer
            .invocation_dialect
            .and_then(tcl_native_model::InvocationDialect::parameter_grammar)
            == Some(tcl_dialect::ParameterGrammar::Tcl)
        && let Some(parameters) = ordinary_parameters
        && let Ok(names) = parameters
            .iter()
            .map(|name| name.try_utf8().map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()
    {
        lowerer.incoming_formals = names;
    }
    lowerer.set_source_image_channel(source);
    lowerer.module.native_namespace = Some(namespace.clone());
    if let (Ok(text), Some(namespace_key)) = (
        source.try_text(),
        lowerer.module.advisory_top_level_namespace(),
    ) {
        if tcl_source::segment::segment_commands_image_with_offset_and_config(
            source,
            0,
            lowerer.config,
        )
        .is_none()
        {
            return Err(tcl_lexer::word_parts::NativeWordError::LexicalStreamUnavailable);
        }
        let namespace_key = tcl_syntax::naming::unroot_rooted_key(&namespace_key)
            .expect("checked analytical namespace is rooted");
        match kind {
            crate::ir::TopLevelKind::Script => {
                lowerer.lower_script_target(text, namespace_key);
            }
            crate::ir::TopLevelKind::ProcedureBody => {
                lowerer.lower_procedure_target(text, namespace_key);
            }
        }
        let mut module = lowerer.finish_module(text);
        module.source = source.clone();
        module.native_namespace = Some(namespace.clone());
        return Ok(module);
    }
    lower_native_byte_calls(lowerer, source, namespace, registry, options, kind)
}

fn lower_native_byte_calls(
    mut lowerer: Lowerer<'_>,
    source: &tcl_lexer::SourceImage,
    namespace: &tcl_core_types::ByteNamespacePath,
    registry: &CommandRegistry,
    options: Option<SourceAnalysisOptions<'_>>,
    kind: crate::ir::TopLevelKind,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    lowerer.start_module();
    let config = lowerer.config;
    let tokens = tcl_lexer::Lexer::with_source_image(source, config)
        .tokenise_all()
        .map_err(|_| tcl_lexer::word_parts::NativeWordError::LexicalStreamUnavailable)?;
    let groups = tcl_lexer::group_commands_bytes(&tokens, source.bytes(), config);
    let mut module = lowerer.module;
    module.source = source.clone();
    module.native_namespace = Some(namespace.clone());
    module.top_level_kind = kind;
    module.top_level_namespace = module.advisory_top_level_namespace().unwrap_or_default();
    for group in groups {
        let words = group
            .words
            .iter()
            .map(|word| {
                let word = tcl_lexer::word_parts::NativeWord::from_group(
                    source.clone(),
                    config,
                    &tokens,
                    word,
                )?;
                Ok(word)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if let (Some(first), Some(last)) = (words.first(), words.last()) {
            module.top_level.statements.push(Statement::NativeCall {
                span: tcl_lexer::Span::new(first.span().start(), last.span().end()),
                words,
            });
        }
    }
    let origin = std::sync::Arc::new(tcl_names::origin::SourceOriginId::authored_image(
        source.clone(),
    ));
    let executed =
        tcl_names::origin::ExecutedScriptSource::contiguous_image(origin, source.clone(), 0)
            .expect("the complete original image is contiguous");
    module.top_level.executed_source = Some(std::sync::Arc::new(executed.clone()));
    let compiler_dialect = options.and_then(|options| options.native_compiler_dialect());
    let native_inline_disabled = compiler_dialect
        .and_then(tcl_native_model::InvocationDialect::native_string_protocol)
        .is_some_and(|protocol| protocol.tcl_version().is_some())
        && options
            .and_then(|options| options.native_entry)
            .is_some_and(|entry| entry.inline_compilation_disabled);
    let original_byte_compilation = options
        .and_then(|options| options.native_entry)
        .is_some_and(|entry| {
            let mut context = module.source_entry.native_compilation;
            if module.top_level_kind == crate::ir::TopLevelKind::ProcedureBody {
                context.mode =
                    tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
                context.frame =
                    tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
            }
            crate::native_byte_compilation::closed_original_byte_compilation(
                &module, entry, registry, context,
            )
        });
    if !native_inline_disabled
        && !original_byte_compilation
        && compiler_dialect
            .is_none_or(|dialect| dialect.family() != Some(tcl_dialect::model::Family::Jim))
    {
        module.top_level.native_compilation_admission = Some(std::sync::Arc::new(
            crate::native_compilation_admission::NativeCompilationAdmission {
                source: Some(executed),
                failure: None,
                provider_required: true,
            },
        ));
    }
    // Native calls retain unconstrained callbacks and variable effects.
    module.has_dynamic_trace = true;
    module.has_dynamic_variable_trace = true;
    module.retain_original_byte_source_namespace(namespace, registry);
    Ok(module)
}
