// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Logical expression identifiers retain source lookup intentions, never dispatch.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocationContext, AdviceNamingPolicy,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
};
use crate::analyser::types::LogicalSourceProcedureDeclaration;
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath};
use tcl_lexer::{NativeWord, SourceImage, Span};

/// One genuine checked Logical expression identifier and its conditional source
/// coordinates. The original parent, body/frame recipe, complete argv, expression
/// and grammar remain held. No function table, runtime winner, entered frame,
/// command-head word, Native lookup, completion or edit permission is supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalMathFunctionOccurrence {
    parent: Arc<OriginalSourceCommandTransitionAdvice>,
    original: NativeWord,
    document: Option<super::OriginalLogicalCommandCandidates>,
    effective_argument: usize,
    name: tcl_registry::mathfunc::OriginalSourceFunctionName,
    span: Span,
    namespace: ByteNamespacePath,
    coordinates: Option<Vec<ByteCommandSlot>>,
    excluded_headers: Vec<ByteCommandSlot>,
    earlier_unknown_mutations: bool,
    operand_obligations: bool,
}
impl OriginalLogicalMathFunctionOccurrence {
    /// Whole original expression word, without a synthetic script head.
    #[must_use]
    pub const fn original_word(&self) -> &NativeWord {
        &self.original
    }
    /// Exact complete parent schema, argv and applicability obligations.
    #[must_use]
    pub fn parent(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.parent
    }
    /// Actual identifier extent in the complete original source image.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Checked source identifier bytes, independent of reporting names.
    #[must_use]
    pub fn function(&self) -> &str {
        self.name.function()
    }
    /// Original function traversal ordinal, distinct from effective argv indices.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.name.ordinal()
    }
    /// Checked argument topology, not runtime arity acceptance or formal binding.
    #[must_use]
    pub const fn argument_count(&self) -> usize {
        self.name.argument_count()
    }
    /// Original Logical source namespace recipe, without a runtime namespace.
    #[must_use]
    pub const fn namespace(&self) -> &ByteNamespacePath {
        &self.namespace
    }
    /// Ordered source naming intentions. Missing path representation remains
    /// unavailable, rather than an empty candidate set or a global fallback.
    #[must_use]
    pub fn coordinates(&self) -> Option<&[ByteCommandSlot]> {
        self.coordinates.as_deref()
    }
    /// Earlier unknown effects remain conditional applicability obligations.
    #[must_use]
    pub const fn has_earlier_unknown_mutations(&self) -> bool {
        self.earlier_unknown_mutations
    }
    /// The original expression contains opaque/string or substitution operands.
    /// False still closes no function lookup, conversion or object observer.
    #[must_use]
    pub const fn has_opaque_or_substitution_operands(&self) -> bool {
        self.operand_obligations
    }
    /// Exact whole source, configuration, input and authentic parent correspondence.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.parent.logical_source_input() == Some(input)
            && input.has_logical_source_name_context()
            && self.parent.matches_source(image, input.lexer_config())
            && self
                .parent
                .matches_context(input.borrowed_context_registry())
            && match (self.parent.logical_source_scope(), &self.document) {
                (Some(scope), None) => scope.namespace_recipe() == &self.namespace,
                (None, Some(document)) => {
                    document.is_original_document_scope()
                        && document.matches_source(image, input)
                        && document.original_words() == self.parent.original_words()
                        && self.namespace == ByteNamespacePath::root()
                }
                _ => false,
            }
            && self.original.image() == image
            && self.original.config() == input.lexer_config()
            && self.name.expression().lexer_config() == input.lexer_config()
            && self
                .parent
                .arguments()
                .get(self.effective_argument)
                .is_some_and(|argument| {
                    argument.original == self.original
                        && matches!(
                            argument.origin,
                            crate::registry_invocation::InvocationWordOrigin::Written(_)
                        )
                        && argument.value.as_deref()
                            == Some(self.name.expression().source().as_bytes())
                })
            && self.original.content_span().is_ok_and(|span| {
                image.bytes().get(span.as_range())
                    == Some(self.name.expression().source().as_bytes())
                    && span.start().checked_add(self.name.offset()) == Some(self.span.start())
                    && self.span.end() <= span.end()
            })
    }
    fn matches_analysis(&self, analysis: &AnalysisResult) -> bool {
        let Some(input) = analysis.resolved_input.as_ref() else {
            return false;
        };
        analysis.allows_lexical_declaration_advice()
            && analysis.matches_original_source_image(self.original.image(), input.lexer_config())
            && self.matches_source(self.original.image(), input)
            && analysis
                .retained_command_realm()
                .and_then(|realm| realm.original_logical_math_function_at(input, self.span.start()))
                .as_ref()
                == Some(self)
    }
    /// Genuine current header cards at the ordered source coordinates. Repeated
    /// declarations remain distinct allocations; this never selects a winner or
    /// proves absence. Missing source/path ownership returns no inventory.
    #[must_use]
    pub fn source_declarations<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<Vec<&'a LogicalSourceProcedureDeclaration>> {
        self.matches_analysis(analysis).then_some(())?;
        let input = analysis.resolved_input.as_ref()?;
        let declarations = analysis.checked_logical_source_procedure_declarations(
            self.original.image(),
            input.lexer_config(),
        )?;
        let mut candidates = Vec::new();
        for coordinate in self.coordinates.as_ref()? {
            if self.excluded_headers.contains(coordinate) {
                continue;
            }
            for declaration in &declarations {
                if declaration.source_candidate_coordinate()? == *coordinate {
                    candidates.push(*declaration);
                }
            }
        }
        Some(candidates)
    }
    /// Documentation from this exact source Registry and expression grammar.
    /// It is one conditional metadata alternative, never an installed function,
    /// mutable command-table selection or Native fixed-table registration.
    #[must_use]
    pub fn builtin_metadata<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a tcl_registry::CommandSpec> {
        self.matches_analysis(analysis).then_some(())?;
        let input = analysis.resolved_input.as_ref()?;
        analysis
            .resolved_registry()?
            .math_function_spec(self.function(), input.unit_profile())
    }
}

