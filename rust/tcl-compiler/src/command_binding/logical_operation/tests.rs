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
use crate::analyser::{Analyser, ResolvedAnalysisInput};
use crate::command_binding::{
    SourceAnalysisOptions, SourceCommandSlotPresence, SourceInvocationBinding,
};

fn input() -> ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    )
}

fn final_binding(source: &str, input: &ResolvedAnalysisInput) -> SourceInvocationBinding {
    let analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, input.analyser_profile().name);
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, input.lexer_config())
            .pop()
            .unwrap();
    let binding = analysis
        .retained_command_realm()
        .unwrap()
        .invocation_at_source("", command.span.start());
    assert_eq!(binding.logical_source_name_advice_input(), Some(input));
    assert!(
        binding
            .original_recorded_head_name_input()
            .is_none_or(|head| {
                head.policy().authority()
                    == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
            })
    );
    binding
}

#[test]
fn logical_namespace_cell_continuation_keeps_the_independent_storage_facet() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let input = input();
    let registry = input.borrowed_context_registry().commands();
    let options = SourceAnalysisOptions::for_logical_source(&input).unwrap();
    let binding = final_binding("set", &input);
    let state = binding.lookup_state.as_ref().unwrap().state.clone();
    let namespace = state.source_variables.namespace_identity.clone().unwrap();
    let frame = state.variable_frame.clone();
    let context = super::super::root_source_execution_context(
        &frame,
        &state.source_variables.namespace,
        &namespace,
        input.lexer_config(),
        registry,
        options,
        tcl_lexer::SourceSegmentationPurpose::WholeInput,
    );
    let access = crate::var_resolve::resolve_literal_access(
        "suffix",
        &state.source_variables,
        false,
        registry,
        tcl_registry::TraceOperation::Write,
    );
    let storage = Some(tcl_registry::f5::VariableStorageDomain::InterpreterNamespace);
    assert_eq!(access.cell.as_ref().unwrap().storage_domain, storage);
    assert!(logical_cell_is_local(&access, &state, context));
    assert_eq!(access.cell.as_ref().unwrap().storage_domain, storage);
    let mut worker = access.clone();
    worker.cell.as_mut().unwrap().storage_domain =
        Some(tcl_registry::f5::VariableStorageDomain::WorkerNamespace);
    assert!(!logical_cell_is_local(&worker, &state, context));
    let mut foreign = access.clone();
    foreign.cell.as_mut().unwrap().owner = crate::place::CellOwner::Namespace("::other".into());
    assert!(!logical_cell_is_local(&foreign, &state, context));
    // A same-display Native coordinate supplies no authored correspondence.
    let native = SourceNamespaceKey::Native(
        tcl_runtime_api::native_compilation::NativeNamespaceContext {
            interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                owner: 37,
                interpreter: 13,
            },
            token: 99,
            path: tcl_core_types::ByteNamespacePath::root(),
        },
    );
    assert_eq!(native.display(), namespace.display());
    let mut native_access = access.clone();
    native_access.cell.as_mut().unwrap().owner =
        crate::place::CellOwner::NamespaceIdentity(Box::new(native));
    assert!(!logical_cell_is_local(&native_access, &state, context));
    let mut recreated = access.clone();
    recreated.cell.as_mut().unwrap().generation = crate::place::CellGeneration::After(17);
    assert!(!logical_cell_is_local(&recreated, &state, context));
    let mut physical = access.clone();
    physical.cell.as_mut().unwrap().interpreter = Some("retained interpreter".into());
    assert!(!logical_cell_is_local(&physical, &state, context));
    let mut missing = state.clone();
    super::super::Arc::make_mut(&mut missing.baseline).logical_source_input = None;
    assert!(!logical_cell_is_local(&access, &missing, context));
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    assert!(!logical_cell_is_local(
        &access,
        &state,
        SourceExecutionContext { config, ..context }
    ));
}

