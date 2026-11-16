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

//! Selecting an original source definition or read of a compilation unit by its exact document extent.

use super::super::{SsaReadReference, Symbol, Version};
use super::analysis_definitions::{
    current_analysis_function, current_authored_analysis_function,
    owns_original_authored_analysis_tokens,
};
use super::{OriginalSourceSsaDefinitionName, SsaSourceView};
use tcl_registry::CommandRegistry;

/// Current analysis unit and authored SSA symbol selected by original definition geometry.
/// This readonly source projection grants no reached cell, Native value or edit permission.
pub struct OriginalSourceSsaDefinition<'a> {
    pub(super) function: &'a crate::compilation_unit::FunctionUnit,
    pub(in super::super) name: OriginalSourceSsaDefinitionName,
    pub(in super::super) symbol: Symbol,
    pub(in super::super) version: Version,
}

impl<'a> OriginalSourceSsaDefinition<'a> {
    /// Actual function from the supplied current compilation unit.
    #[must_use]
    pub const fn function(&self) -> &'a crate::compilation_unit::FunctionUnit {
        self.function
    }
    /// Exact original source definition and its issued body owner.
    #[must_use]
    pub const fn name(&self) -> &OriginalSourceSsaDefinitionName {
        &self.name
    }
    /// Authored analysis symbol under this function's own interner.
    #[must_use]
    pub const fn symbol(&self) -> Symbol {
        self.symbol
    }
    /// Version at the represented conditional source definition.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }
}

/// Select a unique current source definition by its original document extent.
/// Equal variable labels and function display names never select the owner.
#[must_use]
pub fn original_source_definition_at_unit<'a>(
    unit: &'a crate::compilation_unit::CompilationUnit,
    span: tcl_lexer::Span,
    registry: &CommandRegistry,
) -> Option<OriginalSourceSsaDefinition<'a>> {
    let mut selected: Option<OriginalSourceSsaDefinition<'a>> = None;
    for function in unit.all_body_function_units() {
        let Some(body) = function.body_id else {
            continue;
        };
        for (&block, statements) in &function.ssa.blocks {
            for (index, statement) in statements.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                for (&symbol, &version) in &statement.defs {
                    let Some(name) =
                        view.original_source_definition_for_body(symbol, unit, body, registry)
                    else {
                        continue;
                    };
                    if name.span() != span {
                        continue;
                    }
                    if selected.as_ref().is_some_and(|old| {
                        old.name != name
                            || old.symbol != symbol
                            || old.version != version
                            || !std::ptr::eq(old.function, function)
                    }) {
                        return None;
                    }
                    selected = Some(OriginalSourceSsaDefinition {
                        function,
                        name,
                        symbol,
                        version,
                    });
                }
            }
        }
    }
    selected
}

/// An original read mapped to its current function's own SSA symbol.
/// This readonly association supplies no successful read, physical value or edit authority.
pub struct OriginalSourceSsaRead<'a> {
    pub(super) function: &'a crate::compilation_unit::FunctionUnit,
    pub(in super::super) span: tcl_lexer::Span,
    pub(super) reference: SsaReadReference,
}

impl<'a> OriginalSourceSsaRead<'a> {
    /// Actual current compilation owner of this read.
    #[must_use]
    pub const fn function(&self) -> &'a crate::compilation_unit::FunctionUnit {
        self.function
    }
    /// Complete unchanged original substitution extent.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Canonical symbol retained by this function's original read producer.
    #[must_use]
    pub const fn symbol(&self) -> Symbol {
        self.reference.symbol
    }
    /// Represented contents version, when independently available.
    #[must_use]
    pub const fn version(&self) -> Option<Version> {
        self.reference.version
    }
}

/// Select an original read by exact document geometry and actual compilation ownership.
/// Missing read provenance never falls back to a variable or function display name.
#[must_use]
pub fn original_source_read_at_unit<'a>(
    unit: &'a crate::compilation_unit::CompilationUnit,
    span: tcl_lexer::Span,
    registry: &CommandRegistry,
) -> Option<OriginalSourceSsaRead<'a>> {
    let module = &unit.ir_module;
    if module.source.try_text().ok()? != unit.source {
        return None;
    }
    let input = module.source_metadata_input.as_ref();
    let retained = module.retained_source_bindings.as_deref()?;
    let positioned = retained.positioned_for_module(module, registry)?;
    let origin = positioned.source_origin()?;
    let mut selected: Option<OriginalSourceSsaRead<'a>> = None;
    for function in unit.all_body_function_units() {
        let current = if input.is_some() {
            current_analysis_function(unit, function, registry)
        } else {
            current_authored_analysis_function(unit, function, registry)
        };
        if !current {
            continue;
        }
        for ((block, index, source, spelling), reference) in function
            .ssa
            .read_references
            .iter()
            .chain(function.ssa.source_read_references.iter())
        {
            if source.provenance != crate::ir::Provenance::Source {
                continue;
            }
            let start =
                u32::try_from(i64::from(source.span.start()).checked_add(function.base_offset)?)
                    .ok()?;
            let end =
                u32::try_from(i64::from(source.span.end()).checked_add(function.base_offset)?)
                    .ok()?;
            if tcl_lexer::Span::new(start, end) != span
                || unit.source.get(span.as_range()) != Some(spelling.as_str())
            {
                continue;
            }
            let view = SsaSourceView::at_statement(&function.ssa, *block, *index);
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
            let owner = if input.is_none() {
                !binding.has_original_native_source_command()
                    && !binding.has_original_logical_source_advice()
                    && owns_original_authored_analysis_tokens(unit, function, &original, registry)
            } else if binding.has_original_native_source_command() {
                function.body_id.is_some_and(|body| {
                    binding.original_native_source_owner_for_body(&original, module, body, registry)
                }) && binding
                    .original_native_source_words(&original, input?, registry)
                    .is_some()
            } else if binding.has_original_logical_source_advice() {
                binding.original_logical_source_words(&original, input?, registry).is_some_and(|words| {
                    matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice)
                        if std::sync::Arc::ptr_eq(&advice.site().source, origin))
                })
            } else {
                function.body_id.map_or_else(
                    || {
                        retained.owns_original_tokens(&original)
                            && binding
                                .invocation_site()
                                .is_some_and(|site| std::sync::Arc::ptr_eq(&site.source, origin))
                    },
                    |body| retained.owns_original_body_tokens(module, body, &original, registry),
                )
            };
            if !owner {
                continue;
            }
            let reference = function
                .ssa
                .read_references
                .get(&(*block, *index, source.clone(), spelling.clone()))
                .copied()
                .unwrap_or(*reference);
            if selected.as_ref().is_some_and(|old| {
                !std::ptr::eq(old.function, function) || old.reference != reference
            }) {
                return None;
            }
            selected = Some(OriginalSourceSsaRead {
                function,
                span,
                reference,
            });
        }
    }
    selected
}
