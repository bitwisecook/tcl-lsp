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

    /// Current original body geometry for source readers. This does not
    /// authorise evaluation or editing of the body.
    #[must_use]
    pub fn original_body_word(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> Option<&tcl_lexer::NativeWord> {
        self.matches_source(image, input).then_some(())?;
        Some(self.body.scope().scope_body().original_container())
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
            .chain(
                self.original_procedure_declarations()
                    .map(|declaration| declaration.metadata()),
            )
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
    fn logical_procedure_body_geometry_keeps_actual_input_and_original_word() {
        let source = "proc helper {value} {puts \"literal \\} ${value}\"}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let procedure = &analysis.all_procs["::helper"];
        let declaration = analysis
            .logical_source_procedure_declaration(procedure)
            .unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = SourceImage::document(source);
        let word = declaration.original_body_word(&image, input).unwrap();
        assert_eq!(
            source.get(word.content_span().unwrap().as_range()),
            Some("puts \"literal \\} ${value}\"")
        );
        assert!(
            declaration
                .original_body_word(&SourceImage::document("changed"), input)
                .is_none()
        );
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let different = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        assert!(declaration.original_body_word(&image, &different).is_none());
    }

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
    fn logical_procedure_insertions_keep_nominal_name_metadata_without_native_admission() {
        let source = "namespace eval N {proc helper {arg} {return $arg}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        let procedure = &analysis.all_procs["::N::helper"];
        assert!(procedure.source_name.is_some());
        let declaration = analysis
            .logical_source_procedure_declaration(procedure)
            .expect("nominal source name accompanies the genuine Logical header");
        assert_eq!(
            declaration
                .source_word(analysis.body_lexer_config.unwrap())
                .as_deref(),
            Some("::N::helper")
        );
        let mut changed = analysis.clone();
        changed.resolved_input = None;
        assert!(
            changed
                .logical_source_procedure_declaration(&changed.all_procs["::N::helper"])
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
    #[test]
    fn logical_body_geometry_accepts_actual_original_records_and_rejects_copies() {
        let source = "proc p {x} {expr {$x + 1}}\np 2";
        let analysis = Analyser::new().analyse(source, "tcl");
        let record = analysis.original_procedure_declarations().next().unwrap();
        let procedure = record.metadata();
        assert!(!std::ptr::eq(procedure, &analysis.all_procs["::p"]));
        let declaration = analysis
            .logical_source_procedure_declaration(procedure)
            .expect("the actual retained original record shares its Logical body");
        let image = SourceImage::document(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        assert_eq!(
            declaration
                .original_body_word(&image, input)
                .unwrap()
                .content_span()
                .ok()
                .and_then(|span| source.get(span.as_range())),
            Some("expr {$x + 1}")
        );
        assert!(
            analysis
                .logical_source_procedure_declaration(&procedure.clone())
                .is_none()
        );
        let foreign = Analyser::new().analyse(source, "tcl");
        assert!(
            analysis
                .logical_source_procedure_declaration(
                    foreign
                        .original_procedure_declarations()
                        .next()
                        .unwrap()
                        .metadata(),
                )
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            missing
                .logical_source_procedure_declaration(
                    missing
                        .original_procedure_declarations()
                        .next()
                        .unwrap()
                        .metadata(),
                )
                .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_procedure_declaration(
                    native
                        .original_procedure_declarations()
                        .next()
                        .unwrap()
                        .metadata(),
                )
                .is_none()
        );
    }
}
