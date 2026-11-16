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

//! Selecting a represented analysis definition by its written occurrence: authored operands, logical advice and the original word vector.

use super::super::{SsaStatement, Symbol, Version};
use super::SsaSourceView;
use crate::cfg::BlockId;
use crate::ir::CommandTokens;
use tcl_registry::CommandRegistry;

/// One exact source definition in the current compilation owner's analysis.
/// Its SSA version supplies no physical cell, completed store or edit authority.
pub struct OriginalAnalysisSsaDefinition<'a> {
    pub(super) function: &'a crate::compilation_unit::FunctionUnit,
    pub(in super::super) span: tcl_lexer::Span,
    pub(in super::super) symbol: Symbol,
    pub(in super::super) version: Version,
    source_name: Vec<u8>,
}

impl<'a> OriginalAnalysisSsaDefinition<'a> {
    /// Actual current function owning this definition.
    #[must_use]
    pub const fn function(&self) -> &'a crate::compilation_unit::FunctionUnit {
        self.function
    }
    /// Exact original written variable operand extent.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Symbol from this function's own source operation.
    #[must_use]
    pub const fn symbol(&self) -> Symbol {
        self.symbol
    }
    /// Version at this represented definition, independently of subsequent writes.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }
    /// Exact literal receiver value retained by this original definition's
    /// source producer. It is not a reference spelling or a physical cell.
    #[must_use]
    pub fn source_name_bytes(&self) -> &[u8] {
        &self.source_name
    }
}

pub(in super::super) fn original_written_source_variable_name(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    written: usize,
    span: tcl_lexer::Span,
) -> Option<&str> {
    let mut selected = words
        .origins()
        .iter()
        .enumerate()
        .filter_map(|(effective, origin)| {
            (*origin
                == crate::registry_invocation::InvocationWordOrigin::Written(
                    written.checked_add(1)?,
                ))
            .then(|| effective.checked_sub(1))
            .flatten()
        });
    let argument = selected.next()?;
    if selected.next().is_some() || words.operands().get(argument)?.as_ref()?.span() != span {
        return None;
    }
    std::str::from_utf8(words.arguments().get(argument)?.literal_bytes()?).ok()
}

/// Exact source word geometry at one current represented analysis operation.
/// This supplies no read value, completed invocation, Normal or edit authority.
pub struct OriginalAnalysisSsaWords {
    pub(in super::super) words: Vec<tcl_lexer::NativeWord>,
    pub(in super::super) config: tcl_lexer::LexerConfig,
}

impl OriginalAnalysisSsaWords {
    /// Complete original lexical vector in document coordinates.
    #[must_use]
    pub fn words(&self) -> &[tcl_lexer::NativeWord] {
        &self.words
    }

    /// Actual grammar used to authenticate this vector.
    #[must_use]
    pub const fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

/// Authenticate the actual function, body, input and whole original word vector.
/// Present readonly child purposes remain terminal when their owner refuses.
#[must_use]
pub fn original_analysis_words_for_statement(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    block: BlockId,
    index: usize,
    registry: &CommandRegistry,
) -> Option<OriginalAnalysisSsaWords> {
    if !current_analysis_function(unit, function, registry) {
        return None;
    }
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref()?;
    let retained = module.retained_source_bindings.as_deref()?;
    let positioned = retained.positioned_for_module(module, registry)?;
    let origin = positioned.source_origin()?;
    let tokens = SsaSourceView::at_statement(&function.ssa, block, index).source_tokens()?;
    let mut original = tokens.clone();
    crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
    let binding = original.source_binding.as_ref()?;
    retained
        .owns_original_analysis_attachment(&original)
        .then_some(())?;
    let config = if binding.has_original_native_source_command() {
        let body = function.body_id?;
        if !binding.original_native_source_owner_for_body(&original, module, body, registry) {
            return None;
        }
        binding.original_native_source_words(&original, input, registry)?;
        input.lexer_config()
    } else if binding.has_original_logical_source_advice() {
        let words = binding.original_logical_source_words(&original, input, registry)?;
        if !matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice)
            if std::sync::Arc::ptr_eq(&advice.site().source, origin))
        {
            return None;
        }
        input.lexer_config()
    } else {
        let owned = function.body_id.map_or_else(
            || {
                retained.owns_original_tokens(&original)
                    && binding
                        .invocation_site()
                        .is_some_and(|site| std::sync::Arc::ptr_eq(&site.source, origin))
            },
            |body| retained.owns_original_body_tokens(module, body, &original, registry),
        );
        if !owned {
            return None;
        }
        binding.original_lexer_config_for_tokens(&original)?
    };
    let offset = original.words().first()?.source().span.start();
    let words = crate::registry_invocation::original_native_compiler_words(
        &module.source,
        original.words(),
        offset,
        config,
    )?;
    Some(OriginalAnalysisSsaWords { words, config })
}