#[test]
fn logical_scalar_continuation_keeps_original_values_and_captured_set_operands() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let input = input();
    for source in [
        "proc foo_hi {} {}; rename foo_hi {}; set suffix _hi; foo$suffix",
        "proc foo_hi {} {}; rename foo_hi {}; interp alias {} write {} set suffix; write _hi; foo$suffix",
        "proc foo_é {} {}; rename foo_é {}; interp alias {} ::écrire {} set suffix; ::écrire _é; foo$suffix",
    ] {
        let binding = final_binding(source, &input);
        assert_eq!(
            binding.selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Absent,
            "{source}"
        );
        assert!(binding.variable_context.execution_name_policy.is_none());
        assert!(binding.variable_context.interpreter.is_none());
        assert!(binding.variable_context.execution.is_none());
    }
    for source in [
        "proc foo_hi {} {}; rename foo_hi {}; proc set args {}; set suffix _hi; foo$suffix",
        "proc foo_hi {} {}; rename foo_hi {}; set suffix $unknown; foo$suffix",
        "proc foo_hi {} {}; rename foo_hi {}; mystery; set suffix _hi; foo$suffix",
    ] {
        assert_eq!(
            final_binding(source, &input).selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Unknown,
            "{source}"
        );
    }
}

#[test]
fn logical_original_transfers_preserve_unicode_registry_moves_and_aliases() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    // These are conditional authored continuations, not Native rename or
    // alias execution, abrupt-edge exclusion or successful allocation.
    let input = input();
    for source in [
        "rename proc ::α; ::α subject {} {}; subject",
        "interp alias {} ::α {} proc; ::α subject {} {}; subject",
        "rename proc ::α; interp alias {} ::β {} ::α; ::β subject {} {}; subject",
    ] {
        let binding = final_binding(source, &input);
        assert_eq!(
            binding.selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Present,
            "{source}"
        );
        assert!(binding.variable_context.execution_name_policy.is_none());
    }
}

#[test]
fn logical_original_transfers_preserve_moves_deletes_and_scalar_head_values() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let input = input();
    for (source, presence) in [
        (
            "proc original {} {}; rename original moved; moved",
            SourceCommandSlotPresence::Present,
        ),
        (
            "proc original {} {}; rename original moved; original",
            SourceCommandSlotPresence::Absent,
        ),
        (
            "proc original {} {}; rename original {}; original",
            SourceCommandSlotPresence::Absent,
        ),
        (
            "proc original {} {}; rename original ::fresh::nested::moved; ::fresh::nested::moved",
            SourceCommandSlotPresence::Present,
        ),
        (
            "proc foo_hi {} {}; rename foo_hi {}; set suffix _hi; foo$suffix",
            SourceCommandSlotPresence::Absent,
        ),
        (
            "proc foo_hi {} {}; rename foo_hi {}; set suffix _hi; proc foo_hi {} {}; foo$suffix",
            SourceCommandSlotPresence::Present,
        ),
    ] {
        assert_eq!(
            final_binding(source, &input).selected_slot_diagnostic_presence(),
            presence,
            "{source}"
        );
    }
    let source = "proc foo_hi {} {}; rename foo_hi {}; set suffix _hi; foo$suffix";
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    assert_eq!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W123)
            .count(),
        1
    );
}

fn assert_logical_source_prefix_inputs_refused(
    target: &crate::command_binding::SourceCommandTarget,
    input: &crate::analyser::ResolvedAnalysisInput,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
) {
    assert!(
        target
            .original_logical_prepended_source_inputs(input, image, config)
            .is_none()
    );
}

fn assert_logical_name_prefix_inputs_refused(
    target: &crate::command_binding::SourceCommandTarget,
    input: &crate::analyser::ResolvedAnalysisInput,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
) {
    assert!(
        target
            .original_logical_prepended_name_inputs(input, image, config)
            .is_none()
    );
}

fn assert_logical_prefix_source_anchors(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    offset: u32,
) {
    assert_eq!(words.command(), "string");
    assert_eq!(words.arguments().len(), 3);
    assert!(words.operands()[0].is_none() && words.operands()[1].is_none());
    assert_eq!(
        words.operands()[2].as_ref().unwrap().span().start(),
        offset + 4
    );
}

