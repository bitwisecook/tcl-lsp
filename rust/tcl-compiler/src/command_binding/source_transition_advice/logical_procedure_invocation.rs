// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Positioned Logical procedure-header advice through the existing source cells.

use super::{
    AdviceCell, AdviceGraph, AdviceNamingPolicy, OriginalLogicalAbsoluteCommandCandidate,
    OriginalSourceCommandTransition, OriginalSourceTransitionAdviceTape, SourceAdviceWord,
    original_words,
};
use crate::analyser::types::LogicalSourceProcedureDeclaration;
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage};

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceRecipe {
    Named(Box<super::logical_navigation::OriginalLogicalVariableWords>),
    Absolute(Box<OriginalLogicalAbsoluteCommandCandidate>),
}

/// One actual original source call's conditional header selection. The original
/// definition transition, complete current argv, alias/move lineage, captured
/// prefix and caller source recipe remain held. This supplies no installed
/// procedure, actual call, Native formal grammar, entered frame or edit grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalProcedureCall {
    input: ResolvedAnalysisInput,
    original: Arc<[NativeWord]>,
    recipe: SourceRecipe,
    definition: Arc<OriginalSourceCommandTransition>,
    slot: tcl_core_types::ByteCommandSlot,
    prefix: Vec<SourceAdviceWord>,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
    earlier_unknown_mutations: bool,
    operand_effects: bool,
}
impl OriginalLogicalProcedureCall {
    /// Whole current written vector; captured prefix words never gain its spans.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Genuine selected original definition, independent of reporting labels.
    #[must_use]
    pub fn definition(&self) -> &OriginalSourceCommandTransition {
        &self.definition
    }
    /// Conditional source slot reached by the original cell/alias walk. This
    /// grants no installed command table, executed lookup or Native identity.
    #[must_use]
    pub const fn source_slot(&self) -> &tcl_core_types::ByteCommandSlot {
        &self.slot
    }
    /// Original alias/move recipes, without successful transition execution.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }
    /// Captured post-head values occupy slots without written cursor anchors.
    #[must_use]
    pub fn captured_argument_count(&self) -> usize {
        self.prefix.len()
    }
    /// Earlier and current operand effects remain source applicability debt.
    /// False still grants no installed lookup or entered procedure/frame.
    #[must_use]
    pub const fn has_unresolved_applicability(&self) -> bool {
        self.earlier_unknown_mutations || self.operand_effects
    }
    /// Exact full image/input/Registry and independently retained source recipe.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.input == *input
            && input.has_logical_source_name_context()
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && self
                .definition
                .original_words()
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && self.prefix.iter().all(|word| {
                word.original.image() == image && word.original.config() == input.lexer_config()
            })
            && match &self.recipe {
                SourceRecipe::Named(recipe) => {
                    super::logical_navigation::OriginalLogicalVariableWords::matches_original_source_words(recipe, image, input, &self.original)
                }
                SourceRecipe::Absolute(recipe) => {
                    recipe.matches_source(image, input)
                        && recipe.original_words() == self.original.as_ref()
                }
            }
    }
    fn matches_analysis(&self, analysis: &AnalysisResult) -> bool {
        let Some(input) = analysis.resolved_input.as_ref() else {
            return false;
        };
        let Some(head) = self.original.first() else {
            return false;
        };
        analysis.allows_retained_logical_declaration_advice()
            && analysis.matches_original_source_image(head.image(), input.lexer_config())
            && self.matches_source(head.image(), input)
            && analysis
                .retained_command_realm()
                .and_then(|realm| {
                    crate::realm::CommandBindingRealm::original_logical_procedure_call_at(
                        realm,
                        input,
                        head.span().start(),
                    )
                })
                .as_ref()
                == Some(self)
    }
    /// Join only the independently genuine header at this exact original
    /// definition site/vector. Missing/conflicting cards stay unavailable; no
    /// final reporting procedure map or rendered terminal spelling is searched.
    #[must_use]
    pub fn declaration<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a LogicalSourceProcedureDeclaration> {
        self.matches_analysis(analysis).then_some(())?;
        let mut declarations = analysis
            .logical_source_procedure_declarations()
            .filter(|header| {
                LogicalSourceProcedureDeclaration::matches_original_source_transition(
                    header,
                    analysis,
                    &self.definition,
                )
            });
        let header = declarations.next()?;
        declarations.next().is_none().then_some(header)
    }
    /// Cursor ordinals are source geometry only. Native expansion/list/formal
    /// policies remain separate; unrepresented Logical expansion stays unknown.
    #[must_use]
    pub fn active_parameter_for_cursor(
        &self,
        cursor: &tcl_lexer::word_parts::ArgumentCursor,
    ) -> Option<u32> {
        let head = self.original.first()?;
        let config = self
            .input
            .lexer_config()
            .at_depth(u32::from(cursor.region_span().start() != 0));
        if !cursor.matches_source(head.image(), cursor.region_span(), config)
            || cursor.head_span() != head.span()
            || cursor.written_word_count() != self.original.len()
            || self.prefix.iter().any(|word| word.original.group().expand)
        {
            return None;
        }
        let written = cursor.word_index();
        if written == 0
            || written > self.original.len()
            || (!cursor.is_empty_boundary() && written == self.original.len())
            || self
                .original
                .iter()
                .take(written.saturating_add(1))
                .skip(1)
                .any(|word| word.group().expand)
        {
            return None;
        }
        u32::try_from(self.prefix.len().checked_add(written.checked_sub(1)?)?).ok()
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(super) fn retain_logical_procedure_call(
        &mut self,
        graph: &AdviceGraph,
        original: &[NativeWord],
    ) {
        let Some(head) = original.first() else {
            return;
        };
        if !matches!(graph.policy, AdviceNamingPolicy::Logical(_)) {
            return;
        }
        let call = self.capture_logical_procedure_call(graph, original);
        super::source_class::merge_receipt(
            &mut self.logical_procedure_calls,
            head.span().start(),
            call,
        );
    }
    fn capture_logical_procedure_call(
        &self,
        graph: &AdviceGraph,
        original: &[NativeWord],
    ) -> Option<OriginalLogicalProcedureCall> {
        let AdviceNamingPolicy::Logical(input) = &graph.policy else {
            return None;
        };
        let head = original.first()?;
        let recipe = self
            .original_logical_source_word_context(input, original)
            .map(|recipe| SourceRecipe::Named(Box::new(recipe)))
            .or_else(|| {
                self.logical_absolute_navigation
                    .get(&head.span().start())
                    .and_then(Option::as_ref)
                    .map(|recipe| SourceRecipe::Absolute(Box::new(recipe.clone())))
            })?;
        let written = original_words(original, &graph.policy)?;
        let name = written.first()?.input.as_ref()?;
        name.logical_input()?;
        let mut selected: super::SourceAliasCellSelection<'_> = graph.resolve_source_cell(name)?;
        let AdviceCell::LogicalSourceProcedure {
            definition,
            lineage,
        } = selected.cell
        else {
            return None;
        };
        selected.lineage.extend(lineage.iter().cloned());
        for (ordinal, word) in selected.prefix.iter_mut().enumerate() {
            word.origin = crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        let call = OriginalLogicalProcedureCall {
            input: input.clone(),
            original: Arc::from(original),
            recipe,
            definition: Arc::clone(definition),
            slot: selected.slot,
            prefix: selected.prefix,
            lineage: selected.lineage,
            earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
            operand_effects: !super::body_effects::original_operand_effects(original).is_empty(),
        };
        call.matches_source(head.image(), input).then_some(call)
    }
    pub(crate) fn logical_procedure_call_at(
        &self,
        input: &ResolvedAnalysisInput,
        image: &SourceImage,
        offset: u32,
    ) -> Option<OriginalLogicalProcedureCall> {
        let call = self.logical_procedure_calls.get(&offset)?.as_ref()?;
        (call.original.first()?.span().start() == offset && call.matches_source(image, input))
            .then(|| call.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_logical_procedure_call_at;

    fn analyse(source: &str) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }
    #[test]
    fn logical_calls_keep_actual_prior_definition_alias_prefix_move_and_replacement() {
        let source = "proc join {first second} {}\ninterp alias {} joined {} join captured\njoined value\nrename join archived\nproc join {new} {}\narchived alpha beta\njoin one\n";
        let analysis = analyse(source);
        let alias = source_logical_procedure_call_at(
            source,
            &analysis,
            u32::try_from(source.find("joined value").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(alias.captured_argument_count(), 1);
        assert_eq!(
            alias
                .declaration(&analysis)
                .unwrap()
                .original_source_formals()
                .unwrap()
                .len(),
            2
        );
        for (needle, count) in [("archived alpha", 2), ("join one", 1)] {
            let call = source_logical_procedure_call_at(
                source,
                &analysis,
                u32::try_from(source.find(needle).unwrap()).unwrap(),
            )
            .unwrap();
            assert_eq!(
                call.declaration(&analysis)
                    .unwrap()
                    .original_source_formals()
                    .unwrap()
                    .len(),
                count
            );
        }
        let mut reports = analysis.clone();
        reports.all_procs.clear();
        assert_eq!(
            alias
                .declaration(&reports)
                .unwrap()
                .original_source_formals()
                .unwrap()
                .len(),
            2
        );
    }
    #[test]
    fn logical_call_selection_retains_unknown_debt_and_refuses_deleted_or_dynamic_heads() {
        let source = "proc helper {value} {}\nmystery\nhelper $value\nrename helper {}\nhelper ignored\n$head data\n";
        let analysis = analyse(source);
        let call = source_logical_procedure_call_at(
            source,
            &analysis,
            u32::try_from(source.find("helper $value").unwrap()).unwrap(),
        )
        .unwrap();
        assert!(call.has_unresolved_applicability());
        assert!(call.declaration(&analysis).is_some());
        for needle in ["helper ignored", "$head data"] {
            assert!(
                source_logical_procedure_call_at(
                    source,
                    &analysis,
                    u32::try_from(source.find(needle).unwrap()).unwrap()
                )
                .is_none()
            );
        }
        assert!(
            call.declaration(&Analyser::new().analyse(source, "tcl8.6"))
                .is_none()
        );
    }
    #[test]
    fn logical_header_join_rejects_missing_source_input_config_and_foreign_word_data() {
        let source = "proc helper {value} {}\nhelper data\nset data helper\n";
        let analysis = analyse(source);
        let offset = u32::try_from(source.find("helper data").unwrap()).unwrap();
        let call = source_logical_procedure_call_at(source, &analysis, offset).unwrap();
        assert!(
            source_logical_procedure_call_at(
                source,
                &analysis,
                u32::try_from(source.rfind("helper").unwrap()).unwrap()
            )
            .is_none()
        );
        assert!(
            source_logical_procedure_call_at(
                "proc helper {value} {}\nhelper other\nset data helper\n",
                &analysis,
                offset
            )
            .is_none()
        );
        let mut stale = analysis.clone();
        stale.body_lexer_config = None;
        assert!(call.declaration(&stale).is_none());
        let mut stale = analysis.clone();
        stale.resolved_input = None;
        assert!(call.declaration(&stale).is_none());
    }
}
