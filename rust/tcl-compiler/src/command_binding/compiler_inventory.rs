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

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(source: &str) -> (SourceCommandBindings, tcl_lexer::LexerConfig) {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "inventory-test".to_owned(),
        };
        let bindings = SourceCommandBindings::analyse_in_frame_with_options(
            source,
            &frame,
            config,
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        (bindings, config)
    }

    #[test]
    fn unknown_iterable_retains_the_entered_expression_read_inventory() {
        let source = "proc p {l} {foreach x $l {set y [expr {$x + 1}]}}";
        let (bindings, _) = analyse(source);
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
        let (bindings, config) = analyse(source);
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
        let (bindings, _) = analyse(source);
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
        let (bindings, config) = analyse(source);
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
        let (bindings, config) = analyse(source);
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
    ) -> (SourceCommandBindings, tcl_lexer::LexerConfig) {
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
                native_entry: entry.as_ref(),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        (inventory, config)
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
                let (inventory, _) = namespace_analysis(source, name);
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
            let (inventory, _) =
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
        let (inventory, _) = namespace_analysis(source, "tcl8.6");
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
                let (inventory, config) = namespace_analysis(source, name);
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
        let (inventory, config) = namespace_analysis(source, name);
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
            let (inventory, _) = namespace_analysis(SWITCH_SOURCES[1], name);
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
            let (inventory, _) = namespace_analysis(source, name);
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
        let (inventory, config) = namespace_analysis(source, "tcl9.1");
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
            let (inventory, config) = namespace_analysis(source, name);
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
        let (inventory, config) = namespace_analysis(source, "tcl9.1");
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
                let (inventory, _) = namespace_analysis(source, name);
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
                let (without_tail, _) = namespace_analysis(source, name);
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
        if !tokens.words_align_with_argv_text()
            || tokens
                .words()
                .iter()
                .zip(&tokens.argv_texts)
                .any(|(word, text)| word.legacy_text() != *text)
        {
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

    /// Original switch compiler body visits, independent of runtime arm entry.
    /// Changed source vectors or an unknown joined admission retain no recipe.
    #[must_use]
    pub fn original_switch_compilation(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&tcl_registry::native_switch_compilation::NativeSwitchInstruction> {
        if !tokens.words_align_with_argv_text()
            || tokens
                .words()
                .iter()
                .zip(&tokens.argv_texts)
                .any(|(word, text)| word.legacy_text() != *text)
        {
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
            if std::env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_COMPILER_ATTACHMENT offset={offset} reason=reached-without-child selection={:?}",
                    binding.native_compilation_admission
                );
            }
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
            if std::env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_COMPILER_ATTACHMENT offset={offset} reason=no-visits reachability={:?} selection={:?}",
                    binding.runtime_reachability, binding.native_compilation_admission
                );
            }
            return binding;
        };
        if reached && first.namespace_key != binding.lookup_namespace_key {
            if std::env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_COMPILER_ATTACHMENT offset={offset} reason=namespace-mismatch head={:?} visits={} selection={:?}",
                    first.head,
                    invocations.len(),
                    binding.native_compilation_admission
                );
            }
            return binding;
        }
        attach_unanimous_original_compiler_metadata(&mut binding, first, &invocations, reached);
        if std::env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            eprintln!(
                "NATIVE_COMPILER_ATTACHMENT offset={offset} head={:?} visits={} selections={:?} final={:?} original_words={} policy={} table={}",
                first.head,
                invocations.len(),
                invocations
                    .iter()
                    .map(|visit| visit.selection)
                    .collect::<Vec<_>>(),
                binding.native_compilation_admission,
                binding.original_compiler_words.is_some(),
                binding.compiler_policy.is_some(),
                binding.compiler_lookup_state.is_some()
            );
        }
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