impl AdviceInvocationContext<'_> {
    pub(super) fn retain_logical_math_functions(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        parent: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        // naming.source.logical-expression-function-candidates
        // docs/design/analysis/name-resolution-proofs/original-expression-source-selection.md
        let AdviceNamingPolicy::Logical(input) = self.policy else {
            return;
        };
        let Some(namespace) = graph.logical_namespace.as_ref() else {
            return;
        };
        if parent.logical_source_input() != Some(input)
            || !parent.matches_source(self.origin.source_image(), self.config)
            || !parent.matches_context(self.context)
            || schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            || schema.semantics.frame_effect.is_some()
            || schema.semantics.traits.intersects(
                tcl_registry::traits::FRAME_REACH_TRAITS | tcl_registry::Traits::DEFERS_BODY,
            )
            || parent
                .original_words()
                .iter()
                .any(|word| word.group().expand)
            || schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(parent.arguments().get(ordinal)?.value.as_deref()?).ok()
            }) != tcl_registry::world_effect::InterpreterScope::Current
        {
            return;
        }
        let document = if let Some(scope) = parent.logical_source_scope() {
            if !scope.matches_input(input)
                || !scope.owns_words(parent.original_words())
                || namespace != scope.namespace_recipe()
            {
                return;
            }
            None
        } else {
            // Only the whole-source root traversal issued this Document receipt.
            // A missing member/body/frame cannot acquire a new root context.
            if parent.logical_source_body().is_some()
                || parent.catalogue_body.is_some()
                || parent.member_catalogue_body.is_some()
            {
                return;
            }
            let Some(document) = tape
                .logical_navigation
                .get(&parent.site().offset)
                .and_then(Option::as_ref)
                .filter(|document| {
                    document.is_original_document_scope()
                        && document.matches_source(self.origin.source_image(), input)
                        && document.original_words() == parent.original_words()
                        && namespace == &ByteNamespacePath::root()
                })
            else {
                return;
            };
            Some(document.clone())
        };
        let Some(roles) = parent.roles() else {
            return;
        };
        let mut context = self
            .dialect
            .expression_parse_context(self.context.commands().profile());
        context.lexer_grammar = self.config.grammar_over(context.lexer_grammar);
        for &(effective_argument, role) in roles {
            if role != tcl_registry::ArgRole::Expr {
                continue;
            }
            let Some(argument) = parent.arguments().get(effective_argument) else {
                continue;
            };
            if !matches!(
                argument.origin,
                crate::registry_invocation::InvocationWordOrigin::Written(_)
            ) {
                continue;
            }
            let Ok(extent) = argument.original.content_span() else {
                continue;
            };
            let Some(source) = self
                .origin
                .source_image()
                .try_text()
                .ok()
                .and_then(|source| source.get(extent.as_range()))
            else {
                continue;
            };
            if argument.value.as_deref() != Some(source.as_bytes()) {
                continue;
            }
            let Some(expression) =
                tcl_syntax::expr::OriginalSourceExpression::capture(source, &context, self.config)
                    .map(Arc::new)
            else {
                continue;
            };
            for ordinal in 0..expression.function_calls().len() {
                let Some(name) = tcl_registry::mathfunc::OriginalSourceFunctionName::capture(
                    Arc::clone(&expression),
                    ordinal,
                ) else {
                    continue;
                };
                let Some(start) = extent.start().checked_add(name.offset()) else {
                    continue;
                };
                let Some(end) = u32::try_from(name.function().len())
                    .ok()
                    .and_then(|width| start.checked_add(width))
                else {
                    continue;
                };
                let coordinates = super::logical_navigation::logical_coordinates(
                    graph,
                    namespace,
                    name.relative_source_name(),
                );
                let excluded_headers = coordinates
                    .iter()
                    .flatten()
                    .filter(|coordinate| {
                        graph
                            .cells
                            .get(*coordinate)
                            .is_some_and(|cell| !matches!(cell, AdviceCell::Shadowed))
                    })
                    .cloned()
                    .collect();
                let occurrence = OriginalLogicalMathFunctionOccurrence {
                    parent: Arc::new(parent.clone()),
                    original: argument.original.clone(),
                    document: document.clone(),
                    effective_argument,
                    name,
                    span: Span::new(start, end),
                    namespace: namespace.clone(),
                    coordinates,
                    excluded_headers,
                    earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
                    operand_obligations: expression.has_opaque_or_substitution_operands(),
                };
                let occurrence = occurrence
                    .matches_source(self.origin.source_image(), input)
                    .then_some(occurrence);
                super::source_class::merge_receipt(&mut tape.logical_math, start, occurrence);
            }
        }
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn logical_math_at(
        &self,
        input: &ResolvedAnalysisInput,
        image: &SourceImage,
        offset: u32,
    ) -> Option<OriginalLogicalMathFunctionOccurrence> {
        let mut candidates = self.logical_math.values().flatten().filter(|occurrence| {
            occurrence.span.start() <= offset && offset < occurrence.span.end()
        });
        let candidate = candidates.next()?;
        (candidates.next().is_none() && candidate.matches_source(image, input))
            .then(|| candidate.clone())
    }
}
impl AnalysisResult {
    /// Checked original Logical math identifier at this exact whole-source offset.
    /// This is conditional readonly source navigation. Native function tables and
    /// reached dispatch keep their separate terminal owner and selection purpose.
    #[must_use]
    pub fn logical_math_function_at(
        &self,
        source: &str,
        offset: u32,
    ) -> Option<OriginalLogicalMathFunctionOccurrence> {
        let input = self.resolved_input.as_ref()?;
        let image = SourceImage::document(source);
        (self.allows_lexical_declaration_advice()
            && self.body_lexer_config == Some(input.lexer_config())
            && self.matches_original_source_image(&image, input.lexer_config()))
        .then_some(())?;
        crate::realm::CommandBindingRealm::original_logical_math_function_at(
            self.retained_command_realm()?,
            input,
            offset,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_math_candidates_keep_original_tree_scope_and_distinct_headers() {
        let source = "namespace eval ::tcl::mathfunc {proc local {root} {return $root}}; namespace eval N {namespace eval tcl::mathfunc {proc local {first} {return $first}; proc local {second} {return $second}}; proc caller {x} {expr {local(local($x))}}}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let outer = u32::try_from(source.rfind("local(local").unwrap()).unwrap();
        let occurrence = analysis
            .logical_math_function_at(source, outer)
            .expect("actual original expression function");
        assert_eq!(occurrence.ordinal(), 0);
        assert_eq!(occurrence.argument_count(), 1);
        assert_eq!(occurrence.function(), "local");
        assert_eq!(
            occurrence.namespace(),
            &ByteNamespacePath::from_segments([b"N".as_slice()])
        );
        assert!(occurrence.has_opaque_or_substitution_operands());
        assert!(!occurrence.parent().obligations().is_empty());
        let declarations = occurrence
            .source_declarations(&analysis)
            .expect("genuine retained header catalogue");
        assert_eq!(
            declarations
                .iter()
                .map(|row| row.metadata().params[0].name.as_str())
                .collect::<Vec<_>>(),
            ["first", "second", "root"]
        );
        assert_ne!(
            declarations[0].metadata().name_span,
            declarations[1].metadata().name_span
        );
        let nested = analysis
            .logical_math_function_at(source, outer + 6)
            .unwrap();
        assert_eq!(nested.ordinal(), 1);
        assert_eq!(nested.function(), "local");
        assert_eq!(nested.span().start(), outer + 6);
        // Report strings/maps cannot select or replace an original allocation.
        analysis.all_procs.clear();
        assert_eq!(occurrence.source_declarations(&analysis).unwrap().len(), 3);
    }

    #[test]
    fn logical_math_candidates_keep_alias_prefix_unknown_effects_and_currentness() {
        let source = "interp alias {} compute {} expr; unknown_prefix; compute {max(abs($x), 2)}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(source.find("max(").unwrap()).unwrap();
        let occurrence = analysis
            .logical_math_function_at(source, offset)
            .expect("selected original alias expression");
        assert_eq!(occurrence.parent().original_words().len(), 2);
        assert!(!occurrence.parent().lineage().is_empty());
        assert!(occurrence.has_earlier_unknown_mutations());
        assert!(occurrence.has_opaque_or_substitution_operands());
        assert!(occurrence.builtin_metadata(&analysis).is_some());
        assert!(
            analysis
                .logical_math_function_at(&format!("{source} "), offset)
                .is_none()
        );
        let input = analysis.resolved_input.as_ref().unwrap();
        let foreign =
            SourceImage::from_bytes(source.as_bytes(), tcl_lexer::SourceChannel::NativeValue);
        assert!(!occurrence.matches_source(&foreign, input));
        let mut changed = analysis.clone();
        let mut config = changed.body_lexer_config.unwrap();
        config.strict_quoting = !config.strict_quoting;
        changed.body_lexer_config = Some(config);
        assert!(changed.logical_math_function_at(source, offset).is_none());
        assert!(occurrence.source_declarations(&changed).is_none());
    }

    #[test]
    fn logical_math_candidates_keep_selected_namespace_path_order() {
        let source = "namespace eval B {namespace eval tcl::mathfunc {proc through_path {value} {return $value}}}; namespace eval A {namespace path ::B; proc caller {} {expr {through_path(1)}}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(source.rfind("through_path(").unwrap()).unwrap();
        let occurrence = analysis
            .logical_math_function_at(source, offset)
            .expect("original path-scoped expression");
        let coordinates = occurrence.coordinates().unwrap();
        assert_eq!(coordinates.len(), 3);
        assert_eq!(
            coordinates[0].namespace,
            ByteNamespacePath::from_segments([
                b"A".as_slice(),
                b"tcl".as_slice(),
                b"mathfunc".as_slice()
            ])
        );
        assert_eq!(
            coordinates[1].namespace,
            ByteNamespacePath::from_segments([
                b"B".as_slice(),
                b"tcl".as_slice(),
                b"mathfunc".as_slice()
            ])
        );
        let declarations = occurrence.source_declarations(&analysis).unwrap();
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].metadata().params[0].name, "value");
    }

    #[test]
    fn logical_math_candidates_refuse_unowned_cooked_and_native_expressions() {
        for source in [
            "expr $code",
            "expr {max(1,}",
            "set code {max(1,2)}; expr $code",
            "expr \"max(1,\\\n2)\"",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            for offset in 0..u32::try_from(source.len()).unwrap() {
                assert!(
                    analysis.logical_math_function_at(source, offset).is_none(),
                    "{source}"
                );
            }
        }
        let source = "expr {abs(1)}";
        for profile in ["tcl8.4", "tcl8.6", "jim"] {
            let analysis = Analyser::new().analyse(source, profile);
            assert!(
                analysis.logical_math_function_at(source, 6).is_none(),
                "{profile}"
            );
        }
    }
}
