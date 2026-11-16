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

//! Cards from source documentation: schema and root declarations, original procedures and native formals.

use super::classes::class_hover_text;
use super::procedures::{format_docstring, proc_hover_text};
use super::variables::{
    current_special_variable_source_hover, selected_root_special_variable_source_hover,
};
use super::{Hover, original_registry};
use tcl_compiler::analyser::AnalysisResult;

/// Bounded source documentation from the original schema or selected root
/// special-variable reference. This performs no procedure or alias lookup.
#[must_use]
pub fn source_documentation_hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<Hover> {
    analysis
        .analysis_context_unavailable
        .is_none()
        .then_some(())?;
    let position = crate::original_context::CurrentSourcePosition::capture_variable(
        source, analysis, line, character,
    )?;
    let (line, character) = position.line_character();
    let cursor = position.offset();
    let resolution = crate::definition::CallResolution::document_only()
        .with_registry(position.context().registry());
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select_navigation(source, analysis, line, character, resolution)
    {
        let selected = selected?;
        let mut hover = match selected {
            crate::variable_symbol::NavigationVariable::NativeRootSourceReference(reference) => {
                selected_root_special_variable_source_hover(
                    source,
                    analysis,
                    reference.occurrence(),
                    cursor,
                )
            }
            crate::variable_symbol::NavigationVariable::LogicalRootSourceReference(reference) => {
                current_special_variable_source_hover(
                    source,
                    analysis,
                    reference.occurrence(),
                    cursor,
                )
            }
            crate::variable_symbol::NavigationVariable::Original(occurrence) => {
                current_special_variable_source_hover(source, analysis, occurrence, cursor)
            }
            _ => None,
        }?;
        hover
            .value
            .push_str("\n\n_Source documentation; the binding, availability and the current value are unresolved._");
        return Some(hover);
    }
    if crate::source_structure::original_variable_reference_at(source, analysis, cursor).is_some() {
        return None;
    }
    source_class_declaration_hover(source, analysis, cursor)
        .or_else(|| original_registry::schema_only_hover(source, analysis, cursor))
}

fn source_class_declaration_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<Hover> {
    use crate::original_declaration::OriginalDeclarationRole;
    let rows = crate::original_declaration::declarations("hover://document", source, analysis)?;
    let mut selected = rows.iter().filter(|row| {
        matches!(
            row.role(),
            OriginalDeclarationRole::Class | OriginalDeclarationRole::Method(_)
        ) && row.span().start() <= cursor
            && cursor < row.span().end()
    });
    let identity = selected.next()?;
    selected.next().is_none().then_some(())?;
    identity
        .is_current("hover://document", source, analysis)
        .then_some(())?;
    let class = identity.class_metadata(analysis)?;
    let mut text = if identity.role() == OriginalDeclarationRole::Class {
        class_hover_text(analysis, class.metadata())
    } else {
        let method = identity.method_metadata(analysis)?.metadata();
        let mut text = format!(
            "**{}** `{}::{}` ({} parameter(s))",
            method.kind,
            class.metadata().qualified_name,
            identity.label(),
            method.params.len()
        );
        if !method.doc.is_empty() {
            text.push_str("\n\n");
            text.push_str(&format_docstring(&method.doc));
        }
        text
    };
    text.push_str("\n\n_Source declaration; member and class availability depend on execution._");
    Some(Hover::markdown(text))
}

pub(crate) fn source_documentation_region_owns_cursor(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    region: &tcl_compiler::command_binding::OriginalSourceConditionalRegion,
) -> Option<bool> {
    original_registry::region_owns_documentation_cursor(source, analysis, cursor, region)
}

/// Render one original procedure selected under its exact source owner.
/// The source-view consumer independently joins its publication and region.
pub(crate) fn original_source_procedure_hover(
    source: &str,
    analysis: &AnalysisResult,
    identity: &crate::original_declaration::OriginalDeclarationIdentity,
) -> Option<Hover> {
    identity
        .is_current(identity.uri(), source, analysis)
        .then_some(())?;
    let declaration = identity.procedure_metadata(analysis)?;
    Some(Hover::markdown(format!(
        "**Conditional source declaration**\n\n{}",
        proc_hover_text(declaration.metadata()),
    )))
}

/// Conditional name documentation from an actual Native source formal list.
/// This reports no installed binding, entered frame or current value.
pub(super) fn conditional_native_formal_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<Hover> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let mut matches = Vec::new();
    for declaration in analysis.original_procedure_declarations() {
        let Some(formals) = analysis
            .conditional_native_procedure_formal_name_sources(declaration, current.registry())
        else {
            continue;
        };
        for formal in formals {
            if formal
                .source_span()
                .is_some_and(|span| span.start() <= cursor && cursor < span.end())
            {
                matches.push(formal);
            }
        }
    }
    let [formal] = matches.as_slice() else {
        return None;
    };
    Some(Hover::markdown(format!(
        "**Parameter** `{}`\n\nConditional source parameter; runtime binding and value are unresolved.",
        tcl_syntax::native_string::resident_name_label(formal.name())
    )))
}
