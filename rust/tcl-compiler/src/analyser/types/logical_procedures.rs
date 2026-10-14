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

//! Original Logical procedure names for readonly insertion advice.

use super::{AnalysisResult, ProcDef};
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::OriginalLogicalProcedureBody;
use std::sync::Arc;
use tcl_lexer::{LexerConfig, SourceImage};

/// Genuine Logical header and source namespace, without a callable publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceProcedureDeclaration {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    body: Arc<OriginalLogicalProcedureBody>,
    metadata: ProcDef,
}

impl LogicalSourceProcedureDeclaration {
    pub(crate) fn capture(
        analysis: &AnalysisResult,
        source: &str,
        input: ResolvedAnalysisInput,
        procedure: &ProcDef,
    ) -> Option<Self> {
        let realm = analysis.retained_command_realm()?;
        let image = realm.original_source_image()?;
        if image.bytes() != source.as_bytes()
            || !input.has_logical_source_name_context()
            || !realm.matches_resolved_analysis_input(&input)
            || procedure.source_name.is_some()
            || procedure.params_computed
        {
            return None;
        }
        let cards = realm.original_logical_procedure_body_cards(&input)?;
        let mut cards = cards.into_iter().filter(|card| {
            card.matches_source(image, &input)
                && card.scope().scope_name_input().original_word().group().span
                    == procedure.name_span
                && card.scope().scope_body().extent_span() == procedure.body_span
                && card
                    .parameters()
                    .iter()
                    .map(|parameter| (parameter.name.as_str(), parameter.default.as_deref()))
                    .eq(procedure.params.iter().map(|parameter| {
                        (parameter.name.as_str(), parameter.default_value.as_deref())
                    }))
        });
        let body = cards.next()?;
        cards.next().is_none().then_some(Self {
            image: image.clone(),
            input,
            body,
            metadata: procedure.clone(),
        })
    }

    /// Original simple name from the header's source coordinate.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        std::str::from_utf8(self.body.summary_coordinate().simple.as_bytes()).ok()
    }

    /// Quoted insertion data from the retained source namespace and header.
    #[must_use]
    pub fn source_word(&self, config: LexerConfig) -> Option<String> {
        let value =
            tcl_syntax::naming::authored_source_command_spelling(self.body.summary_coordinate())?;
        tcl_syntax::word_rules::lexical_ascii_source_word(&value, config)
    }

    /// Original declared metadata for presentation, never invocation admission.
    #[must_use]
    pub fn metadata(&self) -> &ProcDef {
        &self.metadata
    }

    /// Indexed source and input must still be the exact original declaring owner.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        image == &self.image && input == &self.input && self.body.matches_source(image, input)
    }

    fn matches(&self, analysis: &AnalysisResult, procedure: &ProcDef) -> bool {
        if !analysis.allows_retained_logical_declaration_advice()
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || !analysis.matches_original_source_image(&self.image, self.input.lexer_config())
            || !analysis
                .retained_command_realm()
                .is_some_and(|realm| realm.matches_resolved_analysis_input(&self.input))
            || !self.body.matches_source(&self.image, &self.input)
        {
            return false;
        }
        let mut current = procedure.clone();
        current.name.clone_from(&self.metadata.name);
        current
            .qualified_name
            .clone_from(&self.metadata.qualified_name);
        current == self.metadata
    }
}

impl AnalysisResult {
    /// Current actual procedure metadata must agree with the genuine header card.
    /// Reporting names are excluded from selection and insertion.
    #[must_use]
    pub fn logical_source_procedure_declaration(
        &self,
        procedure: &ProcDef,
    ) -> Option<&LogicalSourceProcedureDeclaration> {
        self.all_procs
            .values()
            .any(|current| std::ptr::eq(current, procedure))
            .then_some(())?;
        let mut declarations = self
            .logical_procedure_declarations
            .iter()
            .filter(|declaration| declaration.matches(self, procedure));
        let selected = declarations.next()?;
        declarations.next().is_none().then_some(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_procedure_insertions_keep_original_namespace_and_current_header() {
        let source = "namespace eval N {proc helper {arg} {return $arg}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let procedure = &analysis.all_procs["::N::helper"];
        let declaration = analysis
            .logical_source_procedure_declaration(procedure)
            .expect("actual Logical original header");
        let config = analysis.body_lexer_config.unwrap();
        assert_eq!(declaration.name(), Some("helper"));
        assert_eq!(
            declaration.source_word(config).as_deref(),
            Some("::N::helper")
        );
        assert!(
            analysis
                .logical_source_procedure_declaration(&procedure.clone())
                .is_none()
        );
        let mut labels = analysis.clone();
        let procedure = labels.all_procs.get_mut("::N::helper").unwrap();
        procedure.name = "display".into();
        procedure.qualified_name = "::foreign::display".into();
        let declaration = labels
            .logical_source_procedure_declaration(&labels.all_procs["::N::helper"])
            .unwrap();
        assert_eq!(
            declaration.source_word(config).as_deref(),
            Some("::N::helper")
        );
        let mut changed = analysis.clone();
        changed
            .all_procs
            .get_mut("::N::helper")
            .unwrap()
            .params
            .clear();
        assert!(
            changed
                .logical_source_procedure_declaration(&changed.all_procs["::N::helper"])
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            missing
                .logical_source_procedure_declaration(&missing.all_procs["::N::helper"])
                .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_procedure_declaration(&native.all_procs["::N::helper"])
                .is_none()
        );
    }
    #[test]
    fn logical_procedure_insertions_match_original_fold_extents() {
        for source in [
            "namespace eval N {proc {helper} {arg} {return $arg}}",
            "namespace eval N {proc \"helper\" {arg} {return $arg}}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let procedure = &analysis.all_procs["::N::helper"];
            let declaration = analysis
                .logical_source_procedure_declaration(procedure)
                .expect("original grouped header and body fold geometry");
            assert_eq!(declaration.name(), Some("helper"));
            assert_eq!(
                declaration
                    .source_word(analysis.body_lexer_config.unwrap())
                    .as_deref(),
                Some("::N::helper")
            );
            let mut changed = analysis.clone();
            let procedure = changed.all_procs.get_mut("::N::helper").unwrap();
            procedure.body_span =
                tcl_lexer::Span::new(procedure.body_span.start() + 1, procedure.body_span.end());
            assert!(
                changed
                    .logical_source_procedure_declaration(&changed.all_procs["::N::helper"])
                    .is_none()
            );
        }
    }
}
