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

//! `AnalysisResult` queries over its retained inputs: the optimiser request, analysis entry and region, the command realm, profile and registry, and math functions in source.

use super::AnalysisResult;

impl AnalysisResult {
    /// Rejoin the actual source-build request retained by the diagnostic producer.
    /// This does not grant worker, final group, edit or runtime authority.
    #[must_use]
    pub fn matches_optimiser_source_origin(
        &self,
        origin: &crate::compilation_unit::OriginalOptimiserSourceOrigin,
    ) -> bool {
        self.optimiser_source_origin.as_ref() == Some(origin)
            && origin.is_current_analysis_context()
    }

    /// Reborrow the actual retained source-build issuer after the complete
    /// current source/input/Registry/evaluator request joins. Identity alone
    /// supplies no token, invocation, body or edit permission.
    pub(crate) fn original_optimiser_request_owner(
        &self,
        source: &str,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&crate::compilation_unit::OriginalOptimiserSourceOrigin> {
        if self.analysis_context_unavailable.is_some() {
            return None;
        }
        crate::source_graph::current_analysis(source, self)?;
        let input = self.resolved_input.as_ref()?;
        let origin = self.optimiser_source_origin.as_ref()?;
        origin.source_analysis_entry_for_current_analysis(source, registry, input, None)?;
        Some(origin)
    }

    /// Rebuild the actual retained source request once for an uncached report.
    /// The source/config/Registry and independently requested external evidence
    /// must still match. This supplies no Native entry or edit permission;
    /// optimiser findings need their own complete worker and final group.
    #[must_use]
    pub fn original_optimiser_source_unit(
        &self,
        source: &str,
        registry: &tcl_registry::CommandRegistry,
        external: Option<&crate::unit_scope::CallSiteEvidence>,
    ) -> Option<crate::compilation_unit::CompilationUnit> {
        if self.analysis_context_unavailable.is_some() {
            return None;
        }
        crate::source_graph::current_analysis(source, self)?;
        let input = self.resolved_input.as_ref()?;
        self.optimiser_source_origin
            .as_ref()?
            .rebuild_for_current_analysis(source, registry, input, external)
    }

    /// Reborrow the actual source entry without rebuilding a unit. Missing or
    /// stale ownership refuses; no fresh entry is substituted for this request.
    #[must_use]
    pub fn original_source_analysis_entry(
        &self,
        source: &str,
        registry: &tcl_registry::CommandRegistry,
        external: Option<&crate::unit_scope::CallSiteEvidence>,
    ) -> Option<std::sync::Arc<crate::command_binding::SourceAnalysisEntry>> {
        if self.analysis_context_unavailable.is_some() {
            return None;
        }
        crate::source_graph::current_analysis(source, self)?;
        let input = self.resolved_input.as_ref()?;
        let realm = self.retained_command_realm()?;
        realm.matches_resolved_analysis_input(input).then_some(())?;
        let entry = self
            .optimiser_source_origin
            .as_ref()?
            .source_analysis_entry_for_current_analysis(source, registry, input, external)?;
        if entry
            .source_point_entry
            .as_ref()
            .is_some_and(|point| !point.matches_child_analysis(self))
        {
            return None;
        }
        Some(entry)
    }

    /// Exact retained partial-region owner. Whole-document query APIs must not
    /// treat this as complete coverage; hosts retain publication leases separately.
    #[must_use]
    pub fn original_source_region(
        &self,
    ) -> Option<&crate::command_binding::OriginalSourceConditionalRegion> {
        let region = self.original_source_region.as_ref()?;
        region.matches_region_analysis(self).then_some(region)
    }

    /// Conditional expression syntax in an incomplete final operand.
    /// This supplies no evaluated argument, invocation, or dispatch fact.
    #[must_use]
    pub fn original_partial_expression_span(&self, source: &str) -> Option<tcl_lexer::Span> {
        let input = self.resolved_input.as_ref()?;
        let config = self.body_lexer_config?;
        if config != input.lexer_config()
            || !self
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        {
            return None;
        }
        let realm = self.retained_command_realm()?;
        if !realm.matches_resolved_analysis_input(input) {
            return None;
        }
        realm
            .source_bindings_ref()
            .original_closed_prefix_expression_span(input)
    }

    /// Exact temporal source world used by this analysis. Missing evidence
    /// remains unknown; rebuilding a realm from the dialect label cannot
    /// replace its native entry, provider or source-instance obligations.
    #[must_use]
    pub fn retained_command_realm(&self) -> Option<&crate::realm::CommandBindingRealm> {
        self.command_realm.as_deref()
    }

    /// Whether this is a source shell ([`super::super::Analyser::source_shell`]): the
    /// document's identity with no command interpreted. Display consumers read
    /// written words from it; it holds no fact for any other consumer.
    #[must_use]
    pub fn is_source_shell(&self) -> bool {
        self.command_realm
            .as_deref()
            .is_some_and(|realm| realm.source_bindings_ref().is_unentered())
    }

    /// Borrow the same immutable interpretation owner for source-derived
    /// batch currency. Arc identity grants no invocation or execution facts.
    pub(crate) fn retained_command_realm_arc(
        &self,
    ) -> Option<&std::sync::Arc<crate::realm::CommandBindingRealm>> {
        self.command_realm.as_ref()
    }

    /// Whether the root's final modelled command world holds an unknown
    /// command-binding mutation (for example `eval $script`). `None` when no
    /// command realm was retained. This is narrower than an absent
    /// [`Self::original_completed_command_world`], which any possible
    /// abrupt or unknown route also withholds.
    #[must_use]
    pub fn root_unknown_command_mutation(&self) -> Option<bool> {
        Some(
            self.retained_command_realm()?
                .source_bindings_ref()
                .final_world_has_unknown_command_mutation(),
        )
    }

    /// Independently completed Normal source publication world. This is final
    /// source liveness advice, separate from positioned invocation lookup and
    /// physical interpreter existence. Unknown or abrupt source stays absent.
    #[must_use]
    pub const fn original_completed_command_world(
        &self,
    ) -> Option<&crate::command_binding::OriginalCompletedCommandWorld> {
        self.original_command_world.as_ref()
    }

    /// Actual compilation profile retained at analysis ingress. Assistance
    /// records without retained inputs leave this unknown.
    #[must_use]
    pub fn resolved_profile(&self) -> Option<&'static tcl_dialect::DialectProfile> {
        self.resolved_input.as_ref().map(|input| input.unit_profile)
    }

