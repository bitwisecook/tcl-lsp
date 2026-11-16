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
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
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
            include_str!("../../../tests/data/native_switch_compiler_sources/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tests/data/native_switch_compiler_sources/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tests/data/native_switch_compiler_sources/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tests/data/native_switch_compiler_sources/9.1.0.tsv"),
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
                    source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(format!(
                        "{source}\n"
                    )))),
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
    let crate::ir::WordExpr::BracedLiteral { text, .. } = changed.word_exprs.last_mut().unwrap()
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
        crate::registry_invocation::native_compilation_syntax(&registry, &absent, dialect, context)
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
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
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