pub(super) fn current_analysis_function(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> bool {
    let module = &unit.ir_module;
    let Some(input) = module.source_metadata_input.as_ref() else {
        return false;
    };
    let Some(retained) = module.retained_source_bindings.as_deref() else {
        return false;
    };
    if module.source.try_text().ok() != Some(unit.source.as_str())
        || retained.positioned_for_module(module, registry).is_none()
        || unit
            .function_for_body(function.body_id)
            .is_none_or(|current| !std::ptr::eq(current, function))
        || function
            .invocation_metadata_context_for_module(registry, module)
            .is_none()
        || function
            .source_metadata_input()
            .is_none_or(|held| held != input && held != &input.for_nested_source())
    {
        return false;
    }
    match function.body_id {
        Some(body) => unit
            .original_procedure_for_body(body)
            .is_some_and(|procedure| {
                retained.matches_original_procedure_summary(body, procedure)
                    && unit.original_procedure_cfg_for_body(body) == Some(&function.cfg)
            }),
        None => function.cfg == unit.cfg_module.top_level,
    }
}

/// Current original authored analysis graph; no execution or frame authority.
pub(crate) fn current_authored_analysis_function(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> bool {
    let module = &unit.ir_module;
    let Some(retained) = module.retained_source_bindings.as_deref() else {
        return false;
    };
    if module.source_metadata_input.is_some()
        || !module.source_entry.metadata_context.is_standalone()
        || module.source_entry.native_entry.is_some()
        || module.source_entry.logical_source_input.is_some()
        || module.source_entry.vendor_source_input.is_some()
        || module.source_entry.hosted_execution_context.is_some()
        || module.source_entry.unknown_entry
        || function.source_metadata_input().is_some()
        || function.source_lexer_config().nested().normalized()
            != module.native_lexer_config().nested().normalized()
        || module.source.try_text().ok() != Some(unit.source.as_str())
        || retained.positioned_for_module(module, registry).is_none()
        || unit
            .function_for_body(function.body_id)
            .is_none_or(|current| !std::ptr::eq(current, function))
    {
        return false;
    }
    match function.body_id {
        Some(body) => unit
            .original_procedure_for_body(body)
            .is_some_and(|procedure| {
                module
                    .original_authored_declaration_body_units
                    .get(&body)
                    .is_some_and(|card| card.matches_procedure(module, procedure))
                    && retained.matches_original_procedure_summary(body, procedure)
                    && unit.original_procedure_cfg_for_body(body) == Some(&function.cfg)
            }),
        None => function.cfg == unit.cfg_module.top_level,
    }
}

pub(super) fn owns_original_authored_analysis_tokens(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    original: &CommandTokens,
    registry: &CommandRegistry,
) -> bool {
    let module = &unit.ir_module;
    let Some(retained) = module.retained_source_bindings.as_deref() else {
        return false;
    };
    let Some(binding) = original.source_binding.as_ref() else {
        return false;
    };
    if !retained.owns_original_analysis_attachment(original)
        || original.synthetic.is_some()
        || !original.original_words_match_argv_text()
        || binding.has_original_native_source_command()
        || binding.has_original_logical_source_advice()
    {
        return false;
    }
    if binding.has_original_authored_source_body_name_role_owner() {
        let Some(scope) = binding
            .original_authored_scalar_body_scope(original, registry)
            .or_else(|| {
                binding.original_authored_source_body_scope_for_name_roles(original, registry)
            })
        else {
            return false;
        };
        let Some(body) = function.body_id else {
            return false;
        };
        scope.matches_module(module)
            && scope.original_words(original, registry).is_some()
            && module
                .original_authored_declaration_body_units
                .get(&body)
                .is_some_and(|card| card.owns_original_source_scope(scope))
    } else {
        function.body_id.is_none() && retained.owns_original_tokens(original)
    }
}

fn original_authored_definition_operands(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    original: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<Vec<(tcl_lexer::Span, String)>> {
    let module = &unit.ir_module;
    let binding = original.source_binding.as_ref()?;
    if binding.has_original_native_source_command()
        || binding.has_original_logical_source_advice()
        || original.synthetic.is_some()
        || !original.original_words_match_argv_text()
    {
        return None;
    }
    let roles = if binding.has_original_authored_source_body_name_role_owner() {
        let scope = binding
            .original_authored_scalar_body_scope(original, registry)
            .or_else(|| {
                binding.original_authored_source_body_scope_for_name_roles(original, registry)
            })?;
        if !scope.matches_module(module) || scope.has_unclosed_declaration_source_coverage() {
            return None;
        }
        let body = function.body_id?;
        module
            .original_authored_declaration_body_units
            .get(&body)?
            .owns_original_source_scope(scope)
            .then_some(())?;
        let words = scope.original_words(original, registry)?;
        if words.iter().any(|word| word.group().expand) {
            return None;
        }
        scope.with_source_schema(original, registry, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            (complete && schema.semantics.frame_effect.is_none()).then(|| {
                roles
                    .into_iter()
                    .map(|(argument, role)| {
                        (schema.facts().argument_offset + usize::from(argument), role)
                    })
                    .collect::<Vec<_>>()
            })
        })?
    } else {
        function.body_id.is_none().then_some(())?;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.owns_original_tokens(original).then_some(())?;
        let grammar =
            crate::registry_invocation::original_authored_source_structure(original, registry)?
                .into_closed_source_grammar()?;
        if grammar.has_original_word_expansion()
            || grammar.config().nested().normalized()
                != function.source_lexer_config().nested().normalized()
        {
            return None;
        }
        grammar.with_source_schema(registry, |schema| {
            schema.semantics.frame_effect.is_none().then_some(())
        })?;
        grammar.written_roles()
    };
    let offset = original.words().first()?.source().span.start();
    let written = crate::registry_invocation::original_native_compiler_words(
        &module.source,
        original.words(),
        offset,
        module.native_lexer_config(),
    )?;
    roles
        .into_iter()
        .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
        .map(|(argument, _)| {
            let word = written.get(argument.checked_add(1)?)?;
            if word.group().expand {
                return None;
            }
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
            let name = std::str::from_utf8(&value).ok()?.to_owned();
            Some((word.span(), name))
        })
        .collect()
}

fn original_authored_analysis_definition_at_unit<'a>(
    unit: &'a crate::compilation_unit::CompilationUnit,
    span: tcl_lexer::Span,
    registry: &CommandRegistry,
) -> Option<OriginalAnalysisSsaDefinition<'a>> {
    let mut selected: Option<OriginalAnalysisSsaDefinition<'a>> = None;
    for function in unit.all_body_function_units() {
        if !current_authored_analysis_function(unit, function, registry) {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let mut original = tokens.clone();
                crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
                if unit
                    .ir_module
                    .retained_source_bindings
                    .as_deref()
                    .is_none_or(|retained| !retained.owns_original_analysis_attachment(&original))
                {
                    continue;
                }
                let Some(operands) =
                    original_authored_definition_operands(unit, function, &original, registry)
                else {
                    continue;
                };
                for (operand_span, name) in operands {
                    if operand_span != span {
                        continue;
                    }
                    let Some(symbol) = view.symbol(&name) else {
                        continue;
                    };
                    let Some(&version) = statement.defs.get(&symbol) else {
                        continue;
                    };
                    if statement.may_defs.contains(&symbol) {
                        continue;
                    }
                    if selected.as_ref().is_some_and(|old| {
                        !std::ptr::eq(old.function, function)
                            || old.symbol != symbol
                            || old.version != version
                            || old.source_name != name.as_bytes()
                    }) {
                        return None;
                    }
                    selected = Some(OriginalAnalysisSsaDefinition {
                        function,
                        span,
                        symbol,
                        version,
                        source_name: name.into_bytes(),
                    });
                }
            }
        }
    }
    selected
}

