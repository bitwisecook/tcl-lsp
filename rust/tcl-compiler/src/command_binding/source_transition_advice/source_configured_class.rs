// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected configuration operands retain the pretransition source declaration.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    OriginalSourceClassDeclaration, OriginalSourceCommandTransitionAdvice,
    OriginalSourceTransitionAdviceTape, SourceAdviceNameInput, SourceCommandTransitionObligation,
};
use std::sync::Arc;
use tcl_lexer::{LexerConfig, SourceImage};
use tcl_registry::{
    ObjectDispatchLayer, ObjectDispatchTransition, StateTransition, model::ContextRegistry,
};

/// Conditional class configuration source, without a current class object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceConfiguredClassReference {
    consumer: Arc<OriginalSourceCommandTransitionAdvice>,
    target: SourceAdviceNameInput,
    class: Arc<OriginalSourceClassDeclaration>,
    obligations: Vec<SourceCommandTransitionObligation>,
}
impl OriginalSourceConfiguredClassReference {
    /// Whole actual selected configuration invocation, including captured operands.
    #[must_use]
    pub fn consumer(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.consumer
    }
    /// Genuine target operand and its independently selected naming purpose.
    #[must_use]
    pub const fn target_input(&self) -> &SourceAdviceNameInput {
        &self.target
    }
    /// Source class selected before the configuration transition.
    #[must_use]
    pub fn class_declaration(&self) -> &OriginalSourceClassDeclaration {
        &self.class
    }
    /// Unresolved execution and prior mutation premises remain explicit.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Immutable source, whole original words and full availability context agree.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.consumer.matches_source(image, config)
            && self.consumer.matches_context(context)
            && self.class.factory().matches_source(image, config)
            && self.class.factory().matches_context(context)
            && self
                .target
                .original_word()
                .is_some_and(|word| word.image() == image && word.config() == config)
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn configured_class(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceConfiguredClassReference> {
        self.configured_classes.get(&offset)?.as_ref()
    }
}
impl AdviceGraph {
    fn configured_source_class(
        &self,
        input: &SourceAdviceNameInput,
    ) -> Option<Arc<OriginalSourceClassDeclaration>> {
        match (&self.policy, input) {
            (AdviceNamingPolicy::Native(policy), SourceAdviceNameInput::Native(name))
                if *policy == name.policy() =>
            {
                ()
            }
            (AdviceNamingPolicy::Logical(_), SourceAdviceNameInput::Logical(_)) => (),
            _ => return None,
        }
        let key = self.lookup_head_key(input.bytes())?;
        // Configuration names a class object, so callable aliases to the class
        // cannot stand in for its own source declaration.
        match self.cells.get(&key)? {
            AdviceCell::SourceClass(class) => Some(Arc::clone(class)),
            _ => None,
        }
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_source_configured_class(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        let transitions = schema.state_transitions();
        let mut targets = transitions
            .facts()
            .iter()
            .filter_map(|fact| match &fact.transition {
                StateTransition::ObjectDispatch(ObjectDispatchTransition::Configure {
                    target,
                    layer: ObjectDispatchLayer::Class,
                }) => target.argument_index(),
                _ => None,
            });
        let Some(at) = targets.next() else {
            return;
        };
        if targets.any(|other| other != at) {
            return;
        }
        let Some(target) = invocation
            .arguments
            .get(at)
            .and_then(|word| word.input.clone())
        else {
            return;
        };
        let Some(class) = graph.configured_source_class(&target) else {
            return;
        };
        let Some(consumer) = self.schema_advice(invocation, graph, schema).map(Arc::new) else {
            return;
        };
        let mut obligations = consumer.obligations().to_vec();
        for obligation in class.factory().obligations() {
            if !obligations.contains(obligation) {
                obligations.push(obligation.clone());
            }
        }
        if let Some(input) = target.native_input() {
            tape.retain_class_reference(Self::source_class_reference(
                graph,
                Arc::clone(&consumer),
                input.clone(),
                Arc::clone(&class),
                None,
            ));
        }
        let receipt = OriginalSourceConfiguredClassReference {
            consumer,
            target,
            class,
            obligations,
        };
        super::source_class::merge_receipt(
            &mut tape.configured_classes,
            receipt.consumer.site().offset,
            Some(receipt),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyser::Analyser,
        registry_invocation::source_structure::{
            source_class_reference_at, source_configured_class_at,
        },
    };
    fn at(source: &str, marker: &str) -> u32 {
        u32::try_from(source.find(marker).unwrap()).unwrap()
    }
    #[test]
    fn original_configured_class_retains_moved_target_and_captured_operand() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        let source = "oo::class create C {method m {} {}}; rename C Held; interp alias {} configure {} oo::define Held; unavailable; configure {export m}";
        for profile in ["tcl", "tcl9.0"] {
            let analysis = Analyser::new().analyse(source, profile);
            let offset = at(source, "configure {export");
            let receipt = source_configured_class_at(source, &analysis, offset).unwrap();
            assert_eq!(receipt.target_input().bytes(), b"Held");
            assert_eq!(receipt.class_declaration().name_input().bytes(), b"C");
            assert_eq!(
                receipt.class_declaration().source_slot().simple.as_bytes(),
                b"Held"
            );
            assert!(
                receipt
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
            );
            if profile == "tcl" {
                assert!(
                    receipt
                        .class_declaration()
                        .logical_source_class(&analysis)
                        .is_some()
                );
                assert!(
                    receipt
                        .class_declaration()
                        .source_class(&analysis)
                        .is_none()
                );
            } else {
                let native = receipt.target_input().native_input().unwrap();
                assert!(source_class_reference_at(source, &analysis, offset, native).is_some());
                assert!(
                    receipt
                        .class_declaration()
                        .source_class(&analysis)
                        .is_some()
                );
                assert!(
                    receipt
                        .class_declaration()
                        .logical_source_class(&analysis)
                        .is_none()
                );
            }
        }
    }
    #[test]
    fn original_configured_class_refuses_known_barriers_and_object_targets() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        for operation in [
            "rename C {}",
            "rename C Held",
            "rename C {}; proc C {} {}",
            "interp alias {} Alias {} C",
        ] {
            let target = if operation.starts_with("interp") {
                "Alias"
            } else {
                "C"
            };
            let source = format!(
                "oo::class create C {{method m {{}} {{}}}}; {operation}; oo::define {target} {{export m}}"
            );
            for profile in ["tcl", "tcl9.0"] {
                let analysis = Analyser::new().analyse(&source, profile);
                assert!(
                    source_configured_class_at(&source, &analysis, at(&source, "oo::define"))
                        .is_none(),
                    "{profile}: {operation}"
                );
            }
        }
        let source = "oo::class create C {}; oo::objdefine C {export create}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        assert!(
            source_configured_class_at(source, &analysis, at(source, "oo::objdefine")).is_none()
        );
    }
    #[test]
    fn original_configured_class_keeps_full_input_and_unique_logical_metadata() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        let source = "oo::class create C {method m {} {}}; oo::define C {export m}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let offset = at(source, "oo::define");
        let receipt = source_configured_class_at(source, &analysis, offset).unwrap();
        assert!(
            receipt
                .class_declaration()
                .logical_source_class(&analysis)
                .is_some()
        );
        assert!(
            source_configured_class_at(&format!("# shifted\n{source}"), &analysis, offset)
                .is_none()
        );
        let duplicate = analysis.all_classes.values().next().unwrap().clone();
        analysis
            .all_classes
            .insert("reported_duplicate".into(), duplicate);
        assert!(
            receipt
                .class_declaration()
                .logical_source_class(&analysis)
                .is_none()
        );
        analysis.resolved_input = None;
        assert!(source_configured_class_at(source, &analysis, offset).is_none());
    }
}
