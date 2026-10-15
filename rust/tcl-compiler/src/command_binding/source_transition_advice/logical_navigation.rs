// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional Logical workspace candidates retain their actual source occurrence.

use super::{AdviceGraph, AdviceNamingPolicy, OriginalSourceTransitionAdviceTape};
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::OriginalDeclaredLogicalBodyContext;
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::NativeWord;

/// Ordered Logical source coordinates at one authentic original call occurrence.
/// Unavailable actual lookup and unmodelled mutations remain applicability
/// obligations; this is neither an installation nor a resolved call edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalCommandCandidates {
    input: ResolvedAnalysisInput,
    original: Arc<[NativeWord]>,
    scope: Arc<OriginalDeclaredLogicalBodyContext>,
    candidates: Vec<ByteCommandSlot>,
    earlier_unknown_mutations: bool,
}

impl OriginalLogicalCommandCandidates {
    fn capture(
        graph: &AdviceGraph,
        original: &[NativeWord],
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
    ) -> Option<Self> {
        let AdviceNamingPolicy::Logical(input) = &graph.policy else {
            return None;
        };
        if !scope.matches_input(input)
            || !scope.owns_words(original)
            || graph.logical_namespace.as_ref() != Some(scope.namespace_recipe())
        {
            return None;
        }
        let head = original.first()?;
        if head.group().expand {
            return None;
        }
        let written = tcl_syntax::word_rules::original_static_word_unicode_value(head)?;
        let namespace = scope.namespace_recipe();
        if !written.starts_with(b"::") && graph.unknown_paths.contains(namespace) {
            return None;
        }
        let mut candidates = tcl_syntax::naming::authored_source_command_candidates(
            namespace,
            graph
                .namespace_paths
                .get(namespace)
                .map_or(&[], Vec::as_slice),
            &written,
        )?;
        for candidate in &mut candidates {
            let mut seen = Vec::new();
            while let Some(target) = graph.namespace_imports.get(candidate) {
                if seen.contains(candidate) {
                    return None;
                }
                seen.push(candidate.clone());
                *candidate = target.clone();
            }
        }
        Some(Self {
            input: input.clone(),
            original: Arc::from(original),
            scope: Arc::clone(scope),
            candidates,
            earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
        })
    }

    /// Ordered conditional coordinates, without existence or dispatch authority.
    #[must_use]
    pub fn coordinates(&self) -> &[ByteCommandSlot] {
        &self.candidates
    }

    /// Whether an earlier unmodelled source operation adds an applicability premise.
    /// Even false grants no workspace loading or actual command lookup.
    #[must_use]
    pub const fn has_earlier_unknown_mutations(&self) -> bool {
        self.earlier_unknown_mutations
    }

    /// Same complete Logical source/input and authentic retained body ancestry.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input
            && input.has_logical_source_name_context()
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && self.scope.matches_input(input)
            && self.scope.owns_words(&self.original)
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(super) fn retain_logical_navigation(
        &mut self,
        graph: &AdviceGraph,
        original: &[NativeWord],
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
    ) {
        let Some(head) = original.first() else {
            return;
        };
        super::source_class::merge_receipt(
            &mut self.logical_navigation,
            head.span().start(),
            OriginalLogicalCommandCandidates::capture(graph, original, scope),
        );
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn logical_navigation_at(
        &self,
        input: &ResolvedAnalysisInput,
        original: &[NativeWord],
    ) -> Option<OriginalLogicalCommandCandidates> {
        let candidate = self
            .logical_navigation
            .get(&original.first()?.span().start())?
            .as_ref()?;
        (candidate.original.as_ref() == original
            && candidate.matches_source(original.first()?.image(), input))
        .then(|| candidate.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_logical_command_candidates_at;

    fn query(
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
        needle: &str,
    ) -> Option<OriginalLogicalCommandCandidates> {
        source_logical_command_candidates_at(
            source,
            analysis,
            u32::try_from(source.find(needle)?).ok()?,
        )
    }

    #[test]
    fn logical_navigation_candidates_keep_each_original_namespace_occurrence_and_current_input() {
        let source = "proc caller {} {namespace eval A {helper}; namespace eval B {helper}}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let first = query(source, &analysis, "helper").unwrap();
        let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
        let second = source_logical_command_candidates_at(source, &analysis, offset).unwrap();
        let root = tcl_core_types::ByteNamespacePath::root();
        let a = tcl_syntax::naming::authored_source_namespace_path(&root, b"A").unwrap();
        let b = tcl_syntax::naming::authored_source_namespace_path(&root, b"B").unwrap();
        assert_eq!(
            first.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&a, &[], b"helper").unwrap()
        );
        assert_eq!(
            second.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&b, &[], b"helper").unwrap()
        );
        assert_ne!(first.coordinates(), second.coordinates());
        assert!(query(&format!("{source}\n# changed"), &analysis, "helper").is_none());
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(query(source, &analysis, "helper").is_none());
        analysis.body_lexer_config = Some(config);
        analysis.resolved_input = None;
        assert!(query(source, &analysis, "helper").is_none());
    }

    #[test]
    fn logical_navigation_candidates_use_the_original_path_and_refuse_unowned_heads() {
        let source =
            "namespace eval B {}; namespace eval A {namespace path {::B}; proc caller {} {helper}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let request = query(source, &analysis, "helper").unwrap();
        let root = tcl_core_types::ByteNamespacePath::root();
        let a = tcl_syntax::naming::authored_source_namespace_path(&root, b"A").unwrap();
        let b = tcl_syntax::naming::authored_source_namespace_path(&root, b"B").unwrap();
        assert_eq!(
            request.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&a, &[b], b"helper").unwrap()
        );
        for (source, needle) in [
            ("proc caller {} {$head}", "$head"),
            ("proc caller {} {{*}$heads}", "{*}"),
            ("proc caller {} {puts {helper}}", "helper"),
            (
                "namespace eval A {namespace path $paths; proc caller {} {helper}}",
                "helper",
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            assert!(query(source, &analysis, needle).is_none(), "{source}");
        }
        let source = "proc caller {} {helper}";
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(query(source, &native, "helper").is_none());
    }
}
