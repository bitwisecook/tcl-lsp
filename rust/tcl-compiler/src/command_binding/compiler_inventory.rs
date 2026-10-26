// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compiler visitation and runtime entry are independent source inventories.

use super::{Arc, CommandAllocationSite, SourceCommandBindings, SourceInvocationBinding};
use std::collections::BTreeMap;

/// Runtime entry knowledge for one exact original command allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SourceRuntimeReachability {
    /// An original declaration/template was interpreted conditionally. This
    /// retains operand layout and potential effects, never actual entry.
    Conditional,
    /// At least one retained traversal reached this command's dispatch.
    Reached,
    /// Complete bounded traversals covered the source but never reached dispatch.
    NotEntered,
    /// Source coverage, suspension or an opaque execution remains unresolved.
    #[default]
    Unknown,
}

/// Exact original words retained by an actual compiler observation. The
/// snapshot keeps its compiler world and descriptive frame; it never grants a
/// runtime lookup, variable cell, entered body or successful invocation.
pub(crate) struct OriginalCompilerArgumentSyntax<'a> {
    metadata: crate::registry_invocation::InvocationMetadataContext<'a>,
    original: &'a super::compiled_invocation::SourceOriginalCompilerWords,
    snapshot: &'a Arc<super::SourceLookupSnapshot>,
    namespace: &'a super::SourceNamespaceKey,
    dialect: Option<tcl_registry::InvocationDialect>,
    analysis_owner: &'a Arc<super::OriginalSourceAnalysisOwner>,
}

impl<'a> OriginalCompilerArgumentSyntax<'a> {
    pub(crate) fn metadata(&self) -> crate::registry_invocation::InvocationMetadataContext<'a> {
        self.metadata
    }

    pub(crate) fn site(&self) -> &'a CommandAllocationSite {
        &self.original.site
    }

    pub(crate) fn image(&self) -> &'a tcl_lexer::SourceImage {
        self.original.site.source.source_image()
    }

    pub(crate) fn words(&self) -> &'a [crate::ir::WordExpr] {
        &self.original.words
    }

    pub(crate) fn config(&self) -> tcl_lexer::LexerConfig {
        self.original.config
    }

    pub(crate) fn realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.snapshot.realm
    }

    pub(crate) fn source_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        let mut dialect = self.dialect?;
        dialect.lexer_grammar = self.config().grammar_over(dialect.lexer_grammar);
        dialect.word_values = tcl_syntax::word_rules::WordValueRules::from_config(&self.config());
        Some(dialect)
    }

    pub(crate) fn source_lookup_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<super::OriginalCompilationLookupAdvice> {
        super::original_site_operand_layout_advice(
            self.site(),
            tokens,
            self.snapshot,
            self.namespace,
            self.config(),
        )
    }

    pub(crate) fn same_analysis(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.analysis_owner, other.analysis_owner)
    }
}

impl SourceInvocationBinding {
    /// Syntax from the compiler's own full-input snapshot. This is a distinct
    /// purpose from `original_argument_metadata_for_input`: an abrupt argv read
    /// can prevent a runtime point while the compiler still visited the words.
    /// Source-only inherited entries have no compiler policy/word observation
    /// and cannot manufacture this carrier from their source naming recipe.
    pub(crate) fn original_compiler_argument_syntax<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalCompilerArgumentSyntax<'a>> {
        if tokens.source_binding.as_ref() != Some(self)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
        {
            return None;
        }
        self.native_compiler_dialect()?;
        self.compiler_source_protocol()?;
        let original = self.original_compiler_words.as_ref()?;
        let snapshot = self.compiler_lookup_state.as_ref()?;
        let analysis_owner = self.original_analysis_owner.as_ref()?;
        if original.words.as_ref() != tokens.words()
            || original.config != self.original_argument_lexer_config(tokens)?
            || snapshot.state.current_source_origin.as_ref() != Some(&original.site.source)
            || snapshot.state.baseline.registry_snapshot != Some(registry.snapshot().semantic_key())
            || self.compiler_invocation_realm() != Some(snapshot.realm)
            || !Arc::ptr_eq(&analysis_owner.source, &original.site.source)
            || analysis_owner.config != original.config
        {
            return None;
        }
        original.site.source.original_lexical_command(
            &original.words,
            original.site.offset,
            original.config,
        )?;
        let point = snapshot
            .state
            .baseline
            .metadata_context
            .source_analysis_input()?;
        let owner_input = analysis_owner.metadata.source_analysis_input()?;
        // The caller supplies the actual retained whole-input attachment. Only
        // that issuer may project the compiler point to nested-source rules;
        // normalising an arbitrary caller would erase changed whole-file axes.
        if owner_input != input
            || (point != owner_input && point != &owner_input.for_nested_source())
        {
            return None;
        }
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            point,
            original.config,
            Some(point.unit_profile()),
        )?;
        Some(OriginalCompilerArgumentSyntax {
            metadata,
            original,
            snapshot,
            namespace: &self.lookup_namespace_key,
            dialect: self.compiler_word_dialect(),
            analysis_owner,
        })
    }
}

impl SourceCommandBindings {
    /// Every actual compiler observation retains its own immutable table and
    /// namespace. Missing unanimous Native attachment cannot select one table.
    pub(crate) fn original_compiler_syntax_alternatives<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<OriginalCompilerArgumentSyntax<'a>>> {
        let owner = self.original_analysis_owner.as_ref()?;
        let binding = tokens.source_binding.as_ref()?;
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
            || !binding
                .original_analysis_owner
                .as_ref()
                .is_some_and(|actual| Arc::ptr_eq(actual, owner))
            || owner.metadata.source_analysis_input() != Some(input)
            || !self
                .matches_original_source_image(owner.source.source_image(), input.lexer_config())
        {
            return None;
        }
        self.original_compiler_syntax_visits_for_owner(tokens, input, registry, owner, offset)
    }

    /// Shared actual-visit validation. The ordinary consumer first checks its
    /// attached owner; restoration instead authenticates the current recorded
    /// child at the source producer before publishing an attachment.
    pub(super) fn original_compiler_syntax_visits_for_owner<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
        owner: &'a Arc<super::OriginalSourceAnalysisOwner>,
        offset: Option<u32>,
    ) -> Option<Vec<OriginalCompilerArgumentSyntax<'a>>> {
        if !self
            .original_analysis_owner
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, owner))
        {
            return None;
        }
        let site = CommandAllocationSite {
            source: Arc::clone(&owner.source),
            offset: tokens.words().first()?.source().span.start(),
        };
        let mut visits = self
            .compiler_invocations
            .get(&site)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if let Some(children) = self.original_child_compiler_visits(&site) {
            visits.extend(children);
        }
        if visits.is_empty() {
            return None;
        }
        visits
            .into_iter()
            .map(|visit| {
                original_compiler_syntax_alternative(visit, owner, tokens, input, registry)
            })
            .collect()
    }
}

fn original_compiler_syntax_alternative<'a>(
    visit: &'a SourceCompilerInvocation,
    owner: &'a Arc<super::OriginalSourceAnalysisOwner>,
    tokens: &crate::ir::CommandTokens,
    input: &crate::analyser::ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
) -> Option<OriginalCompilerArgumentSyntax<'a>> {
    let original = visit.original_words.as_ref()?;
    let policy = visit.policy.as_ref()?;
    policy.compiler?;
    policy.source_protocol?;
    let dialect = policy.words?;
    let snapshot = &visit.table;
    if original.words.as_ref() != tokens.words()
        || original.config != input.lexer_config()
        || owner.config != original.config
        || !Arc::ptr_eq(&owner.source, &original.site.source)
        || snapshot.state.current_source_origin.as_ref() != Some(&original.site.source)
        || snapshot.state.baseline.registry_snapshot != Some(registry.snapshot().semantic_key())
        || policy.realm != snapshot.realm
    {
        return None;
    }
    original.site.source.original_lexical_command(
        &original.words,
        original.site.offset,
        original.config,
    )?;
    let point = snapshot
        .state
        .baseline
        .metadata_context
        .source_analysis_input()?;
    if point != input && point != &input.for_nested_source() {
        return None;
    }
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        point,
        original.config,
        Some(point.unit_profile()),
    )?;
    Some(OriginalCompilerArgumentSyntax {
        metadata,
        original,
        snapshot,
        namespace: &visit.namespace_key,
        dialect: Some(dialect),
        analysis_owner: owner,
    })
}

#[cfg(test)]
mod source_syntax_alternative_tests {
    use super::*;