#[test]
fn logical_source_prefix_words_keep_actual_alias_lineage_without_name_key_policy() {
    let mut profile = tcl_registry::model::ingress::resolve_environment("f5-irules")
        .analyser_profile()
        .clone();
    profile.name = "original-logical-source-prefix-without-name-policy";
    let profile = profile.intern();
    assert!(
        tcl_registry::InvocationDialect::of_profile(profile)
            .authored_name_policy()
            .is_none()
    );
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig {
            escapes: tcl_registry::model::ingress::resolve_environment("tcl8.6")
                .analyser_profile()
                .grammar
                .escapes,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        },
    );
    for source in [
        "interp alias {} idx {} string index held; idx 1",
        "interp alias {} inner {} string index held; interp alias {} idx {} inner; idx 1",
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        let offset = u32::try_from(source.rfind("idx 1").unwrap()).unwrap();
        let binding = analysis
            .retained_command_realm()
            .unwrap()
            .invocation_at_source("", offset);
        let [target] = binding.targets.as_slice() else {
            panic!("actual single Logical alias target")
        };
        let image = tcl_lexer::SourceImage::document(source);
        let original = target
            .original_logical_prepended_source_inputs(&input, &image, input.lexer_config())
            .expect("actual original Source words without a donated naming policy");
        assert_eq!(
            original
                .iter()
                .map(crate::command_binding::OriginalLogicalSourceNameInput::bytes)
                .collect::<Vec<_>>(),
            [b"index".as_slice(), b"held".as_slice()]
        );
        assert!(
            original
                .iter()
                .all(|word| word.original_word().span().start() < offset)
        );
        assert!(target.original_prepended_name_inputs().is_none());
        assert!(
            target
                .original_logical_prepended_name_inputs(&input, &image, input.lexer_config())
                .is_none()
        );
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            source, &analysis, offset,
        )
        .expect("readonly original alias source schema");
        assert_logical_prefix_source_anchors(&words, offset);
        let mut config = input.lexer_config();
        config.strict_quoting ^= true;
        assert_logical_source_prefix_inputs_refused(target, &input, &image, config);
        assert_logical_source_prefix_inputs_refused(
            target,
            &input,
            &tcl_lexer::SourceImage::document(&source.replace("held", "away")),
            input.lexer_config(),
        );
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
            input.lexer_config(),
        );
        assert_logical_source_prefix_inputs_refused(target, &foreign, &image, input.lexer_config());
        let mut missing = target.clone();
        missing.logical_original_prefix = None;
        assert_logical_source_prefix_inputs_refused(&missing, &input, &image, input.lexer_config());
    }
}

#[test]
fn logical_captured_prefix_keeps_original_words_without_native_prefix_permission() {
    for source in [
        "interp alias {} branch {} if 1\nbranch {puts $value}",
        "interp alias {} inner {} if 1\ninterp alias {} branch {} inner\nbranch {puts $value}",
    ] {
        let input = input();
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "tcl");
        let head = u32::try_from(source.rfind("branch {").unwrap()).unwrap();
        let binding = analysis
            .retained_command_realm()
            .unwrap()
            .invocation_at_source("", head);
        let [target] = binding.targets.as_slice() else {
            panic!("one actual Logical alias target")
        };
        let image = tcl_lexer::SourceImage::document(source);
        let captured = target
            .original_logical_prepended_name_inputs(&input, &image, input.lexer_config())
            .unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].bytes(), b"1");
        assert_eq!(
            captured[0].policy().authority(),
            tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        );
        assert_eq!(
            captured[0]
                .original_word_key()
                .unwrap()
                .original_word()
                .image(),
            &image
        );
        assert!(target.original_prepended_name_inputs().is_none());
        assert!(binding.proved_handler_target().is_none());
        let stale = tcl_lexer::SourceImage::document(&format!("{source} "));
        assert_logical_name_prefix_inputs_refused(target, &input, &stale, input.lexer_config());
        let mut changed = input.lexer_config();
        changed.strict_quoting = !changed.strict_quoting;
        assert_logical_name_prefix_inputs_refused(target, &input, &image, changed);
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::new(
                input.context_registry().with_command_store(
                    input
                        .context_registry()
                        .commands()
                        .snapshot()
                        .shared_registry(),
                ),
            ),
            input.lexer_config(),
        );
        assert_logical_name_prefix_inputs_refused(target, &foreign, &image, input.lexer_config());
        let mut changed = target.clone();
        changed.prepended[0] =
            crate::registry_invocation::EffectiveInvocationWord::Literal("0".to_owned());
        assert_logical_name_prefix_inputs_refused(&changed, &input, &image, input.lexer_config());
        let lookup_world = &binding.lookup_state.as_ref().unwrap().state;
        let cloned = lookup_world.clone();
        assert!(
            target
                .logical_original_prefix
                .as_ref()
                .unwrap()
                .matches_state(&cloned)
        );
        let mut missing = cloned.clone();
        missing.logical_alias_prefixes = Arc::new(std::collections::BTreeMap::new());
        assert_ne!(&missing, lookup_world);
        let mut joined = cloned;
        joined.join(&missing);
        let unavailable = super::source_binding_projection(
            &joined,
            "branch",
            &SourceNamespaceKey::authored("::"),
        );
        assert!(unavailable.targets.iter().all(|target| {
            target
                .original_logical_prepended_name_inputs(&input, &image, input.lexer_config())
                .is_none()
        }));
    }
}

