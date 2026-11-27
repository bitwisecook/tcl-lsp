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
        tcl_source::segment::segment_commands_image_with_offset_and_config(&image, 0, config)
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

fn assert_expression_syntax_inventory_boundary(
    owner: &SourceCommandBindings,
    parent: &crate::ir::CommandTokens,
) {
    let offset = parent.nested_bindings.last().unwrap().0;
    let site = CommandAllocationSite {
        source: Arc::clone(owner.root_origin.as_ref().unwrap()),
        offset,
    };
    assert!(!owner.expression_syntax_children.is_empty());
    assert!(owner.compiler_invocations.get(&site).is_none());
    assert!(owner.original_child_compiler_visits(&site).is_none());
    let raw = owner.invocation_at_source("", offset);
    assert!(owner.restored_child_analysis_owner(offset, &raw).is_some());
    let mut withdrawn = owner.clone();
    withdrawn.expression_syntax_children.clear();
    assert!(
        withdrawn
            .restored_child_analysis_owner(offset, &raw)
            .is_none()
    );
    let mut missing_parent = owner.clone();
    missing_parent.compiler_invocations.clear();
    assert!(
        missing_parent
            .restored_child_analysis_owner(offset, &raw)
            .is_none()
    );
}

#[test]
fn restored_nested_compiler_carriers_issue_only_current_complete_source_owners() {
    let (actual, bindings) =
        fixture_with_bindings("set result [expr {1 + [expr {3 + 4}]}]", "tcl8.6");
    let owner = &bindings;
    let parent = &actual.tokens[0];
    let registry = actual.input.borrowed_context_registry().commands();
    assert!(parent.nested_bindings.len() >= 2);
    assert_expression_syntax_inventory_boundary(owner, parent);
    for (offset, _) in parent.nested_bindings.iter() {
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
    for (offset, restored_binding) in restored.nested_bindings.iter() {
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
    for (offset, binding) in foreign.tokens[0].nested_bindings.iter() {
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
    attachment.metadata = OwnedInvocationMetadataContext::for_source_input(Some(&foreign_input));
    changed.original_analysis_owner = Some(Arc::new(attachment));
    for (offset, _) in parent.nested_bindings.iter() {
        let raw = owner.invocation_at_source("", *offset);
        assert!(
            changed
                .restored_child_analysis_owner(*offset, &raw)
                .is_none()
        );
    }
    let mut duplicate = parent.clone();
    let first = duplicate.nested_bindings[0].clone();
    duplicate.nested_bindings_mut().push(first);
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
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
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
        let tcl_source::words::WordExpr::BracedLiteral { text, .. } =
            &mut changed_word.word_exprs[1]
        else {
            panic!("original literal predicate")
        };
        text.push('x');
        changed_word.argv_texts[1].push('x');
        assert!(
            binding
                .original_inline_control_metadata_for_input(&changed_word, &fixture.input, registry)
                .is_none()
        );
        let foreign_context = tcl_registry::model::ingress::resolve_known_environment(environment)
            .unwrap()
            .default_context_registry();
        // Resolving the same environment returns its cached original Arc.
        // Assemble an independent actual context over the same command store.
        let foreign_context =
            Arc::new(foreign_context.with_command_store(Arc::clone(foreign_context.commands())));
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
                    let snapshot = Arc::make_mut(actual.compiler_lookup_state.as_mut().unwrap());
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
                    .original_inline_control_metadata_for_input(tokens, &fixture.input, registry)
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