    #[test]
    fn source_syntax_requires_every_actual_alternative_and_full_input() {
        let source = "proc f {x} {if {$x <= 1} {return 1}}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let offset = source.find("if {").unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source[offset..],
            u32::try_from(offset).unwrap(),
            input.lexer_config(),
        )
        .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            input.lexer_config(),
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let views = bindings
            .original_compiler_syntax_alternatives(&tokens, input, registry)
            .unwrap();
        assert!(!views.is_empty());
        assert!(
            views.iter().all(
                |view| view
                    .source_lookup_advice(&tokens)
                    .is_some_and(|advice| advice.closed_authored_simulation_source_lookup(
                        &tokens,
                        view.metadata(),
                        registry
                    ))
            )
        );
        let site = views[0].site().clone();
        let mut withdrawn = bindings.clone();
        let visits = withdrawn.compiler_invocations.get_mut(&site).unwrap();
        let mut missing = visits[0].clone();
        missing.policy = None;
        visits.push(missing);
        assert!(
            withdrawn
                .original_compiler_syntax_alternatives(&tokens, input, registry)
                .is_none()
        );
        let mut changed = bindings.clone();
        let visits = changed.compiler_invocations.get_mut(&site).unwrap();
        let mut opaque = visits[0].clone();
        Arc::make_mut(&mut opaque.table)
            .state
            .opaque_binding_mutation = true;
        visits.push(opaque);
        let all = changed
            .original_compiler_syntax_alternatives(&tokens, input, registry)
            .unwrap();
        assert!(all.len() > views.len());
        assert!(all.iter().any(
            |view| !view.source_lookup_advice(&tokens).is_some_and(|advice| {
                advice.closed_authored_simulation_source_lookup(&tokens, view.metadata(), registry)
            })
        ));
        let other = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            other
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_compiler_syntax_alternatives(&tokens, input, registry)
                .is_none()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(
        source: &str,
    ) -> (
        SourceCommandBindings,
        tcl_lexer::LexerConfig,
        Option<crate::environment_ingress::CapturedNativeEntry>,
    ) {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "inventory-test".to_owned(),
        };
        let entry = Some(crate::environment_ingress::captured_native_entry(profile));
        let bindings = SourceCommandBindings::analyse_in_frame_with_options(
            source,
            &frame,
            config,
            &registry,
            super::super::SourceAnalysisOptions {
                native_entry: entry
                    .as_ref()
                    .map(crate::environment_ingress::CapturedNativeEntry::as_entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        (bindings, config, entry)
    }

    #[test]
    fn unknown_iterable_retains_the_entered_expression_read_inventory() {
        let source = "proc p {l} {foreach x $l {set y [expr {$x + 1}]}}";
        let (bindings, _, _owner) = analyse(source);
        assert!(
            bindings
                .variable_accesses
                .values()
                .flatten()
                .any(|access| { access.original_spelling == "$x" }),
            "points={:?} accesses={:?}",
            bindings
                .points
                .iter()
                .map(|point| (point.offset, &point.head, point.state.has_opaque_domain()))
                .collect::<Vec<_>>(),
            bindings
                .variable_accesses
                .values()
                .flatten()
                .map(|access| (&access.source, &access.original_spelling))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn command_enumeration_condition_retains_closed_full_chunk_admission() {
        let source = "if {[llength [info commands dict]] == 0} {list UNAVAILABLE} else {array set blocked {k OLD}; set d {first NEW second OTHER}; set ok BEFORE; set entered 0; set c [catch {dict update d first ok second blocked {set entered 1}} r]; list $c $ok $entered [catch {set blocked} v] $v}";
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let fixture = tcl_registry::model::ingress::static_context_for(name);
            let profile = fixture
                .commands()
                .profile()
                .expect("selected native fixture profile");
            let registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
            let bindings = SourceCommandBindings::analyse_with_options(
                source, config, &registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                        mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            assert!(
                !bindings.native_compilation_provider_required_at(0),
                "{name}"
            );
        }
    }

    #[test]
    fn return_does_not_remove_later_compiler_admission() {
        let source = "return done; set never 1";
        let (bindings, config, _owner) = analyse(source);
        let offset = u32::try_from(source.find("set").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("", offset);
        assert_eq!(
            binding.runtime_reachability(),
            SourceRuntimeReachability::NotEntered
        );
        assert!(binding.admitted_inline_invocation().is_some());
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.lookup_command_word("set").unknown);
        let map = tcl_lexer::SourceMap::new(source);
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .find(|segment| segment.span.start() == offset)
            .unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
        tokens.source_binding = Some(binding);
        assert!(
            crate::registry_invocation::native_operation_selection_plan(
                &tokens,
                config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_grammar(
                    &config
                        .grammar_over(tcl_dialect::DialectProfile::find("tcl9.0").unwrap().grammar)
                ),
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn literal_colon_namespace_retains_its_entered_declaration_points() {
        let source =
            "namespace eval : {proc p {} {namespace export exposed; return [namespace current]}}";
        let (bindings, _, _owner) = analyse(source);
        let offset = u32::try_from(source.find("proc p").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("proc", offset);
        assert_eq!(
            binding.runtime_reachability(),
            SourceRuntimeReachability::Reached
        );
        assert_eq!(binding.lookup_namespace, ":::");
        assert!(binding.proved_handler_target().is_some(), "{binding:#?}");
        assert!(bindings.points.iter().any(|point| {
            point.head.as_deref() == Some("namespace")
                && point.offset == u32::try_from(source.find("namespace export").unwrap()).unwrap()
        }));
    }

    #[test]
    fn literal_colon_procedure_inventory_uses_the_published_slot_without_relookup() {
        let source = "namespace eval : {proc p {} {return [namespace current]}}";
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let fixture = tcl_registry::model::ingress::static_context_for(name);
            let profile = fixture
                .commands()
                .profile()
                .expect("selected native fixture profile");
            let registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            frame:
                                tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let owner_local = matches!(name, "tcl8.6" | "tcl9.0" | "tcl9.1");
            let expected_key = if owner_local { ":::::p" } else { "::p" };
            let expected_namespace = if owner_local { ":::" } else { "::" };
            assert!(
                bindings.deferred.values().any(|body| {
                    body.identity == expected_key && body.namespace == expected_namespace
                }),
                "{name}: {:?}",
                bindings
                    .deferred
                    .values()
                    .map(|body| (&body.identity, &body.namespace))
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn expanded_head_stamps_the_original_invocation_site_including_its_marker() {
        let source = "set command {set result READY}; {*}$command";
        let (bindings, config, _owner) = analyse(source);
        let map = tcl_lexer::SourceMap::new(source);
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let segment = segments.last().unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, segment);
        assert_ne!(
            tokens.argv[0].start(),
            tokens.words()[0].source().span.start()
        );
        bindings.stamp_original_tokens(&mut tokens);
        let proof = tokens.source_binding.as_ref().unwrap();
        assert_eq!(
            proof.invocation_site().unwrap().offset,
            segment.span.start()
        );
        assert_eq!(proof.evaluated_command_word(), Some("set"));
        assert_eq!(
            proof.runtime_reachability(),
            SourceRuntimeReachability::Reached
        );
    }

    #[test]
    fn dynamically_compiled_unentered_arm_remains_in_parent_compiler_inventory() {
        let source = "set flag 0; if {$flag} {set never 1}; set reached 2";
        let (bindings, config, _owner) = analyse(source);
        let offset = u32::try_from(source.find("set never").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("", offset);
        assert_eq!(
            binding.runtime_reachability(),
            SourceRuntimeReachability::NotEntered
        );
        assert!(binding.admitted_inline_invocation().is_some());
        let map = tcl_lexer::SourceMap::new(source);
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .find(|segment| {
                segment.span.start() == u32::try_from(source.find("if").unwrap()).unwrap()
            })
            .unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
        bindings.stamp_original_tokens(&mut tokens);
        assert!(tokens.nested_bindings.iter().any(|(site, proof)| {
            *site == offset
                && proof.admitted_inline_invocation().is_some()
                && proof.runtime_reachability() == SourceRuntimeReachability::NotEntered
        }));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceCompilerInvocation {
    pub source_locals:
        Option<Arc<super::compiled_preflight::ordered_locals::SourceCompilerLocalInventory>>,
    pub selection: Option<tcl_registry::native_compilation::NativeCompilationSelection>,
    pub admitted: Option<Arc<super::compiled_invocation::SourceNativeCompilerAdmission>>,
    pub operand_layout: Option<Arc<super::SourceNativeOperandLayoutProof>>,
    pub namespace_bindings:
        Option<Arc<super::compiled_invocation::SourceNativeNamespaceBindingPreparation>>,
    pub switch: Option<Arc<super::compiled_invocation::SourceNativeSwitchPreparation>>,
    pub structured: Option<Arc<super::compiled_invocation::SourceNativeStructuredPreparation>>,
    pub original_words: Option<Arc<super::compiled_invocation::SourceOriginalCompilerWords>>,
    pub policy: Option<Arc<SourceNativeCompilerPolicy>>,
    pub table: Arc<super::SourceLookupSnapshot>,
    pub namespace: String,
    pub namespace_key: super::SourceNamespaceKey,
    pub head: Option<String>,
}

pub(super) type SourceCompilerVisits =
    BTreeMap<CommandAllocationSite, Vec<SourceCompilerInvocation>>;

/// Policies of original compiler observations, independent of their changing
/// lookup/variable worlds. Missing axes remain missing and all observations
/// must agree; this carrier contains no implementation or handler authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceNativeCompilerPolicy {
    words: Option<tcl_registry::InvocationDialect>,
    source_protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    compiler: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
}

impl SourceNativeCompilerPolicy {
    pub(super) fn of_compilation(
        state: &super::ModuleCommandBindings,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Self {
        Self {
            words: state.baseline.dialect,
            source_protocol: Self::source_protocol_of(state),
            compiler: state.baseline.compilation_dialect(),
            realm,
        }
    }

    pub(super) fn source_protocol_of(
        state: &super::ModuleCommandBindings,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        match state.baseline.native_entry.as_deref() {
            Some(entry) => entry.source_string_protocol,
            None => state.baseline.dialect?.native_source_string_protocol(),
        }
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn compiler_policy_consensus_does_not_borrow_a_later_lookup_world() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let state = super::super::ModuleCommandBindings::initial(registry.commands());
        let policy = Arc::new(SourceNativeCompilerPolicy::of_compilation(
            &state,
            state.baseline.invocation_realm,
        ));
        let mut first = SourceInvocationBinding {
            compiler_policy: Some(Arc::clone(&policy)),
            ..Default::default()
        };
        let same_policy = SourceInvocationBinding {
            compiler_policy: Some(Arc::clone(&policy)),
            ..Default::default()
        };
        first.join(&same_policy);
        assert!(first.compiler_lookup_state.is_none());
        assert_eq!(first.compiler_word_dialect(), policy.words);
        assert_eq!(first.native_compiler_dialect(), policy.compiler);
        let mut different = same_policy.clone();
        Arc::make_mut(different.compiler_policy.as_mut().unwrap()).compiler = None;
        first.join(&different);
        assert!(first.compiler_word_dialect().is_none());
        assert!(first.native_compiler_dialect().is_none());
        first.join(&same_policy);
        assert!(first.native_compiler_dialect().is_none());
    }

    fn namespace_analysis(
        source: &str,
        name: &str,
    ) -> (
        SourceCommandBindings,
        tcl_lexer::LexerConfig,
        Option<crate::environment_ingress::CapturedNativeEntry>,
    ) {
        let profile = tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "namespace-preparation".to_owned(),
        };
        let entry = (tcl_registry::InvocationDialect::of_profile(profile).family()
            == Some(tcl_dialect::model::Family::Tcl))
        .then(|| crate::environment_ingress::captured_native_entry(profile));
        let inventory = SourceCommandBindings::analyse_in_frame_with_options(
            source,
            &frame,
            config,
            &registry,
            super::super::SourceAnalysisOptions {
                native_entry: entry
                    .as_ref()
                    .map(crate::environment_ingress::CapturedNativeEntry::as_entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        (inventory, config, entry)
    }

    #[test]
    fn original_structured_preparation_retains_native_recipe_and_exact_source_owner() {
        let sources = [
            "if {0} {set skipped 1} else {set kept 1}",
            "while {0} {set skipped 1}",
            "for {set x 0} {$x} {set next 1} {set body 1}",
            "catch {set protected 1} result",
            "foreach x {A} {set body 1}",
            "expr {1 + 2}",
        ];
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in sources {
                let (inventory, _, _owner) = namespace_analysis(source, name);
                let site = CommandAllocationSite {
                    source: Arc::clone(inventory.source_origin().unwrap()),
                    offset: 0,
                };
                let retained = inventory
                    .structured_compilation_at(&site)
                    .unwrap_or_else(|| panic!("{name}/{source}: original structured preparation"));
                assert_eq!(retained.compilation_site(), &site);
                assert!(
                    !inventory.native_compilation_provider_required_at(0),
                    "{name}/{source}"
                );
                let relocated = CommandAllocationSite {
                    source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(format!(
                        "{source}\n"
                    )))),
                    offset: 0,
                };
                assert!(inventory.structured_compilation_at(&relocated).is_none());
            }
        }
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let (inventory, _, _owner) =
                namespace_analysis("try {set protected 1} finally {set cleanup 1}", name);
            let site = CommandAllocationSite {
                source: Arc::clone(inventory.source_origin().unwrap()),
                offset: 0,
            };
            assert!(matches!(
                inventory.structured_compilation_at(&site).unwrap().recipe(),
                tcl_registry::native_instruction_plan::NativeInstructionPlan::Try(_)
            ));
        }
        let source = "if {0} {set skipped 1} else {set kept 1}";
        let (inventory, _, _owner) = namespace_analysis(source, "tcl8.6");
        for (child, visited) in [("set skipped", false), ("set kept", true)] {
            let offset = u32::try_from(source.find(child).unwrap()).unwrap();
            assert_eq!(
                inventory
                    .compiler_invocations
                    .keys()
                    .any(|site| site.offset == offset),
                visited
            );
        }
    }

    const SWITCH_SOURCES: [&str; 6] = [
        "switch -exact -- miss x {set kept 1} x {set masked 2} default {set done 3}",
        "switch -exact -- miss {x {set kept 1} x {set masked 2} default {set done 3}}",
        "switch -glob -- miss {x {set kept 1} x {set masked 2} default {set done 3}}",
        "switch -- miss {x {set kept 1} y - x {set forced 2} default {set done 3}}",
        "switch -- miss {*}{x {set kept 1} x {set masked 2} default {set done 3}}",
        "switch -- [set visited miss] {x {set kept 1} x {set masked 2} default {set done 3}}",
    ];

    #[test]
    fn original_each_invocation_keeps_inline_generic_and_rejected_visits() {
        use tcl_registry::native_control_compilation::NativeControlOutcome;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for (index, source) in [
                "foreach i {A} {set body $i}",
                "foreach {i a(k)} {A B} {set body 1}",
                "foreach \"{i\" {A} {set body 1}",
            ]
            .into_iter()
            .enumerate()
            {
                let (inventory, config, _owner) = namespace_analysis(source, name);
                let segment =
                    crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                        .into_iter()
                        .next()
                        .unwrap();
                let map = tcl_lexer::SourceMap::new(source);
                let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
                tokens.source_binding = Some(inventory.invocation_at_source("foreach", 0));
                let binding = tokens.source_binding.as_ref().unwrap();
                let preparation = binding
                    .original_structured_compilation(&tokens)
                    .unwrap_or_else(|| panic!("{name}/{source}: retained original Each"));
                let NativeInstructionPlan::Each(recipe) = preparation.recipe() else {
                    panic!("Each recipe")
                };
                match index {
                    0 => assert!(matches!(recipe.outcome, NativeControlOutcome::Inline(_))),
                    2 if name == "tcl8.4" => {
                        assert!(matches!(recipe.outcome, NativeControlOutcome::Rejected(_)));
                    }
                    1 | 2 => assert!(matches!(recipe.outcome, NativeControlOutcome::Generic)),
                    _ => unreachable!(),
                }
                let mut changed = tokens.clone();
                changed.argv_texts[2] = "CHANGED".to_owned();
                assert!(
                    binding.original_structured_compilation(&changed).is_none(),
                    "{name}: changed vector"
                );
                changed = tokens.clone();
                changed.synthetic = Some(crate::ir::SyntheticMarker::EmptyClause);
                assert!(
                    binding.original_structured_compilation(&changed).is_none(),
                    "{name}: synthetic vector"
                );
                let mut withdrawn = binding.clone();
                withdrawn.join(&SourceInvocationBinding::default());
                assert!(
                    withdrawn.original_structured_compilation(&tokens).is_none(),
                    "{name}: unknown alternative"
                );
                withdrawn.join(binding);
                assert!(
                    withdrawn.original_structured_compilation(&tokens).is_none(),
                    "{name}: later agreement cannot restore withdrawn visits"
                );
            }
        }
    }

    fn assert_original_switch_preflight(name: &str, row: &str) {
        let fields: Vec<_> = row.split('\t').collect();
        let case: usize = fields[0].parse().unwrap();
        let source = SWITCH_SOURCES[case];
        let (inventory, config, _owner) = namespace_analysis(source, name);
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .next()
            .unwrap();
        let map = tcl_lexer::SourceMap::new(source);
        let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
        tokens.source_binding = Some(inventory.invocation_at_source("switch", 0));
        let binding = tokens.source_binding.as_ref().unwrap();
        let original_recipe = binding
            .original_switch_compilation(&tokens)
            .expect("same original switch compiler vector");
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "CHANGED".into();
        assert!(binding.original_switch_compilation(&changed).is_none());
        let mut withdrawn = binding.clone();
        withdrawn.join(&SourceInvocationBinding::default());
        assert!(withdrawn.original_switch_compilation(&tokens).is_none());
        withdrawn.join(binding);
        assert!(withdrawn.original_switch_compilation(&tokens).is_none());
        let origin = Arc::clone(inventory.source_origin().unwrap());
        let site = CommandAllocationSite {
            source: Arc::clone(&origin),
            offset: 0,
        };
        let recipe = inventory
            .switch_compilation_at(&site)
            .expect("retained original switch compiler recipe");
        assert_eq!(original_recipe, recipe);
        assert_eq!(
            recipe.arms.len(),
            if case == 3 { 4 } else { 3 },
            "{name}/{case}"
        );
        let expected: Vec<_> = fields[3]
            .split(',')
            .map(|hex| {
                let bytes: Vec<_> = hex
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| {
                        u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap()
                    })
                    .collect();
                String::from_utf8(bytes).unwrap()
            })
            .collect();
        for variable in ["visited", "kept", "masked", "forced", "done"] {
            let needle = format!("set {variable}");
            let Some(position) = source.find(&needle) else {
                continue;
            };
            let child = CommandAllocationSite {
                source: Arc::clone(&origin),
                offset: u32::try_from(position).unwrap(),
            };
            assert_eq!(
                inventory.compiler_invocations.contains_key(&child),
                expected.iter().any(|name| name == variable),
                "{name}/{case}/{variable}"
            );
            if expected.iter().any(|name| name == variable) {
                assert!(
                    inventory
                        .invocation_at_source("set", child.offset)
                        .admitted_inline_invocation()
                        .is_some(),
                    "{name}/{case}/{variable}: original child admission"
                );
            }
        }
        assert!(
            !inventory.native_compilation_provider_required_at(0),
            "{name}/{case}"
        );
        let relocated = CommandAllocationSite {
            source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(format!(
                "{source}\n"
            )))),
            offset: 0,
        };
        assert!(inventory.switch_compilation_at(&relocated).is_none());
    }

    #[test]
    fn original_switch_preflight_uses_native_arm_spans_and_duplicate_masking() {
        let fixtures = [
            (
                "tcl8.5",
                include_str!("../../tests/data/native_switch_compiler_sources/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_switch_compiler_sources/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_switch_compiler_sources/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_switch_compiler_sources/9.1.0.tsv"),
            ),
        ];
        for (name, table) in fixtures {
            for row in table.lines().skip(1) {
                assert_original_switch_preflight(name, row);
            }
        }
        for name in ["tcl8.4", "jim"] {
            let (inventory, _, _owner) = namespace_analysis(SWITCH_SOURCES[1], name);
            let site = CommandAllocationSite {
                source: Arc::clone(inventory.source_origin().unwrap()),
                offset: 0,
            };
            assert!(
                inventory.switch_compilation_at(&site).is_none(),
                "{name}: no C switch compiler recipe"
            );
        }
    }

    #[test]
    fn original_namespace_binding_preparation_keeps_declined_prefix_geometry() {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        use tcl_registry::native_namespace_binding_compilation::{
            NativeNamespaceBindingOutcome, NativeNamespaceBindingVisit,
        };
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "variable v [set reached 1] {a)} [set later 2]";
            let (inventory, _, _owner) = namespace_analysis(source, name);
            let site = CommandAllocationSite {
                source: Arc::clone(inventory.source_origin().unwrap()),
                offset: 0,
            };
            let prepared = inventory.namespace_binding_preparation_at(&site).unwrap();
            assert_eq!(
                prepared.recipe().outcome,
                NativeNamespaceBindingOutcome::Generic,
                "{name}"
            );
            assert_eq!(prepared.recipe().bindings.len(), 1, "{name}");
            assert_eq!(prepared.recipe().bindings[0].local, b"v");
            assert_eq!(
                prepared.recipe().visits,
                vec![
                    NativeNamespaceBindingVisit::DeclareLocal(b"v".to_vec()),
                    NativeNamespaceBindingVisit::Word(NativeCompilerWordOperand::Original(1)),
                    NativeNamespaceBindingVisit::Word(NativeCompilerWordOperand::Original(2)),
                ]
            );
            assert!(
                inventory.native_compilation_failure_at(0).is_none(),
                "{name}"
            );
            assert!(
                !inventory.native_compilation_provider_required_at(0),
                "{name}"
            );
            for child in ["set reached", "set later"] {
                let offset = u32::try_from(source.find(child).unwrap()).unwrap();
                assert!(
                    inventory
                        .invocation_at_source("set", offset)
                        .admitted_inline_invocation()
                        .is_some(),
                    "{name}: original child {child}"
                );
            }
            assert!(
                inventory
                    .namespace_binding_preparation_at(&CommandAllocationSite {
                        source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(
                            format!("{source}\n")
                        ))),
                        offset: 0,
                    })
                    .is_none()
            );
        }
    }

    #[test]
    fn original_switch_source_admission_withdraws_changed_or_truncated_vectors() {
        use tcl_registry::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
            NativeCompilationSelection,
        };
        let source = SWITCH_SOURCES[1];
        let (inventory, config, _owner) = namespace_analysis(source, "tcl9.1");
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        inventory.stamp_original_tokens(&mut tokens);
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        assert!(
            matches!(
                crate::registry_invocation::native_compilation_syntax(
                    &registry, &tokens, dialect, context
                )
                .unwrap()
                .1,
                NativeCompilationSelection::Inline { .. }
            ),
            "original compiler={:?}; requested={:?}; source protocol={:?}; original config={:?}",
            tokens
                .source_binding
                .as_ref()
                .and_then(SourceInvocationBinding::native_compiler_dialect),
            dialect,
            tokens
                .source_binding
                .as_ref()
                .and_then(SourceInvocationBinding::compiler_source_protocol),
            tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_lexer_config_for_tokens(&tokens))
        );
        let mut truncated = tokens.clone();
        truncated.word_exprs.truncate(3);
        truncated.argv_texts.truncate(3);
        assert_eq!(
            crate::registry_invocation::native_compilation_syntax(
                &registry, &truncated, dialect, context
            )
            .unwrap()
            .1,
            NativeCompilationSelection::Unknown
        );
        let mut changed = tokens.clone();
        let crate::ir::WordExpr::BracedLiteral { text, .. } =
            changed.word_exprs.last_mut().unwrap()
        else {
            panic!("original list arms")
        };
        *text = "x {set replacement 9}".to_owned();
        assert_eq!(
            crate::registry_invocation::native_compilation_syntax(
                &registry, &changed, dialect, context
            )
            .unwrap()
            .1,
            NativeCompilationSelection::Unknown
        );
        let mut absent = tokens.clone();
        absent
            .source_binding
            .as_mut()
            .unwrap()
            .join(&SourceInvocationBinding::default());
        assert_eq!(
            crate::registry_invocation::native_compilation_syntax(
                &registry, &absent, dialect, context
            )
            .unwrap()
            .1,
            NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn original_variable_join_with_unknown_withdraws_compiler_word_owner() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "set x";
            let (inventory, config, _owner) = namespace_analysis(source, name);
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                &command,
            );
            inventory.stamp_original_tokens(&mut tokens);
            let original = tokens.source_binding.as_ref().unwrap();
            assert!(
                original.original_compiler_source(&tokens).is_some(),
                "{name}"
            );
            assert!(original.compiler_word_dialect().is_some(), "{name}");
            tokens
                .source_binding
                .as_mut()
                .unwrap()
                .join(&SourceInvocationBinding::default());
            let joined = tokens.source_binding.as_ref().unwrap();
            assert!(
                joined.original_compiler_source(&tokens).is_none(),
                "{name}: Unknown cannot retain original compiler words"
            );
            assert!(
                joined.compiler_word_dialect().is_none(),
                "{name}: Unknown cannot retain compiler policy"
            );
        }
    }

    #[test]
    fn original_namespace_source_admission_rejects_changed_or_truncated_vectors() {
        use tcl_registry::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
            NativeCompilationSelection,
        };
        let source = "variable v 1 w 2";
        let (inventory, config, _owner) = namespace_analysis(source, "tcl9.1");
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        inventory.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap().clone();
        assert!(binding.original_compiler_source(&tokens).is_some());
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        assert!(
            matches!(
                crate::registry_invocation::native_compilation_syntax(
                    &registry, &tokens, dialect, context
                )
                .unwrap()
                .1,
                NativeCompilationSelection::Inline { .. }
            ),
            "original compiler={:?}; requested={:?}; source protocol={:?}; original config={:?}",
            tokens
                .source_binding
                .as_ref()
                .and_then(SourceInvocationBinding::native_compiler_dialect),
            dialect,
            tokens
                .source_binding
                .as_ref()
                .and_then(SourceInvocationBinding::compiler_source_protocol),
            tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_lexer_config_for_tokens(&tokens))
        );
        let mut truncated = tokens.clone();
        truncated.word_exprs.truncate(3);
        truncated.argv_texts.truncate(3);
        assert!(binding.original_compiler_source(&truncated).is_none());
        assert_eq!(
            crate::registry_invocation::native_compilation_syntax(
                &registry, &truncated, dialect, context
            )
            .unwrap()
            .1,
            NativeCompilationSelection::Unknown
        );
        let mut joined = binding.clone();
        joined.join(&SourceInvocationBinding::default());
        assert!(joined.original_compiler_source(&tokens).is_none());
        joined.join(&binding);
        assert!(joined.original_compiler_source(&tokens).is_none());
    }

    #[test]
    fn original_namespace_dynamic_tail_admission_uses_the_selected_release() {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for (source, head) in [
                ("variable ${ns}::v", "variable"),
                ("global ${ns}::v", "global"),
            ] {
                let (inventory, _, _owner) = namespace_analysis(source, name);
                let binding = inventory.invocation_at_source(head, 0);
                assert_eq!(
                    matches!(
                        binding.native_compilation_admission_selection(),
                        NativeCompilationSelection::Inline { .. }
                    ),
                    matches!(name, "tcl8.6" | "tcl9.0" | "tcl9.1"),
                    "{name}: {binding:?}"
                );
            }
            for (source, head) in [("variable ${ns}v", "variable"), ("global ${ns}v", "global")] {
                let (without_tail, _, _owner) = namespace_analysis(source, name);
                assert_eq!(
                    without_tail
                        .invocation_at_source(head, 0)
                        .native_compilation_admission_selection(),
                    NativeCompilationSelection::Generic,
                    "{name}"
                );
            }
        }
    }
}