#[test]
fn logical_original_alias_capture_composes_registry_operations_at_the_call() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let input = input();
    for source in [
        "proc victim {} {}; interp alias {} drop {} rename victim; drop {}; victim",
        "proc foo_hi {} {}; rename foo_hi {}; interp alias {} write {} set suffix; write _hi; foo$suffix",
        "proc victim {} {}; interp alias {} inner {} rename; interp alias {} outer {} inner victim; outer {}; victim",
    ] {
        assert_eq!(
            final_binding(source, &input).selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Absent,
            "{source}"
        );
    }
    let source = "interp alias {} ensure {} package require; ensure Tcl";
    let binding = final_binding(source, &input);
    assert_eq!(binding.targets.len(), 1);
    assert!(binding.targets[0].registry_backed);
    assert_eq!(
        binding.targets[0].prepended,
        vec![crate::registry_invocation::EffectiveInvocationWord::Literal("require".into())]
    );
    assert!(
        binding
            .original_recorded_head_name_input()
            .is_none_or(|head| {
                head.policy().authority()
                    == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
            })
    );
}

#[test]
fn logical_dispatch_projection_keeps_exact_targets_without_native_receipts() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let input = input();
    for (source, captured) in [
        ("package require Tcl", false),
        (
            "interp alias {} ensure {} package require; ensure Tcl",
            true,
        ),
        (
            "interp alias {} ::assuré {} package require; ::assuré Tcl",
            true,
        ),
    ] {
        let binding = final_binding(source, &input);
        let target = binding
            .proved_target()
            .expect("original Logical source target");
        assert!(target.registry_backed, "{source}");
        assert_eq!(
            target.prepended,
            if captured {
                vec![crate::registry_invocation::EffectiveInvocationWord::Literal("require".into())]
            } else {
                Vec::new()
            },
            "{source}",
        );
        assert!(
            binding
                .original_recorded_head_name_input()
                .is_none_or(|head| {
                    head.policy().authority()
                        == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                })
        );
        assert!(binding.proved_handler_target().is_none());
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.native_handler_envelope.is_none());
        assert!(binding.native_compiler_admission.is_none());
        assert!(binding.variable_context.execution_name_policy.is_none());
        assert!(binding.variable_context.interpreter.is_none());
        assert!(binding.variable_context.execution.is_none());
    }
}

#[test]
fn logical_dispatch_projection_refuses_missing_changed_and_uninterpreted_owners() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    use crate::command_binding::SourceCommandBindings;
    use crate::registry_invocation::OwnedInvocationMetadataContext;
    let input = input();
    let actual = input.context_registry();
    let source = "interp alias {} ensure {} package require; ensure Tcl";
    let offset = u32::try_from(source.rfind("ensure Tcl").unwrap()).unwrap();
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        actual.commands(),
        SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    );
    // The caller's reporting name never selects the target. The immutable
    // point retains the original interpreted head at this exact source site.
    assert!(
        bindings
            .invocation_at_source("different", offset)
            .proved_target()
            .is_some()
    );
    let foreign = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    let mut changed_config = input.lexer_config();
    changed_config.strict_quoting = !changed_config.strict_quoting;
    let changed = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&actual),
        changed_config,
    );
    for (label, owner) in [
        (
            "missing metadata",
            OwnedInvocationMetadataContext::Unavailable,
        ),
        (
            "standalone metadata",
            OwnedInvocationMetadataContext::Standalone,
        ),
        (
            "foreign store",
            OwnedInvocationMetadataContext::for_source_input(Some(&foreign)),
        ),
        (
            "changed grammar",
            OwnedInvocationMetadataContext::for_source_input(Some(&changed)),
        ),
    ] {
        let mut withdrawn = bindings.clone();
        let indices = withdrawn.dispatch_points.get(&offset).unwrap().clone();
        for index in indices {
            let point = &mut withdrawn.points[index];
            Arc::make_mut(&mut point.state.baseline).metadata_context = owner.clone();
            point.invalidate_derived();
        }
        let binding = withdrawn.invocation_at_source("ensure", offset);
        assert!(binding.unknown, "{label}");
        assert!(binding.targets.is_empty(), "{label}");
        assert!(binding.declared_command.is_none(), "{label}");
        assert!(binding.catalogue_command.is_none(), "{label}");
        assert!(
            binding.original_recorded_head_name_input().is_none(),
            "{label}"
        );
        assert!(binding.proved_handler_target().is_none(), "{label}");
    }
    for withdraw_head in [false, true] {
        let mut withdrawn = bindings.clone();
        let indices = withdrawn.dispatch_points.get(&offset).unwrap().clone();
        for index in indices {
            let point = &mut withdrawn.points[index];
            if withdraw_head {
                point.head = None;
            } else {
                Arc::make_mut(&mut point.state.baseline).logical_source_input = None;
            }
            point.invalidate_derived();
        }
        let binding = withdrawn.invocation_at_source("ensure", offset);
        assert!(binding.unknown);
        assert_eq!(
            binding.targets,
            [] as [crate::command_binding::SourceCommandTarget; 0]
        );
        assert!(binding.proved_handler_target().is_none());
    }
    let dynamic = final_binding(
        "interp alias {} ensure {} package require; $unknown Tcl",
        &input,
    );
    assert!(dynamic.unknown);
    assert_eq!(
        dynamic.targets,
        [] as [crate::command_binding::SourceCommandTarget; 0]
    );
}

