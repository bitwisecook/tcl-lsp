// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional C-policy namespace coordinates retain original source inputs.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    AdvicePublicationPurpose, SourceAdviceNameInput,
};
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};
use tcl_registry::{NamespaceTransition, NamespaceTransitionTarget, StateTransition, Traits};
use tcl_syntax::naming::{
    NamePolicyProtocol, NativeNameContext, NativeNameProtocol, NativeNamePurpose,
    NativeNameQualification, NativeNamespacePatternSource,
};
use tcl_syntax::native_glob::{NativeGlobProtocol, NativeNameGlobPurpose};

impl AdviceGraph {
    fn c_source_policy(&self) -> Option<NamePolicyProtocol> {
        let policy = self.policy.native()?;
        if !matches!(policy.recipe(), NativeNameProtocol::C(_))
            || self.dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || self.native_baseline
            || self.c_source_namespace.is_none()
        {
            return None;
        }
        Some(policy)
    }

    pub(super) fn c_source_lookup_namespace(&self) -> Option<&ByteNamespacePath> {
        self.c_source_policy()?;
        self.procedure_namespace
            .as_ref()
            .or(self.c_source_namespace.as_ref())
    }

    fn c_source_input<'a>(&self, input: &'a SourceAdviceNameInput) -> Option<&'a [u8]> {
        let input = input.native_input()?;
        (input.policy() == self.c_source_policy()? && input.original_word_key().is_some())
            .then(|| input.bytes())
    }

    pub(super) fn c_source_namespace_path(
        &self,
        input: &SourceAdviceNameInput,
    ) -> Option<ByteNamespacePath> {
        self.c_source_policy()?
            .recipe()
            .namespace_address_path(
                NativeNameContext::new(self.c_source_lookup_namespace()?),
                self.c_source_input(input)?,
            )
            .ok()
    }

    pub(super) fn c_source_head_key(&self, bytes: &[u8]) -> Option<ByteCommandSlot> {
        let candidates = self.c_source_head_candidates(bytes)?;
        let occupied = |slot: &ByteCommandSlot| {
            self.cells
                .get(slot)
                .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        };
        let (local, fallback) = candidates.split_first()?;
        if fallback.is_empty() || occupied(local) {
            return Some(local.clone());
        }
        if self
            .unknown_paths
            .contains(self.c_source_lookup_namespace()?)
        {
            return None;
        }
        fallback
            .iter()
            .find(|slot| occupied(slot))
            .or(fallback.last())
            .cloned()
    }

    /// The coordinates [`Self::c_source_head_key`] tries, in its order: the
    /// local slot alone for an absolute name, otherwise the local slot, each
    /// `namespace path` entry and the root.
    pub(super) fn c_source_head_candidates(&self, bytes: &[u8]) -> Option<Vec<ByteCommandSlot>> {
        let policy = self.c_source_policy()?;
        let current = self.c_source_lookup_namespace()?;
        let recipe = policy.recipe();
        let context = NativeNameContext::new(current);
        let absolute = recipe
            .command_lookup_input(context, bytes)
            .ok()?
            .qualification()
            == NativeNameQualification::Absolute;
        let mut candidates = vec![recipe.command_lookup_slot(context, bytes).ok()?];
        if absolute {
            return Some(candidates);
        }
        if let Some(path) = self.namespace_paths.get(current) {
            for namespace in path {
                candidates.push(
                    recipe
                        .command_lookup_slot(NativeNameContext::new(namespace), bytes)
                        .ok()?,
                );
            }
        }
        candidates.push(
            recipe
                .command_lookup_slot(NativeNameContext::root(), bytes)
                .ok()?,
        );
        Some(candidates)
    }

    pub(super) fn c_source_publication_key(
        &self,
        bytes: &[u8],
        purpose: AdvicePublicationPurpose,
    ) -> Option<ByteCommandSlot> {
        let recipe = self.c_source_policy()?.recipe();
        let context = NativeNameContext::new(self.c_source_lookup_namespace()?);
        match purpose {
            AdvicePublicationPurpose::Define => recipe.command_publication_slot(context, bytes),
            AdvicePublicationPurpose::Move => recipe.rename_destination_slot(context, bytes),
            AdvicePublicationPurpose::Alias => recipe.alias_publication_slot(context, bytes),
        }
        .ok()
    }

    pub(super) fn source_transition_namespace(
        &self,
        receipt: &super::OriginalSourceCommandTransition,
        target: &NamespaceTransitionTarget,
    ) -> Option<ByteNamespacePath> {
        match &self.policy {
            AdviceNamingPolicy::Logical(_) => self.logical_transition_namespace(receipt, target),
            AdviceNamingPolicy::Native(_) => {
                self.c_source_policy()?;
                match target {
                    NamespaceTransitionTarget::Current => self.c_source_namespace.clone(),
                    NamespaceTransitionTarget::Named(subject) => {
                        self.c_source_namespace_path(receipt.input(subject)?)
                    }
                }
            }
        }
    }

    pub(super) fn source_namespace_pattern(
        &self,
        namespace: &ByteNamespacePath,
        input: &SourceAdviceNameInput,
        purpose: NativeNamePurpose,
    ) -> Option<(Option<ByteNamespacePath>, NameBytes)> {
        match &self.policy {
            AdviceNamingPolicy::Logical(_) => {
                let input = input.logical_input()?;
                let slot =
                    tcl_syntax::naming::authored_source_command_slot(namespace, input.bytes())?;
                let qualified = input.bytes().starts_with(b"::")
                    || tcl_syntax::naming::qualifier_segments(input.bytes()).len() > 1;
                Some((qualified.then_some(slot.namespace), slot.simple))
            }
            AdviceNamingPolicy::Native(_) => {
                let parts = self
                    .c_source_policy()?
                    .recipe()
                    .namespace_pattern_parts(
                        NativeNameContext::new(namespace),
                        self.c_source_input(input)?,
                        purpose,
                    )
                    .ok()?;
                let source = match parts.source {
                    Some(NativeNamespacePatternSource::C(path)) => Some(path),
                    Some(NativeNamespacePatternSource::Jim(_)) => return None,
                    None => None,
                };
                Some((source, parts.tail))
            }
        }
    }

    pub(super) fn source_namespace_match(
        &self,
        purpose: NativeNameGlobPurpose,
        pattern: &[u8],
        name: &[u8],
    ) -> Option<bool> {
        match self.policy {
            AdviceNamingPolicy::Logical(_) => {
                Some(tcl_syntax::glob::string_match_bytes(pattern, name))
            }
            AdviceNamingPolicy::Native(_) => {
                NativeGlobProtocol::from_name_policy(self.c_source_policy()?)
                    .match_name_pattern(purpose, pattern, name)
                    .ok()
            }
        }
    }

    pub(super) fn source_namespace_path_list(
        &self,
        current: &ByteNamespacePath,
        input: &SourceAdviceNameInput,
    ) -> Option<Vec<ByteNamespacePath>> {
        let paths = match self.policy {
            AdviceNamingPolicy::Logical(_) => {
                let input = input.logical_input()?;
                let text = std::str::from_utf8(input.bytes()).ok()?;
                tcl_syntax::word_rules::WordValueRules::from_grammar(&self.dialect.lexer_grammar)
                    .split_list(text)
                    .ok()?
                    .iter()
                    .map(|element| {
                        tcl_syntax::naming::authored_source_namespace_path(
                            current,
                            element.as_bytes(),
                        )
                    })
                    .collect::<Option<Vec<_>>>()?
            }
            AdviceNamingPolicy::Native(_) => {
                let policy = self.c_source_policy()?;
                let elements = tcl_syntax::list::split_native_list_bytes(
                    self.c_source_input(input)?,
                    policy.string_protocol(),
                )
                .ok()?;
                elements
                    .iter()
                    .map(|element| {
                        policy
                            .recipe()
                            .namespace_address_path(NativeNameContext::new(current), element)
                            .ok()
                    })
                    .collect::<Option<Vec<_>>>()?
            }
        };
        match self.policy {
            AdviceNamingPolicy::Logical(_) => paths
                .iter()
                .all(|path| self.namespaces.contains(path))
                .then_some(paths),
            // These are independently projected source coordinates. Missing
            // provider rows remain explicit on each consuming source receipt.
            AdviceNamingPolicy::Native(_) => Some(paths),
        }
    }
}

