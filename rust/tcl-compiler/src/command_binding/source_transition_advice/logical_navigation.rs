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
        let candidates = logical_coordinates(graph, namespace, &written)?;
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

fn logical_coordinates(
    graph: &AdviceGraph,
    namespace: &tcl_core_types::ByteNamespacePath,
    written: &[u8],
) -> Option<Vec<ByteCommandSlot>> {
    if !written.starts_with(b"::") && graph.unknown_paths.contains(namespace) {
        return None;
    }
    let mut candidates = tcl_syntax::naming::authored_source_command_candidates(
        namespace,
        graph
            .namespace_paths
            .get(namespace)
            .map_or(&[], Vec::as_slice),
        written,
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
    Some(candidates)
}

/// Conditional callback candidates from an actual selected Registry prefix role.
/// The whole installer, original list field and source frame recipe remain held;
/// these coordinates prove neither callback reach nor an installed procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalCallbackCandidates {
    registration: super::OriginalSourceCommandTransitionAdvice,
    scope: Option<Arc<OriginalDeclaredLogicalBodyContext>>,
    head: tcl_syntax::word_rules::OriginalSourceListElement,
    candidates: Vec<ByteCommandSlot>,
    appended: tcl_registry::AppendedArity,
    baked: usize,
}

impl OriginalLogicalCallbackCandidates {
    /// Independent original installer schema and all applicability obligations.
    #[must_use]
    pub const fn registration(&self) -> &super::OriginalSourceCommandTransitionAdvice {
        &self.registration
    }
    /// Exact source list field; cooked values never manufacture a word or range.
    #[must_use]
    pub const fn head(&self) -> &tcl_syntax::word_rules::OriginalSourceListElement {
        &self.head
    }
    /// Ordered conditional source coordinates, without actual lookup authority.
    #[must_use]
    pub fn coordinates(&self) -> &[ByteCommandSlot] {
        &self.candidates
    }
    /// Arguments the selected callback descriptor appends.
    #[must_use]
    pub const fn appended_arity(&self) -> tcl_registry::AppendedArity {
        self.appended
    }
    /// Original static list elements after the callback head.
    #[must_use]
    pub const fn baked_argument_count(&self) -> usize {
        self.baked
    }
    /// Same complete source/input and, where present, genuine declared ancestry.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        self.registration.logical_source_input() == Some(input)
            && self
                .registration
                .matches_source(image, input.lexer_config())
            && input.has_logical_source_name_context()
            && self.scope.as_ref().is_none_or(|scope| {
                scope.matches_input(input) && scope.owns_words(self.registration.original_words())
            })
    }
}

#[derive(Clone, Copy)]
pub(super) enum LogicalCallbackScope<'a> {
    Document,
    Declared(&'a Arc<OriginalDeclaredLogicalBodyContext>),
    Unavailable,
}