impl SourceInvocationBinding {
    /// Exact original structured compiler visits, including Generic and
    /// Rejected selections. Changed source vectors or compiler policy abstain;
    /// this receipt grants neither handler entry nor normal completion.
    #[must_use]
    pub fn original_structured_compilation(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&super::compiled_invocation::SourceNativeStructuredPreparation> {
        if !tokens.original_words_match_argv_text() {
            return None;
        }
        self.original_compiler_source(tokens)?;
        self.native_compiler_dialect()?;
        self.compiler_source_protocol()?;
        let original = self.original_compiler_words.as_ref()?;
        let preparation = self.native_structured_preparation.as_deref()?;
        (preparation.compilation_site == original.site
            && self.native_compilation_admission_selection()
                != tcl_registry::native_compilation::NativeCompilationSelection::Unknown)
            .then_some(preparation)
    }

    /// Original Inline Control structure for a genuine retained compiler entry.
    /// Generic/rejected recipes and absent physical compiler registrations
    /// cannot supply this source purpose. Runtime entry remains independent.
    pub(crate) fn original_inline_control_compilation(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&super::compiled_invocation::SourceNativeStructuredPreparation> {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        use tcl_registry::native_control_compilation::NativeControlOutcome;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        let preparation = self.original_structured_compilation(tokens)?;
        if !matches!(
            self.native_compilation_admission_selection(),
            NativeCompilationSelection::Inline { .. }
        ) || !matches!(&preparation.recipe, NativeInstructionPlan::Control(control)
                if matches!(&control.outcome, NativeControlOutcome::Inline(_)))
        {
            return None;
        }
        let original = self.original_compiler_words.as_ref()?;
        let snapshot = self.compiler_lookup_state.as_ref()?;
        let entry = snapshot.state.baseline.native_entry.as_deref()?;
        let prerequisite = preparation.dependency.compiler_prerequisite.as_deref()?;
        let policy = self.compiler_policy.as_deref()?;
        let input = snapshot
            .state
            .baseline
            .metadata_context
            .source_analysis_input()?;
        if prerequisite.interpreter != entry.interpreter
            || snapshot.state.current_source_origin.as_ref() != Some(&original.site.source)
            || preparation.compilation_site != original.site
            || self.original_lexer_config_for_tokens(tokens)? != original.config
            || original.config != input.lexer_config()
            || policy.words != snapshot.state.baseline.dialect
            || policy.compiler != snapshot.state.baseline.compilation_dialect()
            || policy.source_protocol != entry.source_string_protocol
            || policy.realm != snapshot.realm
        {
            return None;
        }
        Some(preparation)
    }

    /// Join compiler-only source structure to the caller's complete input.
    /// The retained compiler point supplies metadata, never a new handler,
    /// Normal completion, native execution entry or body activation.
    pub(crate) fn original_inline_control_metadata_for_input<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'a>> {
        self.original_inline_control_compilation(tokens)?;
        let snapshot = self.compiler_lookup_state.as_ref()?;
        if snapshot.state.baseline.registry_snapshot.as_ref()
            != Some(&registry.snapshot().semantic_key())
        {
            return None;
        }
        let original = self.original_compiler_words.as_ref()?;
        let point = snapshot
            .state
            .baseline
            .metadata_context
            .source_analysis_input()?;
        if point != input {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            point,
            original.config,
            Some(point.unit_profile()),
        )
    }

    /// Original switch compiler body visits, independent of runtime arm entry.
    /// Changed source vectors or an unknown joined admission retain no recipe.
    #[must_use]
    pub fn original_switch_compilation(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&tcl_registry::native_switch_compilation::NativeSwitchInstruction> {
        if !tokens.original_words_match_argv_text() {
            return None;
        }
        self.original_compiler_source(tokens)?;
        self.native_compiler_dialect()?;
        self.compiler_source_protocol()?;
        let original = self.original_compiler_words.as_ref()?;
        let preparation = self.native_switch_preparation.as_deref()?;
        (preparation.compilation_site == original.site
            && self.native_compilation_admission_selection()
                != tcl_registry::native_compilation::NativeCompilationSelection::Unknown)
            .then_some(&preparation.recipe)
    }

    /// Authenticate the complete original compiler vector, including source
    /// channel and delimiters. A transformed or truncated token vector abstains.
    pub(crate) fn original_compiler_source<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<(&'a tcl_lexer::SourceImage, &'a [crate::ir::WordExpr], u32)> {
        let original = self.original_compiler_words.as_ref()?;
        (tokens.synthetic.is_none() && tokens.words() == original.words.as_ref()).then(|| {
            (
                original.site.source.source_image(),
                original.words.as_ref(),
                original.site.offset,
            )
        })
    }

    /// Borrow the original compiler receipt retained before IR expression
    /// transformations. Admission and emitted operation guards remain required.
    pub(crate) fn retained_original_compiler_source(
        &self,
    ) -> Option<(&tcl_lexer::SourceImage, &[crate::ir::WordExpr], u32)> {
        let original = self.original_compiler_words.as_ref()?;
        Some((
            original.site.source.source_image(),
            original.words.as_ref(),
            original.site.offset,
        ))
    }

    /// Runtime entry is not implied by an admitted native compiler recipe.
    #[must_use]
    pub fn runtime_reachability(&self) -> SourceRuntimeReachability {
        self.runtime_reachability
    }

    /// Original word-value policy retained at compiler visitation. This is
    /// source decoding evidence, not a compiler engine or normal-handler proof.
    pub(crate) fn compiler_word_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.compiler_policy.as_ref()?.words
    }

    /// Original source-string issuer, independent of the installed compiler.
    pub(crate) fn compiler_source_protocol(
        &self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        self.compiler_policy.as_ref()?.source_protocol
    }

    /// Native dialect retained at compiler visitation, without runtime facts.
    #[must_use]
    pub fn native_compiler_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.compiler_policy.as_ref()?.compiler
    }