fn merge_analysis_definitions<'a>(
    selected: &mut Option<OriginalAnalysisSsaDefinition<'a>>,
    function: &'a crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
    definitions: AnalysisDefinitionRows,
) -> Option<()> {
    for (definition_span, symbol, version, source_name) in definitions {
        if definition_span != span {
            continue;
        }
        if selected.as_ref().is_some_and(|old| {
            !std::ptr::eq(old.function, function)
                || old.symbol != symbol
                || old.version != version
                || old.source_name != source_name
        }) {
            return None;
        }
        *selected = Some(OriginalAnalysisSsaDefinition {
            function,
            span,
            symbol,
            version,
            source_name,
        });
    }
    Some(())
}

type AnalysisDefinitionRows = Vec<(tcl_lexer::Span, Symbol, Version, Vec<u8>)>;

struct LogicalAnalysisDefinitionContext<'a> {
    module: &'a crate::ir::Module,
    pub(super) function: &'a crate::compilation_unit::FunctionUnit,
    pub(in super::super) registry: &'a CommandRegistry,
    pub(super) input: &'a crate::analyser::ResolvedAnalysisInput,
    retained: &'a crate::command_binding::RetainedSourceModuleBindings,
    positioned: &'a crate::command_binding::SourceCommandBindings,
    origin: &'a std::sync::Arc<crate::command_binding::SourceOriginId>,
}