#[test]
fn native_source_dispatch_requires_its_independent_original_head_receipt() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    // This checks the selected source recipe's gate, without a Native
    // process, entered handler, frame, value or Normal-completion claim.
    use crate::command_binding::SourceCommandBindings;
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let native = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    assert!(!native.has_logical_source_name_context());
    let mut bindings = SourceCommandBindings::analyse_with_options(
        "package require Tcl",
        config,
        context.commands(),
        SourceAnalysisOptions {
            metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                Some(&native),
            ),
            invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
                &native,
            )),
            ..Default::default()
        },
    );
    let original = bindings.invocation_at_source("package", 0);
    assert!(original.logical_source_name_advice_input().is_none());
    assert!(original.original_recorded_head_name_input().is_some());
    assert!(original.proved_target().is_some());
    let indices = bindings.dispatch_points.get(&0).unwrap().clone();
    for index in indices {
        let point = &mut bindings.points[index];
        point.original_head_input = None;
        point.invalidate_derived();
    }
    let withdrawn = bindings.invocation_at_source("package", 0);
    assert!(withdrawn.unknown);
    assert_eq!(
        withdrawn.targets,
        [] as [crate::command_binding::SourceCommandTarget; 0]
    );
    assert!(withdrawn.declared_command.is_none());
    assert!(withdrawn.catalogue_command.is_none());
}

#[test]
fn logical_original_operation_refusals_do_not_recover_from_reporting_state() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let input = input();
    for source in [
        "proc victim {} {}; mystery; rename victim {}; victim",
        "proc victim {} {}; rename $which {}; victim",
        "proc victim {} {}; proc rename args {}; rename victim {}; victim",
        "proc victim {} {}; interp alias {} drop foreign rename victim; drop {}; victim",
        "proc victim {} {}; interp alias {} drop {} rename victim; proc rename args {}; drop {}; victim",
        "proc victim {} {}; proc occupied {} {}; rename victim occupied; victim",
        "proc foo_hi {} {}; rename foo_hi {}; set suffix(index) _hi; foo$suffix",
        "proc foo_hi {} {}; rename foo_hi {}; set suffix $unknown; foo$suffix",
        "proc victim {} {}; trace add command victim delete observer; rename victim {}; victim",
        "proc foo_hi {} {}; rename foo_hi {}; trace add variable suffix write observer; set suffix _hi; foo$suffix",
        "interp alias {} cycle {} cycle; proc victim {} {}; victim",
    ] {
        assert_ne!(
            final_binding(source, &input).selected_slot_diagnostic_presence(),
            SourceCommandSlotPresence::Absent,
            "{source}"
        );
    }
}