    pub(crate) fn compiler_invocation_realm(&self) -> Option<tcl_dialect::model::InvocationRealm> {
        Some(self.compiler_policy.as_ref()?.realm)
    }
}

impl SourceCommandBindings {
    /// Original structured compiler recipe whose source, frame and registration
    /// are unanimous across all retained observations of this exact allocation.
    /// The recipe supplies compiler visits, not runtime body or value authority.
    #[must_use]
    pub fn structured_compilation_at(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<&super::compiled_invocation::SourceNativeStructuredPreparation> {
        let visits = self
            .original_child_compiler_visits(site)
            .unwrap_or_else(|| {
                self.compiler_invocations
                    .get(site)
                    .map_or_else(Vec::new, |visits| visits.iter().collect())
            });
        let first = *visits.first()?;
        let preparation = first.structured.as_deref()?;
        (preparation.compilation_site == *site
            && visits.iter().all(|visit| {
                visit.structured == first.structured
                    && visit.policy == first.policy
                    && visit.namespace_key == first.namespace_key
                    && visit.table == first.table
                    && visit.original_words == first.original_words
            }))
        .then_some(preparation)
    }

    /// Exact original compiler preparation, without runtime alias/store facts.
    /// All retained compiler observations must agree on recipe and guards.
    #[must_use]
    pub fn namespace_binding_preparation_at(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<&super::compiled_invocation::SourceNativeNamespaceBindingPreparation> {
        let visits = self
            .original_child_compiler_visits(site)
            .unwrap_or_else(|| {
                self.compiler_invocations
                    .get(site)
                    .map_or_else(Vec::new, |visits| visits.iter().collect())
            });
        let first = *visits.first()?;
        let preparation = first.namespace_bindings.as_deref()?;
        (preparation.compilation_site() == site
            && visits.iter().all(|visit| {
                visit.namespace_bindings == first.namespace_bindings
                    && visit.policy == first.policy
                    && visit.namespace_key == first.namespace_key
                    && visit.table == first.table
            }))
        .then_some(preparation)
    }

    /// Exact original switch compiler arms, with unanimous source and guards.
    /// This grants body-compilation geometry, not runtime arm selection.
    #[must_use]
    pub fn switch_compilation_at(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<&tcl_registry::native_switch_compilation::NativeSwitchInstruction> {
        let visits = self
            .original_child_compiler_visits(site)
            .unwrap_or_else(|| {
                self.compiler_invocations
                    .get(site)
                    .map_or_else(Vec::new, |visits| visits.iter().collect())
            });
        let first = *visits.first()?;
        let preparation = first.switch.as_deref()?;
        (preparation.compilation_site == *site
            && visits.iter().all(|visit| {
                visit.switch == first.switch
                    && visit.policy == first.policy
                    && visit.namespace_key == first.namespace_key
                    && visit.table == first.table
                    && visit.original_words == first.original_words
            }))
        .then_some(&preparation.recipe)
    }

    pub(super) fn retain_compiler_invocations(
        &mut self,
        incoming: &BTreeMap<CommandAllocationSite, Vec<SourceCompilerInvocation>>,
    ) {
        for (site, invocations) in incoming {
            let retained = self.compiler_invocations.entry(site.clone()).or_default();
            for invocation in invocations {
                if !retained.contains(invocation) {
                    retained.push(invocation.clone());
                }
            }
        }
    }

    pub(super) fn attach_compiler_invocation(
        &self,
        mut binding: SourceInvocationBinding,
        origin: Option<&Arc<super::SourceOriginId>>,
        offset: u32,
    ) -> SourceInvocationBinding {
        let Some(origin) = origin else {
            return binding;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let reached = binding.runtime_reachability == SourceRuntimeReachability::Reached;
        let original = self.original_child_compiler_visits(&site);
        if reached && original.is_none() {
            return binding;
        }
        if !reached && binding.runtime_reachability != SourceRuntimeReachability::Conditional {
            binding.runtime_reachability = self.missing_runtime_reachability(&site);
        }
        let invocations = original.unwrap_or_else(|| {
            self.compiler_invocations
                .get(&site)
                .map_or_else(Vec::new, |visits| visits.iter().collect())
        });
        let Some(first) = invocations.first() else {
            return binding;
        };
        if reached && first.namespace_key != binding.lookup_namespace_key {
            return binding;
        }
        attach_unanimous_original_compiler_metadata(&mut binding, first, &invocations, reached);
        binding
    }

    /// An original child compiler visit selects its namespace independently
    /// of whether any runtime branch enters the child. The same parent
    /// admission, source image and entry prerequisites must still be retained.
    pub(crate) fn compiled_script_namespace_context(
        &self,
        source: &super::ExecutedScriptSource,
    ) -> Option<super::SourceNamespaceKey> {
        let mut contexts = self.compiled_children.iter().flat_map(|(site, children)| {
            children.iter().filter_map(move |child| {
                if &child.script != source {
                    return None;
                }
                let current = child.script.origin == site.source
                    && child.script.base() == site.offset
                    && !self
                        .compilation_provider_required
                        .contains(&child.enclosing)
                    && matches!(
                        child.parent.as_ref(),
                        super::compiled_invocation::SourceNativeCompilerAdmission::Inline(_)
                    )
                    && self
                        .compiler_invocations
                        .get(&match child.parent.as_ref() {
                            super::compiled_invocation::SourceNativeCompilerAdmission::Inline(
                                parent,
                            ) => parent.compilation_site.clone(),
                            super::compiled_invocation::SourceNativeCompilerAdmission::Named(_) => {
                                unreachable!("checked inline child")
                            }
                        })
                        .is_some_and(|parents| {
                            !parents.is_empty()
                                && parents.iter().all(|visit| {
                                    visit.admitted.as_deref() == Some(child.parent.as_ref())
                                        && visit.namespace_key == child.parent_namespace_key
                                        && visit.table == child.table
                                })
                        });
                Some(current.then_some(&child.namespace_key))
            })
        });
        let first = contexts.next()??;
        contexts
            .all(|key| key == Some(first))
            .then(|| first.clone())
    }

    /// A prospective runtime preview does not replace the visits owned by an
    /// admitted original parent. A changed parent compilation does withdraw them.
    fn original_child_compiler_visits(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<Vec<&SourceCompilerInvocation>> {
        let mut visits = Vec::new();
        for (child_site, children) in &self.compiled_children {
            if child_site.source != site.source || child_site.offset > site.offset {
                continue;
            }
            for child in children {
                if site
                    .offset
                    .checked_sub(child_site.offset)
                    .is_none_or(|offset| (offset as usize) >= child.script.text.len())
                    || child.script.origin != child_site.source
                    || child.script.base() != child_site.offset
                    || self
                        .compilation_provider_required
                        .contains(&child.enclosing)
                {
                    continue;
                }
                let super::compiled_invocation::SourceNativeCompilerAdmission::Inline(parent) =
                    child.parent.as_ref()
                else {
                    continue;
                };
                if !self
                    .compiler_invocations
                    .get(&parent.compilation_site)
                    .is_some_and(|parents| {
                        !parents.is_empty()
                            && parents.iter().all(|visit| {
                                visit.admitted.as_deref() == Some(child.parent.as_ref())
                                    && visit.namespace_key == child.parent_namespace_key
                                    && visit.table == child.table
                            })
                    })
                {
                    continue;
                }
                if let Some(original) = child.compiler_visits.get(site) {
                    visits.extend(original);
                }
            }
        }
        (!visits.is_empty()).then_some(visits)
    }

    fn missing_runtime_reachability(
        &self,
        site: &CommandAllocationSite,
    ) -> SourceRuntimeReachability {
        let covering = self
            .runtime_coverage
            .iter()
            .filter(|(chunk, (length, _))| {
                chunk.source == site.source
                    && chunk.offset <= site.offset
                    && u64::from(site.offset) < u64::from(chunk.offset) + *length as u64
            })
            .map(|(_, (_, closed))| *closed)
            .collect::<Vec<_>>();
        if !covering.is_empty() && covering.iter().all(|closed| *closed) {
            SourceRuntimeReachability::NotEntered
        } else {
            SourceRuntimeReachability::Unknown
        }
    }

    pub(super) fn record_runtime_coverage(
        &mut self,
        origin: Option<&Arc<super::SourceOriginId>>,
        base: u32,
        length: usize,
        outcomes: &super::SourceOutcomes,
    ) {
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let Some(origin) = origin else {
            return;
        };
        let closed = outcomes
            .normal
            .as_ref()
            .is_none_or(|state| !state.opaque_domain)
            && outcomes.abrupt.iter().all(|(route, state)| {
                !state.opaque_domain
                    && !matches!(
                        route,
                        Route::Unknown
                            | Route::UnknownAbrupt
                            | Route::Tcl(CompletionCode::Other(_))
                    )
            });
        self.runtime_coverage
            .entry(CommandAllocationSite {
                source: Arc::clone(origin),
                offset: base,
            })
            .and_modify(|previous| {
                previous.1 &= closed && previous.0 == length;
            })
            .or_insert((length, closed));
    }
}

impl SourceInvocationBinding {
    /// Original static naming word selected by its exact retained source site.
    /// A first representative token may select its complete grouped word;
    /// arbitrary substrings, transformed vectors and conflicting configs abstain.
    #[must_use]
    pub fn original_source_name_key_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        rules: tcl_syntax::word_rules::WordValueRules,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameKey> {
        self.original_source_name_word_at_span(span, config, rules, policy)
            .map(|(_, key)| key)
    }

    pub(crate) fn original_source_name_word_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        rules: tcl_syntax::word_rules::WordValueRules,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(
        &crate::ir::WordExpr,
        crate::signature_scan::scope::SignatureSourceNameKey,
    )> {
        let site = self.invocation_site()?;
        let (image, words) =
            if let Some(observations) = self.declaration_layout_observations.as_deref() {
                let first = super::declaration_layout::original_declaration_layouts(observations)?
                    .next()?;
                if first.config != config || first.entry.source().origin != site.source {
                    return None;
                }
                (site.source.source_image(), first.words.as_ref())
            } else {
                let original = self.original_compiler_words.as_ref()?;
                if original.site != *site || original.config != config {
                    return None;
                }
                (original.site.source.source_image(), original.words.as_ref())
            };
        original_name_word_from_layout(site, image, words, span, config, rules, policy)
    }
}

fn original_name_word_from_layout<'a>(
    site: &CommandAllocationSite,
    image: &tcl_lexer::SourceImage,
    words: &'a [crate::ir::WordExpr],
    span: tcl_lexer::Span,
    config: tcl_lexer::LexerConfig,
    rules: tcl_syntax::word_rules::WordValueRules,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<(
    &'a crate::ir::WordExpr,
    crate::signature_scan::scope::SignatureSourceNameKey,
)> {
    let native = crate::registry_invocation::original_native_compiler_words(
        image,
        words,
        site.offset,
        config,
    )?;
    let mut selected = words.iter().zip(&native).filter(|(word, native)| {
        word.source().span == span
            || native
                .tokens()
                .first()
                .is_some_and(|token| token.span == span)
    });
    let (word, native) = selected.next()?;
    if selected.next().is_some() {
        return None;
    }
    let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
        native, rules, policy,
    )?;
    Some((word, key))
}

impl SourceCommandBindings {
    /// Issue an original naming occurrence from the existing retained command
    /// layout inventory. A token selects only its complete canonical word;
    /// transformed, materialised or conflicting owners cannot supply one.
    #[must_use]
    pub fn original_source_name_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        rules: tcl_syntax::word_rules::WordValueRules,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        let origin = self.root_origin.as_ref()?;
        let upper = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: span.start(),
        };
        let mut found = None;
        for (site, observations) in self.declaration_layouts.range(..=upper).rev() {
            if &site.source != origin {
                break;
            }
            let Some(original) =
                super::declaration_layout::original_declaration_layouts(observations)
            else {
                continue;
            };
            let first = original.clone().next()?;
            if first.config != config || first.entry.source().origin != *origin {
                continue;
            }
            // A cheap rejection only; the shared native vector below owns the
            // exact complete-word and first-token correspondence.
            if !first
                .words
                .iter()
                .any(|word| word.source().span.start() == span.start())
            {
                continue;
            }
            let (_, input) = original_name_word_from_layout(
                site,
                origin.source_image(),
                &first.words,
                span,
                config,
                rules,
                policy,
            )?;
            let occurrence =
                crate::signature_scan::original_name::SourceOriginalNameOccurrence::new(
                    site, input,
                )?;
            if found
                .as_ref()
                .is_some_and(|previous| previous != &occurrence)
            {
                return None;
            }
            found = Some(occurrence);
        }
        found
    }
}

fn attach_unanimous_original_compiler_metadata(
    binding: &mut SourceInvocationBinding,
    first: &SourceCompilerInvocation,
    invocations: &[&SourceCompilerInvocation],
    reached: bool,
) {
    // Only compiler metadata is attached. Actual handler identities, contents,
    // evaluated argv and successful transfer remain absent/unknown.
    binding.native_compilation_admission = first.selection;
    binding
        .native_operand_layout
        .clone_from(&first.operand_layout);
    binding
        .native_compiler_admission
        .clone_from(&first.admitted);
    binding
        .native_structured_preparation
        .clone_from(&first.structured);
    binding.native_switch_preparation.clone_from(&first.switch);
    binding
        .original_compiler_words
        .clone_from(&first.original_words);
    if !reached {
        binding.lookup_namespace.clone_from(&first.namespace);
        binding
            .lookup_namespace_key
            .clone_from(&first.namespace_key);
        binding.lookup_word.clone_from(&first.head);
    }
    binding.compiler_lookup_state = Some(Arc::clone(&first.table));
    binding.compiler_policy = first.policy.clone().filter(|policy| {
        invocations
            .iter()
            .all(|visit| visit.policy.as_ref() == Some(policy))
    });
    for alternative in &invocations[1..] {
        if alternative.structured != first.structured {
            binding.native_structured_preparation = None;
        }
        if alternative.switch != first.switch {
            binding.native_switch_preparation = None;
        }
        if alternative.original_words != first.original_words {
            binding.original_compiler_words = None;
        }
        if alternative.operand_layout != first.operand_layout {
            binding.native_operand_layout = None;
        }
        if alternative.selection != first.selection {
            binding.native_compilation_admission =
                Some(tcl_registry::native_compilation::NativeCompilationSelection::Unknown);
        }
        if alternative.admitted != first.admitted {
            binding.native_compiler_admission = None;
        }
        if alternative.namespace_key != first.namespace_key || alternative.table != first.table {
            binding.compiler_lookup_state = None;
            binding.native_structured_preparation = None;
            binding.native_switch_preparation = None;
        }
    }
}

#[cfg(test)]
mod original_inline_control_source_tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::registry_invocation::{InvocationMetadataContext, OwnedInvocationMetadataContext};

    struct Fixture {
        input: ResolvedAnalysisInput,
        tokens: Vec<crate::ir::CommandTokens>,
        _owner: tcl_vm::Vm,
    }

    fn fixture(source: &str, environment: &str) -> Fixture {
        fixture_with_bindings(source, environment).0
    }

    fn fixture_with_bindings(source: &str, environment: &str) -> (Fixture, SourceCommandBindings) {
        let environment = tcl_registry::model::ingress::resolve_known_environment(environment)
            .expect("actual original compiler environment");
        let profile = environment.unit_profile();
        let context = environment.default_context_registry();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = super::super::SourceAnalysisEntry {
            metadata_context: OwnedInvocationMetadataContext::for_source_input(Some(&input)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..crate::environment_ingress::authoring_native_compilation()
            },
            native_entry: Some(Arc::new(native)),
            ..Default::default()
        };
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            context.commands(),
            entry.options(),
        );
        let image = tcl_lexer::SourceImage::document(source);
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let tokens = segments
            .iter()
            .map(|segment| {
                let mut tokens =
                    crate::ir::CommandTokens::from_segmented(&image.source_map(), config, segment);
                bindings.stamp_original_tokens(&mut tokens);
                tokens
            })
            .collect();
        (
            Fixture {
                input,
                tokens,
                _owner: owner,
            },
            bindings,
        )
    }

