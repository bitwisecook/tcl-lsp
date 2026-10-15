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

//! Checked source-symbol joins for actual analyser variable declarations.

use super::{AnalysisResult, Scope, VarDef};
use crate::signature_scan::scope::SignatureSourceNameInput;
use crate::signature_scan::variable_symbol::{
    OriginalVariableAliasTemplate, SignatureSourceVariableOccurrence, SignatureSourceVariableSymbol,
};
use tcl_lexer::{LexerConfig, SourceImage, Span};

enum DeclarationSource<'a> {
    Occurrence(&'a SignatureSourceVariableOccurrence),
    ConditionalAlias(OriginalVariableAliasTemplate),
}

/// A genuine current variable declaration joined to its original source symbol.
/// Conditional alias relationships retain their independent applicability and
/// binding obligations; this projection supplies no physical cell or edit grant.
pub struct OriginalVariableSourceDeclaration<'a> {
    span: Span,
    source: DeclarationSource<'a>,
}

impl OriginalVariableSourceDeclaration<'_> {
    /// Exact nonempty declaration extent of the actual owned variable record.
    #[must_use]
    pub const fn declaration_span(&self) -> Span {
        self.span
    }

    /// Typed original naming geometry, independently of a current target cell.
    #[must_use]
    pub fn symbol(&self) -> &SignatureSourceVariableSymbol {
        match &self.source {
            DeclarationSource::Occurrence(occurrence) => occurrence.symbol(),
            DeclarationSource::ConditionalAlias(alias) => alias.target_symbol(),
        }
    }

    /// Actual declaration input, never reconstructed from its reporting label.
    #[must_use]
    pub fn original_name_input(&self) -> &SignatureSourceNameInput {
        match &self.source {
            DeclarationSource::Occurrence(occurrence) => occurrence.original_name_input(),
            DeclarationSource::ConditionalAlias(alias) => alias.original_name_input(),
        }
    }

    /// Independently retained declaration occurrence, when present.
    /// Source coverage and edit permissions remain separate caller obligations.
    #[must_use]
    pub fn original_occurrence(&self) -> Option<&SignatureSourceVariableOccurrence> {
        match &self.source {
            DeclarationSource::Occurrence(occurrence) => Some(occurrence),
            DeclarationSource::ConditionalAlias(_) => None,
        }
    }

    /// Conditional alias owner with its unchanged source-frame/binding obligations.
    /// Its presence cannot establish an editable reference set or successful link.
    #[must_use]
    pub fn alias_template(&self) -> Option<&OriginalVariableAliasTemplate> {
        match &self.source {
            DeclarationSource::Occurrence(_) => None,
            DeclarationSource::ConditionalAlias(alias) => Some(alias),
        }
    }
}

impl AnalysisResult {
    /// Join an actual variable record to unanimous current declaration-source
    /// evidence. Pointer membership and the complete original image/input/config
    /// precede the exact nonempty declaration span; labels and link reports are
    /// never identity inputs. Missing evidence cannot borrow a same-span record.
    #[must_use]
    pub fn original_variable_declaration_in_source(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        variable: &VarDef,
    ) -> Option<OriginalVariableSourceDeclaration<'_>> {
        if !self.matches_original_source_image(image, config)
            || variable.definition_span.is_empty()
            || !owns_variable_declaration(&self.global_scope, variable)
        {
            return None;
        }
        let span = variable.definition_span;
        let mut occurrences = self
            .original_variable_symbols
            .iter()
            .filter(|occurrence| occurrence.span() == span && occurrence.is_declaration());
        if let Some(first) = occurrences.next() {
            if occurrences.any(|other| other != first)
                || self.original_variable_symbol_in_source(image, config, span.start())
                    != Some(first)
            {
                return None;
            }
            return Some(OriginalVariableSourceDeclaration {
                span,
                source: DeclarationSource::Occurrence(first),
            });
        }
        if self.original_variable_symbols.iter().any(|occurrence| {
            occurrence.span() == span && occurrence.original_alias_declaration_span() != Some(span)
        }) {
            return None;
        }
        let mut aliases = self
            .original_variable_alias_templates_in_source(image, config)?
            .into_iter()
            .filter(|alias| alias.is_declaration() && alias.declaration_span() == span);
        let first = aliases.next()?;
        if aliases.any(|other| other != first) {
            return None;
        }
        Some(OriginalVariableSourceDeclaration {
            span,
            source: DeclarationSource::ConditionalAlias(first),
        })
    }
}