impl AdviceInvocationContext<'_> {
    pub(super) fn c_source_lifecycle_selection(
        &self,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<tcl_registry::body_execution::CapturedLifecycleSelection> {
        graph.c_source_policy()?;
        if schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || schema.semantics.body_kind != tcl_registry::BodyKind::Structural
            || schema
                .semantics
                .body_interpreter
                .resolve_with(|ordinal| schema.words.arguments().literal_at(ordinal))
                != tcl_registry::world_effect::InterpreterScope::Current
        {
            return None;
        }
        let tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(lifecycle) =
            schema.semantics.body_execution?
        else {
            return None;
        };
        (self.dialect.family() == Some(lifecycle.provider.required_core_family)).then_some(())?;
        // A source caller coordinate cannot cross a separately compiled procedure
        // frame merely because the descriptor retains the lifecycle grammar.
        (lifecycle.phase_compilation
            == tcl_registry::native_compilation::NativeBodyCompilation::Uplevel)
            .then_some(())?;
        match lifecycle.select(schema.words.arguments()) {
            tcl_registry::body_execution::BodyExecutionSelection::CapturedLifecycle(phases) => {
                Some(phases)
            }
            tcl_registry::body_execution::BodyExecutionSelection::InvalidArguments
            | tcl_registry::body_execution::BodyExecutionSelection::UnknownArguments => None,
        }
    }

    pub(super) fn c_source_namespace_scope(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<ByteNamespacePath> {
        graph.c_source_policy()?;
        if schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || !schema.semantics.traits.contains(Traits::DECLARES_NAMESPACE)
            || schema.semantics.body_kind != tcl_registry::BodyKind::Structural
            || schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
            }) != tcl_registry::world_effect::InterpreterScope::Current
        {
            return None;
        }
        let transitions = schema.state_transitions();
        let mut names = transitions
            .facts()
            .iter()
            .filter_map(|fact| match &fact.transition {
                StateTransition::Namespace(NamespaceTransition::Ensure {
                    namespace: NamespaceTransitionTarget::Named(name),
                }) => Some(name),
                _ => None,
            });
        let name = names.next()?;
        if names.next().is_some() {
            return None;
        }
        let receipt = super::capture_transition(
            self.origin,
            invocation.native,
            schema.canonical_command,
            invocation.arguments,
            transitions.clone(),
            invocation.lineage,
        )?;
        graph.c_source_namespace_path(receipt.input(name)?)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SourceCommandTransitionObligation;
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words_at,
    };

    fn assert_closed_native_entry_cannot_import_absent_package() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let source =
            "namespace import ::tcltest::*; test naming-1 description -body {set x 1} -result 1";
        let analysis = Analyser::new()
            .with_source_analysis_entry(std::sync::Arc::new(
                crate::command_binding::SourceAnalysisEntry {
                    native_entry: Some(std::sync::Arc::new(entry)),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            ))
            .analyse(source, profile.name);
        let offset = source.rfind("test naming-1").unwrap().try_into().unwrap();
        assert!(
            source_registry_words_at(source, &analysis, offset).is_none(),
            "actual closed NativeBaseline cannot obtain an absent imported package command from source coordinates"
        );
    }

    fn assert_namespace_path_mutations_keep_source_selection(dialect: &str) {
        let prefix = "namespace eval ::provider {}; rename if ::provider::choose; namespace path ::provider; ";
        for (mutation, expected) in [
            ("", true),
            ("opaque; ", false),
            ("namespace path {}; ", false),
            ("namespace path $unknown; ", false),
            ("namespace path \"{\"; ", false),
            ("proc choose args {}; ", false),
        ] {
            let source = format!("{prefix}{mutation}choose 1 {{puts ok}}");
            let analysis = Analyser::new().analyse(&source, dialect);
            let offset = source.rfind("choose 1").unwrap().try_into().unwrap();
            let words = source_registry_words_at(&source, &analysis, offset);
            assert_eq!(words.is_some(), expected, "{dialect}: {source}");
            if let Some(words) = words {
                assert_eq!(words.command(), "if");
            }
        }
    }

    fn assert_import_withdrawals_refuse_source_schema(dialect: &str) {
        for prefix in [
            "proc test args {}; namespace import ::tcltest::*; ",
            "namespace import ::tcltest::*; namespace forget test; ",
            "namespace import ::tcltest::*; rename ::tcltest::test {}; ",
        ] {
            let source = format!("{prefix}test naming-1 description -body {{set x 1}} -result 1");
            let analysis = Analyser::new().analyse(&source, dialect);
            let offset = source.rfind("test naming-1").unwrap().try_into().unwrap();
            assert!(
                source_registry_words_at(&source, &analysis, offset).is_none(),
                "{dialect}: {source}"
            );
        }
    }

    fn assert_original_namespace_is_client(
        advice: &crate::command_binding::source_transition_advice::OriginalSourceCommandTransitionAdvice,
    ) {
        assert_eq!(
            advice
                .conditional_c_source_namespace()
                .unwrap()
                .as_segments(),
            &[tcl_core_types::NameBytes::from(&b"client"[..])]
        );
    }

    fn assert_original_child_dict_schema(
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) {
        assert_eq!(words.command(), "dict");
        assert_eq!(
            words.with_source_schema(input.borrowed_context_registry(), |schema| schema
                .subcommand
                .resolved()
                .map(|subcommand| subcommand.canonical_name)),
            Some(Some("get"))
        );
    }

    fn assert_child_lifecycle_namespace_is_conditional(
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        analysis: &crate::analyser::AnalysisResult,
    ) {
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("child must retain its conditional lifecycle caller coordinates");
        };
        assert!(advice.obligations().contains(
            &SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability
        ));
        assert_eq!(
            advice
                .conditional_c_source_namespace()
                .unwrap()
                .as_segments(),
            &[tcl_core_types::NameBytes::from(&b"client"[..])]
        );
        assert!(advice.obligations().contains(
            &SourceCommandTransitionObligation::UnretainedCSourcePathProviderApplicability
        ));
        assert!(analysis.original_completed_command_world().is_none());
    }

    #[test]
    fn c_source_namespace_consumer_keeps_import_path_body_and_withdrawal() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [
                "namespace import tcltest::*\ntest naming-1 description -body {set x 1} -result 1\n",
                "package require tcltest\nopaque\nnamespace eval ::client {namespace import ::tcltest::*; test naming-1 description -body {set x 1} -result 1\n}\n",
            ] {
                let analysis = Analyser::new().analyse(source, dialect);
                let offset = source.rfind("test naming-1").unwrap().try_into().unwrap();
                let words = source_registry_words_at(source, &analysis, offset)
                    .unwrap_or_else(|| panic!("{dialect}: {source}"));
                assert_eq!(words.command(), "tcltest::test");
                let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                    panic!(
                        "C source import must retain its own transition applicability: {dialect}"
                    );
                };
                assert!(advice.original_head().native_input().is_some());
                assert!(advice.original_head().logical_input().is_none());
                assert!(advice.obligations().contains(
                    &SourceCommandTransitionObligation::ConditionalCNamespaceApplicability
                ));
                assert!(
                    advice
                        .obligations()
                        .contains(&SourceCommandTransitionObligation::UnavailableActualLookup)
                );
                assert!(
                    advice.obligations().contains(
                        &SourceCommandTransitionObligation::WrittenTransitionApplicability
                    )
                );
                assert!(!advice.obligations().contains(
                    &SourceCommandTransitionObligation::NativeBaselineSourceApplicability
                ));
                if source.contains("namespace eval") {
                    assert!(advice.c_source_body.is_some());
                    assert!(advice.logical_body.is_none());
                    assert!(advice.obligations().contains(
                        &SourceCommandTransitionObligation::ConditionalCBodyApplicability
                    ));
                    assert!(
                        advice
                            .obligations()
                            .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
                    );
                }
                let context = analysis.resolved_input.as_ref().unwrap().context_registry();
                assert!(words.with_source_schema(&context, |_| ()).is_some());
                assert_eq!(words.source_script_bodies(&context).len(), 1);
                assert!(
                    source_registry_words_at(&format!("{source}# changed"), &analysis, offset)
                        .is_none()
                );
            }
            assert_import_withdrawals_refuse_source_schema(dialect);
            if dialect == "tcl8.4" {
                continue;
            }
            let source = concat!(
                "package require tcltest\npackage require tclopt\n",
                "namespace eval ::client {\nnamespace path ::tclopt\n",
                "namespace import ::tcltest::*\n",
                "test naming-1 description -body {set x 1} -result 1\n}\n",
            );
            let analysis = Analyser::new().analyse(source, dialect);
            let offset = source.rfind("test naming-1").unwrap().try_into().unwrap();
            let words = source_registry_words_at(source, &analysis, offset)
                .expect("unretained source provider");
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("unretained provider must remain a conditional source recipe");
            };
            assert_original_namespace_is_client(advice);
            let path = advice.conditional_c_source_path().unwrap();
            assert_eq!(path.len(), 1);
            assert_eq!(
                path[0].as_segments(),
                &[tcl_core_types::NameBytes::from(&b"tclopt"[..])]
            );
            assert!(advice.obligations().contains(
                &SourceCommandTransitionObligation::UnretainedCSourcePathProviderApplicability
            ));
            assert!(analysis.original_completed_command_world().is_none());
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let policy = advice.original_head().native_input().unwrap().policy();
            let graph = super::AdviceGraph::new(
                &context,
                &super::AdviceNamingPolicy::Native(policy),
                advice.dialect,
            )
            .unwrap();
            assert!(
                !graph.namespaces.contains(&path[0]),
                "source coordinates must not invent provider rows"
            );
            assert_namespace_path_mutations_keep_source_selection(dialect);
        }
        assert_closed_native_entry_cannot_import_absent_package();
    }

    #[test]
    fn c_source_namespace_consumer_retains_lifecycle_child_schema_and_refuses_foreign_scopes() {
        use crate::registry_invocation::source_structure::source_registry_words;
        let source = concat!(
            "package require tcltest\npackage require tclopt\n",
            "namespace eval ::client {\nnamespace path ::tclopt\n",
            "namespace import ::tcltest::*\n",
            "test naming-1 description -body {\nexpr {[dict get $result r] ne $r}\n} -result 1\n}\n",
        );
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let input = analysis.resolved_input.as_ref().unwrap();
        let expr_offset = source.find("expr {").unwrap().try_into().unwrap();
        let expr = source_registry_words_at(source, &analysis, expr_offset)
            .expect("caller-scope source expression");
        let bodies = expr.source_expression_script_bodies(input).unwrap();
        assert_eq!(bodies.len(), 1);
        let span = bodies[0].content_span();
        let child = crate::segmenter::segment_commands_with_offset_and_config(
            &source[span.as_range()],
            span.start(),
            input.lexer_config(),
        )
        .remove(0);
        let words = source_registry_words(source, &analysis, &child)
            .expect("bounded original child source schema");
        assert_original_child_dict_schema(&words, input);
        assert_child_lifecycle_namespace_is_conditional(&words, &analysis);

        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let base = tcl_registry::model::ingress::context_for_profile(profile);
        for condition in [
            "caller",
            "foreign-interpreter",
            "non-caller",
            "foreign-phase-frame",
            "foreign-provider",
        ] {
            let mut registry = tcl_registry::CommandRegistry::build_default();
            let mut descriptor = registry.get("tcltest::test").unwrap().clone();
            descriptor.name = "source-lifecycle";
            descriptor.required_package = None;
            match condition {
                "foreign-interpreter" => {
                    descriptor.body_interpreter = tcl_registry::BodyInterpreter::Argument(0);
                }
                "non-caller" => {
                    descriptor.body_execution =
                        Some(tcl_registry::body_execution::BodyExecutionSpec::DeferredGlobalScript);
                }
                "foreign-provider" | "foreign-phase-frame" => {
                    let Some(tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(
                        original,
                    )) = descriptor.body_execution
                    else {
                        panic!("fixture lifecycle");
                    };
                    let mut lifecycle = *original;
                    if condition == "foreign-provider" {
                        lifecycle.provider.required_core_family = tcl_dialect::model::Family::Jim;
                    } else {
                        lifecycle.phase_compilation =
                            tcl_registry::native_compilation::NativeBodyCompilation::ProcedureObject;
                    }
                    descriptor.body_execution = Some(
                        tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(
                            Box::leak(Box::new(lifecycle)),
                        ),
                    );
                }
                _ => {}
            }
            registry.insert(descriptor);
            let context =
                std::sync::Arc::new(base.with_command_store(std::sync::Arc::new(registry)));
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context,
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            );
            let source = "source-lifecycle case description -body {\nexpr {[dict get $result r] ne $r}\n} -result 1\n";
            let analysis = Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let tape = analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_source_transition_advice_tape(&context)
                .unwrap();
            let offset = source.find("dict get").unwrap().try_into().unwrap();
            let captured = matches!(tape.lookup(offset), super::super::OriginalSourceTransitionAdviceLookup::Advice(advice)
                if advice.obligations().contains(&SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability));
            assert_eq!(captured, condition == "caller", "{condition}");
        }
    }
}