    #[test]
    fn restored_nested_compiler_carriers_issue_only_current_complete_source_owners() {
        let (actual, bindings) =
            fixture_with_bindings("set result [expr {1 + [expr {3 + 4}]}]", "tcl8.6");
        let owner = &bindings;
        let parent = &actual.tokens[0];
        let registry = actual.input.borrowed_context_registry().commands();
        assert!(parent.nested_bindings.len() >= 2);
        for (offset, _) in &parent.nested_bindings {
            let raw = owner.invocation_at_source("", *offset);
            assert!(raw.original_analysis_owner.is_none());
            let (_, mut child) = raw.original_recorded_command().unwrap();
            child.source_binding = Some(raw.clone());
            assert!(
                owner
                    .original_compiler_syntax_alternatives(&child, &actual.input, registry)
                    .is_none()
            );
            assert!(owner.restored_child_analysis_owner(*offset, &raw).is_some());
            assert!(
                owner
                    .restored_child_analysis_owner(offset + 1, &raw)
                    .is_none()
            );
        }
        let mut restored = parent.clone();
        restored.source_binding = Some(owner.invocation_at_source("", 0));
        owner.restore_original_nested_bindings(&mut restored);
        for (offset, restored_binding) in &restored.nested_bindings {
            assert!(Arc::ptr_eq(
                restored_binding.original_analysis_owner.as_ref().unwrap(),
                owner.original_analysis_owner.as_ref().unwrap(),
            ));
            let mut raw = restored_binding.clone();
            raw.original_analysis_owner = None;
            assert_eq!(raw, owner.invocation_at_source("", *offset));
            let (_, mut child) = restored_binding.original_recorded_command().unwrap();
            child.source_binding = Some(restored_binding.clone());
            assert!(
                owner
                    .original_compiler_syntax_alternatives(&child, &actual.input, registry)
                    .is_some()
            );
        }
        let foreign = fixture("set result [expr {1 + [expr {3 + 4}]}]", "tcl8.6");
        for (offset, binding) in &foreign.tokens[0].nested_bindings {
            assert!(
                owner
                    .restored_child_analysis_owner(*offset, binding)
                    .is_none()
            );
        }
        let foreign_context = Arc::new(
            actual
                .input
                .context_registry()
                .with_command_store(Arc::clone(registry)),
        );
        let foreign_input = ResolvedAnalysisInput::new(
            actual.input.analyser_profile(),
            actual.input.unit_profile(),
            foreign_context,
            actual.input.lexer_config(),
        );
        let mut changed = owner.clone();
        let mut attachment = owner.original_analysis_owner.as_deref().unwrap().clone();
        attachment.metadata =
            OwnedInvocationMetadataContext::for_source_input(Some(&foreign_input));
        changed.original_analysis_owner = Some(Arc::new(attachment));
        for (offset, _) in &parent.nested_bindings {
            let raw = owner.invocation_at_source("", *offset);
            assert!(
                changed
                    .restored_child_analysis_owner(*offset, &raw)
                    .is_none()
            );
        }
        let mut duplicate = parent.clone();
        duplicate
            .nested_bindings
            .push(duplicate.nested_bindings[0].clone());
        duplicate.source_binding = Some(owner.invocation_at_source("", 0));
        owner.restore_original_nested_bindings(&mut duplicate);
        let repeated = duplicate.nested_bindings[0].0;
        assert!(
            duplicate
                .nested_bindings
                .iter()
                .filter(|(offset, _)| *offset == repeated)
                .all(|(_, binding)| binding.original_analysis_owner.is_none())
        );
    }