fn with_store_fixture(
    test: impl FnOnce(
        &crate::ir::CommandTokens,
        &SourceCommandTarget,
        ModuleCommandBindings,
        SourceExecutionContext<'_>,
    ),
) {
    let input = input();
    let actual = input.context_registry();
    let registry = actual.commands();
    let options = SourceAnalysisOptions {
        logical_source_input: Some(&input),
        invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
            &input,
        )),
        ..SourceAnalysisOptions::default()
    };
    let bindings = super::super::SourceCommandBindings::analyse_with_options(
        "set suffix _hi",
        input.lexer_config(),
        registry,
        options,
    );
    let mut points = bindings.dispatch_points_at(0);
    let point = points.next().expect("genuine before-set source point");
    assert!(points.next().is_none());
    assert_eq!(point.head.as_deref(), Some("set"));
    let state = point.state.clone();
    assert_eq!(state.logical_source_name_advice_input(), Some(&input));
    let image = state.current_source_origin.as_ref().unwrap().source_image();
    let namespace = point.namespace_key.clone();
    let frame = state.variable_frame.clone();
    assert_eq!(
        state.source_variables.namespace_identity.as_ref(),
        Some(&namespace)
    );
    let context = super::super::root_source_execution_context(
        &frame,
        &point.namespace,
        &namespace,
        input.lexer_config(),
        registry,
        options,
        tcl_lexer::SourceSegmentationPurpose::WholeInput,
    );
    let command =
        crate::segmenter::segment_commands_image_with_offset_and_config(image, 0, context.config)
            .unwrap()
            .remove(0);
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        context.config,
        &command,
    );
    let selected = point.lookup_binding("set");
    assert!(!selected.unknown && !selected.may_be_absent);
    assert_eq!(selected.targets.len(), 1);
    test(&tokens, &selected.targets[0], state, context);
}

#[test]
fn logical_original_store_transfer_retains_source_purpose_without_native_normal() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    with_store_fixture(|tokens, target, state, context| {
        let outcomes = transfer(tokens.words(), target, 0, &mut state.clone(), context).unwrap();
        assert!(outcomes.normal_completion.is_none());
        assert!(outcomes.native_normal_completion.is_none());
        let normal = outcomes.normal.unwrap();
        assert_eq!(normal.original_command_world, state.original_command_world);
        assert_eq!(
            normal
                .source_variables
                .literal_value("suffix", context.registry),
            Some("_hi")
        );
    });
}

#[test]
fn logical_original_store_transfer_refuses_observers_foreign_and_missing_owners() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    with_store_fixture(|tokens, target, state, context| {
        for dynamic in [false, true] {
            let mut observed = state.clone();
            Arc::make_mut(&mut observed.source_variables).dynamic_traces = dynamic;
            Arc::make_mut(&mut observed.source_variables).dynamic_bindings = !dynamic;
            assert!(transfer(tokens.words(), target, 0, &mut observed, context).is_none());
        }
        let mut foreign = state.clone();
        foreign.current_source_origin =
            Some(Arc::new(super::super::SourceOriginId::authored_image(
                tcl_lexer::SourceImage::document("set foreign _hi"),
            )));
        assert!(transfer(tokens.words(), target, 0, &mut foreign, context).is_none());
        let mut config = context.config;
        config.strict_quoting = !config.strict_quoting;
        assert!(
            transfer(
                tokens.words(),
                target,
                0,
                &mut state.clone(),
                SourceExecutionContext { config, ..context }
            )
            .is_none()
        );
        let mut missing = state.clone();
        Arc::make_mut(&mut missing.baseline).logical_source_input = None;
        assert!(transfer(tokens.words(), target, 0, &mut missing, context).is_none());
        let mut foreign_cell = state.clone();
        let mut selected_cell = crate::var_resolve::resolve_literal_access(
            "suffix",
            &state.source_variables,
            false,
            context.registry,
            tcl_registry::TraceOperation::Write,
        );
        selected_cell.cell.as_mut().unwrap().owner =
            crate::place::CellOwner::Namespace("::other".into());
        Arc::make_mut(&mut foreign_cell.source_variables)
            .alias_bindings
            .insert("suffix", selected_cell);
        assert!(transfer(tokens.words(), target, 0, &mut foreign_cell, context).is_none());
        let mut foreign_frame = state.clone();
        foreign_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".into(),
            identity: "other-activation".into(),
        };
        assert!(transfer(tokens.words(), target, 0, &mut foreign_frame, context).is_none());
        for dialect in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
        ] {
            let ingress = tcl_registry::model::ingress::resolve_environment(dialect);
            let profile = ingress.analyser_profile();
            let native = ResolvedAnalysisInput::new(
                profile,
                profile,
                ingress.default_context_registry(),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            let options = SourceAnalysisOptions {
                logical_source_input: Some(&native),
                invocation_dialect: Some(
                    super::super::source_analysis_entry::source_input_dialect(&native),
                ),
                ..SourceAnalysisOptions::default()
            };
            let mut outside = ModuleCommandBindings::initial_with_options(
                context.registry,
                options,
                Some(native.lexer_config()),
            );
            outside.current_source_origin = state.current_source_origin.clone();
            assert!(
                transfer(tokens.words(), target, 0, &mut outside, context).is_none(),
                "{dialect}"
            );
        }
    });
}