fn owns_variable_declaration(root: &Scope, variable: &VarDef) -> bool {
    let mut pending = vec![root];
    while let Some(scope) = pending.pop() {
        if let Some((key, _)) = scope
            .variables
            .iter()
            .find(|(_, candidate)| std::ptr::eq(*candidate, variable))
        {
            return scope.original_variable_declaration_spans.get(key)
                == Some(&variable.definition_span);
        }
        pending.extend(&scope.children);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn original_variable_declaration_join_requires_the_owned_record_and_current_source() {
        let source = "set actual 1; puts $actual";
        let analysis = Analyser::new().analyse(source, "tcl");
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        assert_eq!(analysis.global_scope.variables.len(), 1);
        let variable = analysis.global_scope.variables.values().next().unwrap();
        let declaration = analysis
            .original_variable_declaration_in_source(&image, config, variable)
            .expect("actual source declaration and owned record");
        assert_eq!(declaration.declaration_span(), variable.definition_span);
        assert!(declaration.original_occurrence().is_some());
        assert!(declaration.alias_template().is_none());
        let mut report_only = analysis.clone();
        report_only
            .global_scope
            .variables
            .values_mut()
            .next()
            .unwrap()
            .name = "REPORT_ONLY".to_owned();
        let changed_label = report_only.global_scope.variables.values().next().unwrap();
        let unchanged = report_only
            .original_variable_declaration_in_source(&image, config, changed_label)
            .expect("reporting names do not authenticate declarations");
        assert_eq!(unchanged.symbol(), declaration.symbol());
        let copied = variable.clone();
        assert!(
            analysis
                .original_variable_declaration_in_source(&image, config, &copied)
                .is_none()
        );
        assert!(
            analysis
                .original_variable_declaration_in_source(
                    &SourceImage::document("set actual 2; puts $actual"),
                    config,
                    variable,
                )
                .is_none()
        );
        let mut changed = config;
        changed.leading_bom = match config.leading_bom {
            tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
            tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        };
        assert!(
            analysis
                .original_variable_declaration_in_source(&image, changed, variable)
                .is_none()
        );
    }

    #[test]
    fn original_variable_declaration_join_refuses_changed_definitions_and_unowned_scopes() {
        let source = "set actual 1; puts $actual";
        let analysis = Analyser::new().analyse(source, "tcl");
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let read = u32::try_from(source.find("$actual").unwrap()).unwrap();
        let mut changed_definition = analysis.clone();
        changed_definition
            .global_scope
            .variables
            .values_mut()
            .next()
            .unwrap()
            .definition_span = analysis
            .original_variable_symbol_at_offset(read)
            .unwrap()
            .span();
        let variable = changed_definition
            .global_scope
            .variables
            .values()
            .next()
            .unwrap();
        assert!(
            changed_definition
                .original_variable_declaration_in_source(&image, config, variable)
                .is_none()
        );
        let mut moved = analysis.clone();
        let (_, variable) = moved.global_scope.variables.drain().next().unwrap();
        let mut foreign_scope = Scope::new(crate::analyser::ScopeKind::Namespace, "REPORT_ONLY");
        foreign_scope
            .variables
            .insert("REPORT_ONLY".to_owned(), variable);
        moved.global_scope.children.push(foreign_scope);
        let variable = moved
            .global_scope
            .children
            .last()
            .unwrap()
            .variables
            .values()
            .next()
            .unwrap();
        assert!(
            moved
                .original_variable_declaration_in_source(&image, config, variable)
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let variable = missing.global_scope.variables.values().next().unwrap();
        assert!(
            missing
                .original_variable_declaration_in_source(&image, config, variable)
                .is_none()
        );
    }

    #[test]
    fn original_variable_declaration_join_keeps_source_identity_after_reporting_scope_move() {
        let source = "set actual 1; puts $actual";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let variable = analysis.global_scope.variables.values().next().unwrap();
        let symbol = analysis
            .original_variable_declaration_in_source(&image, config, variable)
            .expect("actual original declaration")
            .symbol()
            .clone();
        let original = std::mem::replace(
            &mut analysis.global_scope,
            Scope::new(crate::analyser::ScopeKind::Namespace, "REPORT_ONLY"),
        );
        analysis.global_scope.children.push(original);
        let variable = analysis.global_scope.children[0]
            .variables
            .values()
            .next()
            .unwrap();
        let declaration = analysis
            .original_variable_declaration_in_source(&image, config, variable)
            .expect("intact original producer belongs to the same source");
        assert_eq!(declaration.symbol(), &symbol);
        assert!(declaration.original_occurrence().is_some());
    }

    #[test]
    fn original_alias_declaration_join_keeps_template_obligations_and_frame() {
        let source = "proc p {i} {upvar ::n::a($i) data; lappend data value; puts $data}";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(
            analysis
                .resolved_input
                .as_ref()
                .unwrap()
                .has_logical_source_name_context()
        );
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let span = u32::try_from(source.find(" data;").unwrap() + 1).unwrap();
        let mut pending = vec![&analysis.global_scope];
        let mut selected = Vec::new();
        while let Some(scope) = pending.pop() {
            selected.extend(
                scope
                    .variables
                    .values()
                    .filter(|variable| variable.definition_span.start() == span),
            );
            pending.extend(&scope.children);
        }
        assert_eq!(selected.len(), 1);
        let declaration = analysis
            .original_variable_declaration_in_source(&image, config, selected[0])
            .expect("actual original alias declaration");
        let alias = declaration
            .alias_template()
            .expect("conditional alias only");
        assert!(declaration.original_occurrence().is_none());
        assert_eq!(alias.declaration_span(), selected[0].definition_span);
        assert_eq!(alias.local_name().as_bytes(), b"data");
        assert_eq!(alias.obligations().len(), 3);
        assert_eq!(declaration.symbol(), alias.target_symbol());
        assert_eq!(
            declaration.original_name_input(),
            alias.original_name_input()
        );
        assert_eq!(alias.original_frame().lexer_config(), config);
    }
}