    #[test]
    fn original_compiler_syntax_keeps_the_actual_input_and_vector_owner() {
        for environment in ["tcl8.4", "tcl9.1"] {
            let fixture = fixture(
                "if {$a} {set one 1}; if {$b} {set two 2}; if {$c} {set three 3}",
                environment,
            );
            let tokens = fixture.tokens.last().unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let registry = fixture.input.borrowed_context_registry().commands();
            let syntax = binding
                .original_compiler_argument_syntax(tokens, &fixture.input, registry)
                .expect("actual compiler observation after the earlier undefined read");
            assert_eq!(syntax.words(), tokens.words());
            assert_eq!(syntax.config(), fixture.input.lexer_config());
            assert!(syntax.metadata().source_analysis_input().is_some());

            let foreign_context = Arc::new(
                fixture
                    .input
                    .context_registry()
                    .with_command_store(Arc::clone(registry)),
            );
            let foreign = ResolvedAnalysisInput::new(
                fixture.input.analyser_profile(),
                fixture.input.unit_profile(),
                foreign_context,
                fixture.input.lexer_config(),
            );
            assert!(
                binding
                    .original_compiler_argument_syntax(tokens, &foreign, registry)
                    .is_none()
            );
            let mut config = fixture.input.lexer_config();
            config.leading_bom = tcl_lexer::LeadingBom::Skip;
            let foreign = ResolvedAnalysisInput::new(
                fixture.input.analyser_profile(),
                fixture.input.unit_profile(),
                fixture.input.context_registry(),
                config,
            );
            assert!(
                binding
                    .original_compiler_argument_syntax(tokens, &foreign, registry)
                    .is_none()
            );
            for axis in 0..6 {
                let mut changed = tokens.clone();
                match axis {
                    0 => changed.synthetic = Some(crate::ir::SyntheticMarker::UnseenCall),
                    1 => changed.argv_texts[1].push('x'),
                    2 => {
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .compiler_lookup_state = None;
                    }
                    3 => changed.source_binding.as_mut().unwrap().compiler_policy = None,
                    4 => {
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .original_compiler_words = None;
                    }
                    _ => {
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .original_analysis_owner = None;
                    }
                }
                assert!(
                    changed
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .original_compiler_argument_syntax(&changed, &fixture.input, registry)
                        .is_none(),
                    "{environment} axis={axis}"
                );
            }
        }
    }