impl super::AdviceInvocationContext<'_> {
    pub(super) fn retain_logical_callback_candidates(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: super::AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        frame: LogicalCallbackScope<'_>,
    ) {
        let AdviceNamingPolicy::Logical(input) = self.policy else {
            return;
        };
        let Some(offset) = invocation.native.first().map(|head| head.span().start()) else {
            return;
        };
        let receipt = (|| {
            if schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
            }) != tcl_registry::InterpreterScope::Current
            {
                return None;
            }
            let prefixes = schema.authored_logical_source_command_prefix_arguments()?;
            let mut registration = self.schema_advice(invocation, graph, schema)?;
            let scope = match frame {
                LogicalCallbackScope::Declared(scope) => {
                    if !scope.matches_input(input)
                        || !scope.owns_words(invocation.native)
                        || graph.logical_namespace.as_ref() != Some(scope.namespace_recipe())
                    {
                        return None;
                    }
                    registration.logical_scope = Some(Arc::clone(scope));
                    Some(Arc::clone(scope))
                }
                LogicalCallbackScope::Document | LogicalCallbackScope::Unavailable => None,
            };
            let mut result = Vec::new();
            for (ordinal, appended) in prefixes {
                let argument = invocation.arguments.get(ordinal)?;
                let crate::registry_invocation::InvocationWordOrigin::Written(written) =
                    argument.origin
                else {
                    continue;
                };
                if invocation.native.get(written) != Some(&argument.original)
                    || argument.original.group().expand
                {
                    continue;
                }
                let root = tcl_core_types::ByteNamespacePath::root();
                let namespace = match schema.facts().script_lookup_scope(ordinal) {
                    Some(tcl_registry::ScriptLookupScope::GlobalFrame) => &root,
                    Some(tcl_registry::ScriptLookupScope::InvokingFrame) => match frame {
                        LogicalCallbackScope::Document => &root,
                        LogicalCallbackScope::Declared(scope) => scope.namespace_recipe(),
                        LogicalCallbackScope::Unavailable => continue,
                    },
                    Some(tcl_registry::ScriptLookupScope::TriggerFrame) | None => continue,
                };
                let Some(elements) =
                    tcl_syntax::word_rules::original_static_word_list_elements(&argument.original)
                else {
                    continue;
                };
                let Some(head) = elements.first() else {
                    continue;
                };
                let Some(candidates) =
                    logical_coordinates(graph, namespace, head.value().as_bytes())
                else {
                    continue;
                };
                result.push(OriginalLogicalCallbackCandidates {
                    registration: registration.clone(),
                    scope: scope.clone(),
                    head: head.clone(),
                    candidates,
                    appended,
                    baked: elements.len().checked_sub(1)?,
                });
            }
            Some((Arc::from(invocation.native), result))
        })();
        super::source_class::merge_receipt(&mut tape.logical_callbacks, offset, receipt);
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn logical_callback_inventory(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
    ) -> Vec<OriginalLogicalCallbackCandidates> {
        self.logical_callbacks
            .iter()
            .filter_map(|(offset, receipt)| {
                let (original, candidates) = receipt.as_ref()?;
                (original.first()?.span().start() == *offset).then_some((original, candidates))
            })
            .flat_map(|(original, candidates)| {
                candidates.iter().filter(move |candidate| {
                    candidate.registration.original_words() == original.as_ref()
                        && candidate.matches_source(image, input)
                })
            })
            .cloned()
            .collect()
    }

    pub(crate) fn logical_callbacks_at(
        &self,
        input: &ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> Option<Vec<OriginalLogicalCallbackCandidates>> {
        let (original, candidates) = self.logical_callbacks.get(&offset)?.as_ref()?;
        if original.first()?.span().start() != offset
            || original
                .iter()
                .any(|word| word.image() != image || word.config() != input.lexer_config())
        {
            return None;
        }
        candidates
            .iter()
            .all(|candidate| {
                candidate.registration.original_words() == original.as_ref()
                    && candidate.matches_source(image, input)
            })
            .then(|| candidates.clone())
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
        image: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> Option<OriginalLogicalCommandCandidates> {
        let candidate = self.logical_navigation.get(&offset)?.as_ref()?;
        (candidate.original.first()?.span().start() == offset
            && candidate.matches_source(image, input))
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
    fn logical_candidate_queries_retain_body_bounds_and_require_an_exact_original_head() {
        use crate::registry_invocation::source_structure::source_logical_callback_candidates_at;
        let source = "proc caller {} {namespace eval A {helper}; after 1 {handler ARG}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let helper = u32::try_from(source.find("helper").unwrap()).unwrap();
        let candidate = source_logical_command_candidates_at(source, &analysis, helper)
            .expect("original body command before its closing delimiter");
        let root = tcl_core_types::ByteNamespacePath::root();
        let namespace = tcl_syntax::naming::authored_source_namespace_path(&root, b"A").unwrap();
        assert_eq!(
            candidate.coordinates().first().unwrap().namespace,
            namespace
        );
        assert_eq!(
            candidate.coordinates().first().unwrap().simple.as_bytes(),
            b"helper"
        );
        let installer = u32::try_from(source.find("after").unwrap()).unwrap();
        let callbacks = source_logical_callback_candidates_at(source, &analysis, installer)
            .expect("original installer vector before its enclosing closing delimiter");
        assert_eq!(callbacks.len(), 1);
        assert_eq!(callbacks[0].head().value().as_bytes(), b"handler");
        for offset in [helper + 1, helper + 6, installer + 1] {
            assert!(source_logical_command_candidates_at(source, &analysis, offset).is_none());
            assert!(source_logical_callback_candidates_at(source, &analysis, offset).is_none());
        }
        let mut missing = analysis.clone();
        missing.body_lexer_config = None;
        assert!(source_logical_command_candidates_at(source, &missing, helper).is_none());
        assert!(source_logical_callback_candidates_at(source, &missing, installer).is_none());
        assert!(
            source_logical_command_candidates_at(&format!("{source} "), &analysis, helper)
                .is_none()
        );
        assert!(
            source_logical_callback_candidates_at(&format!("{source} "), &analysis, installer)
                .is_none()
        );
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
    #[test]
    fn logical_callback_candidates_retain_selected_prefix_and_source_scope() {
        use crate::registry_invocation::source_structure::source_logical_callback_candidates_at;
        let source = "namespace eval A {proc caller {} {lsort -command {helper baked} {a b}}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(source.find("lsort").unwrap()).unwrap();
        let callbacks = source_logical_callback_candidates_at(source, &analysis, offset).unwrap();
        assert_eq!(callbacks.len(), 1);
        let callback = &callbacks[0];
        let root = tcl_core_types::ByteNamespacePath::root();
        let a = tcl_syntax::naming::authored_source_namespace_path(&root, b"A").unwrap();
        assert_eq!(
            callback.coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(&a, &[], b"helper").unwrap()
        );
        assert_eq!(callback.head().value(), "helper");
        assert_eq!(callback.baked_argument_count(), 1);
        assert_eq!(
            callback
                .appended_arity()
                .exact_counts()
                .unwrap()
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(
            callback.registration().original_words()[0].span().start(),
            offset
        );
        let extent = callback.head().source_span().unwrap();
        assert_eq!(
            &source[extent.start() as usize..extent.end() as usize],
            "helper"
        );
        assert!(
            source_logical_callback_candidates_at(
                &format!("{source}\n# changed"),
                &analysis,
                offset
            )
            .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(source_logical_callback_candidates_at(source, &missing, offset).is_none());
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(source_logical_callback_candidates_at(source, &native, offset).is_none());
    }

    #[test]
    fn logical_callback_candidates_refuse_computed_expanded_and_unowned_method_frames() {
        use crate::registry_invocation::source_structure::source_logical_callback_candidates_at;
        for source in [
            "proc caller {} {lsort -command $prefix {a b}}",
            "proc caller {} {lsort {*}{-command helper} {a b}}",
            "oo::class create C {method m {} {lsort -command helper {a b}}}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let offset = u32::try_from(source.find("lsort").unwrap()).unwrap();
            assert!(
                source_logical_callback_candidates_at(source, &analysis, offset)
                    .is_none_or(|candidates| candidates.is_empty()),
                "{source}"
            );
        }
        let source = "lsort -command {helper baked} {a b}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let callbacks = source_logical_callback_candidates_at(source, &analysis, 0).unwrap();
        assert_eq!(callbacks.len(), 1);
        assert_eq!(
            callbacks[0].coordinates(),
            tcl_syntax::naming::authored_source_command_candidates(
                &tcl_core_types::ByteNamespacePath::root(),
                &[],
                b"helper"
            )
            .unwrap()
        );
    }
}