    /// Complete original analysis/source correspondence, including input
    /// channel and every lexer override. This supplies no current value,
    /// completed execution, namespace existence or native capability.
    #[must_use]
    pub fn matches_original_source_image(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        if self.analysis_context_unavailable.is_some() || self.body_lexer_config != Some(config) {
            return false;
        }
        let Some(input) = self.resolved_input.as_ref() else {
            return false;
        };
        self.retained_command_realm().is_some_and(|realm| {
            realm.matches_resolved_analysis_input(input)
                && input.lexer_config() == config
                && realm
                    .source_bindings_ref()
                    .matches_original_source_image(image, config)
        })
    }

    /// Checked original expression function at this source position. Fixed
    /// native tables and command-table navigation retain separate purposes;
    /// this supplies no command-head word, evaluation or compiler admission.
    #[must_use]
    pub fn original_math_function_at_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<crate::command_binding::OriginalMathFunctionOccurrence> {
        let registry = self.resolved_registry()?;
        let end = offset.checked_add(1)?;
        if usize::try_from(end).ok()? > image.bytes().len() {
            return None;
        }
        let matches = self.original_math_functions_in_source(
            image,
            config,
            tcl_lexer::Span::new(offset, end),
        )?;
        let mut selected = matches.into_iter().filter(|function| {
            let span = function.span();
            offset >= span.start()
                && offset < span.end()
                && function.matches_source(image, config, registry)
        });
        let first = selected.next()?;
        selected.all(|other| other == first).then_some(first)
    }

    /// Readonly checked function occurrences in an original source region.
    /// The complete image, channel, lexer configuration and retained Registry
    /// must agree. Missing ownership returns `None`, independently of an empty
    /// occurrence list; no body entry or runtime function roster is issued.
    #[must_use]
    pub fn original_math_functions_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        region: tcl_lexer::Span,
    ) -> Option<Vec<crate::command_binding::OriginalMathFunctionOccurrence>> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        if region.start() > region.end() || usize::try_from(region.end()).ok()? > image.len() {
            return None;
        }
        let registry = self.resolved_registry()?;
        let rows = self
            .retained_command_realm()?
            .source_bindings_ref()
            .original_math_functions_in_source(registry, image, config, region);
        rows.iter()
            .all(|row| row.matches_source(image, config, registry))
            .then_some(rows)
    }

    /// Exact editing command store retained with its availability generation.
    /// It supplies metadata, independently of reached command identities.
    #[must_use]
    pub fn resolved_registry(&self) -> Option<&tcl_registry::CommandRegistry> {
        self.resolved_input
            .as_ref()
            .map(|input| input.context.commands().as_ref())
    }
}