fn logical_analysis_definitions(
    context: &LogicalAnalysisDefinitionContext<'_>,
    binding: &crate::command_binding::SourceInvocationBinding,
    original: &crate::ir::CommandTokens,
    view: SsaSourceView<'_>,
    statement: &SsaStatement,
) -> Option<AnalysisDefinitionRows> {
    let LogicalAnalysisDefinitionContext {
        module,
        function,
        registry,
        input,
        retained,
        positioned,
        origin,
    } = *context;
    let mut definitions = Vec::new();
    if binding.has_original_logical_source_advice() {
        let words = binding.original_logical_source_words(original, input, registry)?;
        if !matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice)
            if std::sync::Arc::ptr_eq(&advice.site().source, origin))
        {
            return None;
        }
        collect_logical_advice_definitions(
            &words,
            original,
            module,
            view,
            statement,
            &mut definitions,
        );
    } else {
        let owned = function.body_id.map_or_else(
            || retained.owns_original_tokens(original),
            |body| retained.owns_original_body_tokens(module, body, original, registry),
        );
        if !owned {
            return None;
        }
        let metadata = function.invocation_metadata_context_for_module(registry, module)?;
        let invocation =
            crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                registry,
                metadata,
                original,
                Some(positioned),
            )?;
        for (argument, role) in invocation.written_roles() {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            let Some(word) = original.words().get(argument + 1) else {
                continue;
            };
            let Some(name) = invocation.written_variable_name_operand(argument) else {
                continue;
            };
            let Some(symbol) = view.symbol(&name) else {
                continue;
            };
            let Some(&version) = statement.defs.get(&symbol) else {
                continue;
            };
            if !statement.may_defs.contains(&symbol) {
                definitions.push((
                    word.source().span,
                    symbol,
                    version,
                    name.as_bytes().to_vec(),
                ));
            }
        }
    }
    Some(definitions)
}