    #[test]
    fn original_inline_control_accepts_the_same_cached_complete_input_owner() {
        // naming.compiler.original-analysis-metadata-context

        // Captured software compiler/source ownership only; no Native execution observation.
        for environment in ["tcl8.4", "tcl9.1"] {
            let fixture = fixture(
                "if {$a} {set one 1}; if {$b} {set two 2}; if {$c} {set three 3}",
                environment,
            );
            let context = tcl_registry::model::ingress::resolve_known_environment(environment)
                .unwrap()
                .default_context_registry();
            assert!(Arc::ptr_eq(&context, &fixture.input.context_registry()));
            let same = ResolvedAnalysisInput::new(
                fixture.input.analyser_profile(),
                fixture.input.unit_profile(),
                context,
                fixture.input.lexer_config(),
            );
            assert_eq!(same, fixture.input);
            let tokens = fixture.tokens.last().unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding
                    .original_inline_control_metadata_for_input(
                        tokens,
                        &same,
                        fixture.input.borrowed_context_registry().commands(),
                    )
                    .is_some()
            );
        }
    }

    #[test]
    fn original_inline_control_source_requires_the_actual_compiler_and_complete_owner() {
        // Source-only compiler structure; no runtime handler or Native execution grant.
        for environment in ["tcl8.4", "tcl9.1"] {
            let fixture = fixture(
                "if {$a} {set one 1}; if {$b} {set two 2}; if {$c} {set three 3}",
                environment,
            );
            let registry = fixture.input.borrowed_context_registry().commands();
            let tokens = fixture.tokens.last().unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding
                    .original_inline_control_metadata_for_input(tokens, &fixture.input, registry)
                    .is_some()
            );
            let mut changed = tokens.clone();
            changed.argv_texts[1].push('x');
            assert!(
                binding
                    .original_inline_control_metadata_for_input(&changed, &fixture.input, registry)
                    .is_none()
            );
            let mut changed_word = tokens.clone();
            let crate::ir::WordExpr::BracedLiteral { text, .. } = &mut changed_word.word_exprs[1]
            else {
                panic!("original literal predicate")
            };
            text.push('x');
            changed_word.argv_texts[1].push('x');
            assert!(
                binding
                    .original_inline_control_metadata_for_input(
                        &changed_word,
                        &fixture.input,
                        registry
                    )
                    .is_none()
            );
            let foreign_context =
                tcl_registry::model::ingress::resolve_known_environment(environment)
                    .unwrap()
                    .default_context_registry();
            // Resolving the same environment returns its cached original Arc.
            // Assemble an independent actual context over the same command store.
            let foreign_context = Arc::new(
                foreign_context.with_command_store(Arc::clone(foreign_context.commands())),
            );
            assert!(!Arc::ptr_eq(
                &foreign_context,
                &fixture.input.context_registry()
            ));
            let foreign = ResolvedAnalysisInput::new(
                fixture.input.analyser_profile(),
                fixture.input.unit_profile(),
                foreign_context,
                fixture.input.lexer_config(),
            );
            assert!(
                binding
                    .original_inline_control_metadata_for_input(tokens, &foreign, registry)
                    .is_none()
            );
            let mut changed_config = fixture.input.lexer_config();
            changed_config.leading_bom = tcl_lexer::LeadingBom::Skip;
            let changed_input = ResolvedAnalysisInput::new(
                fixture.input.analyser_profile(),
                fixture.input.unit_profile(),
                fixture.input.context_registry(),
                changed_config,
            );
            assert!(
                binding
                    .original_inline_control_metadata_for_input(tokens, &changed_input, registry)
                    .is_none()
            );
            for axis in 0..4 {
                let mut withdrawn = tokens.clone();
                let actual = withdrawn.source_binding.as_mut().unwrap();
                match axis {
                    0 => actual.compiler_lookup_state = None,
                    1 => actual.compiler_policy = None,
                    2 => {
                        let snapshot =
                            Arc::make_mut(actual.compiler_lookup_state.as_mut().unwrap());
                        Arc::make_mut(&mut snapshot.state.baseline).native_entry = None;
                    }
                    _ => {
                        let preparation =
                            Arc::make_mut(actual.native_structured_preparation.as_mut().unwrap());
                        preparation.dependency.compiler_prerequisite = None;
                    }
                }
                assert!(
                    actual
                        .original_inline_control_metadata_for_input(
                            tokens,
                            &fixture.input,
                            registry
                        )
                        .is_none(),
                    "{environment} axis={axis}"
                );
            }
        }
    }

    #[test]
    fn original_inline_control_source_keeps_generic_rejected_and_missing_purposes_unavailable() {
        use tcl_registry::native_control_compilation::NativeControlOutcome;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        let selected = fixture("if {$a} {set one 1}; if {$b} {set two 2}", "tcl9.1");
        let registry = selected.input.borrowed_context_registry().commands();
        let tokens = selected.tokens.last().unwrap();
        let mut generic = tokens.clone();
        let binding = generic.source_binding.as_mut().unwrap();
        let preparation = Arc::make_mut(binding.native_structured_preparation.as_mut().unwrap());
        let NativeInstructionPlan::Control(control) = &mut preparation.recipe else {
            panic!("genuine original Control")
        };
        control.outcome = NativeControlOutcome::Generic;
        assert!(
            binding
                .original_inline_control_metadata_for_input(tokens, &selected.input, registry)
                .is_none()
        );
        let dynamic = fixture("if {$a} $body", "tcl9.1");
        let dynamic_tokens = dynamic.tokens.first().unwrap();
        assert!(
            dynamic_tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_inline_control_compilation(dynamic_tokens)
                .is_none()
        );
        let rejected = fixture("if {)} {set one 1}", "tcl9.1");
        let rejected_tokens = rejected.tokens.first().unwrap();
        assert!(
            rejected_tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_inline_control_compilation(rejected_tokens)
                .is_none()
        );
        let mut missing = tokens.clone();
        missing.source_binding = None;
        let context = InvocationMetadataContext::for_source_input(
            registry,
            &selected.input,
            selected.input.lexer_config(),
            Some(selected.input.unit_profile()),
        )
        .unwrap();
        assert!(
            crate::registry_invocation::original_structured_invocation_with_metadata_context(
                registry, context, &missing
            )
            .is_none()
        );
    }
}