fn collect_logical_advice_definitions(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    original: &crate::ir::CommandTokens,
    module: &crate::ir::Module,
    view: SsaSourceView<'_>,
    statement: &SsaStatement,
    definitions: &mut AnalysisDefinitionRows,
) {
    for (argument, role) in words.written_argument_roles() {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let Some(word) = original.words().get(argument + 1) else {
            continue;
        };
        let Some(receiver_span) = word.original_written_span(module.source.bytes()) else {
            continue;
        };
        let Some(name) = original_written_source_variable_name(words, argument, receiver_span)
        else {
            continue;
        };
        let Some(symbol) = view.symbol(name) else {
            continue;
        };
        let Some(&version) = statement.defs.get(&symbol) else {
            continue;
        };
        if !statement.may_defs.contains(&symbol) {
            definitions.push((receiver_span, symbol, version, name.as_bytes().to_vec()));
        }
    }
}

/// Select a represented source definition by its exact current written occurrence.
/// Source body purposes remain separate from physical Native definition receipts.
#[must_use]
pub fn original_analysis_definition_at_unit<'a>(
    unit: &'a crate::compilation_unit::CompilationUnit,
    span: tcl_lexer::Span,
    registry: &CommandRegistry,
) -> Option<OriginalAnalysisSsaDefinition<'a>> {
    let module = &unit.ir_module;
    if module.source_metadata_input.is_none() {
        return original_authored_analysis_definition_at_unit(unit, span, registry);
    }
    let input = module.source_metadata_input.as_ref()?;
    let retained = module.retained_source_bindings.as_deref()?;
    let positioned = retained.positioned_for_module(module, registry)?;
    let origin = positioned.source_origin()?;
    let mut selected: Option<OriginalAnalysisSsaDefinition<'a>> = None;
    for function in unit.all_body_function_units() {
        if !current_analysis_function(unit, function, registry) {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let mut original = tokens.clone();
                crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
                let Some(binding) = original.source_binding.as_ref() else {
                    continue;
                };
                if !retained.owns_original_analysis_attachment(&original) {
                    continue;
                }
                let mut definitions = Vec::new();
                if binding.has_original_native_source_command() {
                    let Some(body) = function.body_id else {
                        continue;
                    };
                    for (&symbol, &version) in &statement.defs {
                        if let Some(name) =
                            view.original_source_definition_for_body(symbol, unit, body, registry)
                        {
                            definitions.push((
                                name.span(),
                                symbol,
                                version,
                                name.original_name_input().bytes().to_vec(),
                            ));
                        }
                    }
                } else if input.has_logical_source_name_context() {
                    let context = LogicalAnalysisDefinitionContext {
                        module,
                        function,
                        registry,
                        input,
                        retained,
                        positioned,
                        origin,
                    };
                    let Some(logical_definitions) =
                        logical_analysis_definitions(&context, binding, &original, view, statement)
                    else {
                        continue;
                    };
                    definitions = logical_definitions;
                } else {
                    let metadata =
                        function.invocation_metadata_context_for_module(registry, module)?;
                    for (&symbol, &version) in &statement.defs {
                        if statement.may_defs.contains(&symbol) {
                            continue;
                        }
                        if let Some(name) = view.original_definition_name_with_metadata_context(
                            symbol,
                            registry,
                            Some(metadata),
                        ) {
                            let offset = function.base_offset;
                            let start =
                                u32::try_from(i64::from(name.span().start()).checked_add(offset)?)
                                    .ok()?;
                            let end =
                                u32::try_from(i64::from(name.span().end()).checked_add(offset)?)
                                    .ok()?;
                            definitions.push((
                                tcl_lexer::Span::new(start, end),
                                symbol,
                                version,
                                name.original_name_input().bytes().to_vec(),
                            ));
                        }
                    }
                }
                merge_analysis_definitions(&mut selected, function, span, definitions)?;
            }
        }
    }
    selected
}
