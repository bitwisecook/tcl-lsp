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
use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

fn logical_unit(source: &str, registry: &tcl_registry::CommandRegistry) -> CompilationUnit {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
    CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    )
}

fn last_invoke(unit: &CompilationUnit) -> &crate::executable_ir::GenericInvoke {
    unit.top_level
        .semantic_facts
        .executable()
        .function()
        .unwrap()
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match instruction {
            ExecutableInstruction::Invoke(invoke) => Some(invoke),
            _ => None,
        })
        .next_back()
        .unwrap()
}

#[test]
fn original_ledger_subject_uses_actual_direct_slot_and_complete_source_vector() {
    // naming.compiler.original-dispatch-ledger-subjects
    // docs/design/analysis/name-resolution-proofs/compiler-original-dispatch-ledger-subjects.md
    // Actual Logical source point; no Native pristine-world inference.
    let registry = tcl_registry::CommandRegistry::build_default();
    let unit = logical_unit("::llength {A B}", &registry);
    let invoke = last_invoke(&unit);
    let InvocationResolution::Resolved(facts) = &invoke.resolution else {
        panic!("selected original builtin");
    };
    let metadata = unit
        .top_level
        .invocation_metadata_context(&registry)
        .unwrap();
    assert_eq!(
        original_ledger_subject(invoke, facts, &registry, Some(metadata)),
        Some("llength".to_owned())
    );
    assert!(original_ledger_subject(invoke, facts, &registry, None).is_none());
    let mut changed = invoke.clone();
    changed.original_words.pop();
    assert!(original_ledger_subject(&changed, facts, &registry, Some(metadata)).is_none());
    let mut missing = invoke.clone();
    missing.source_binding = None;
    assert!(original_ledger_subject(&missing, facts, &registry, Some(metadata)).is_none());
    let mut changed_facts = facts.clone();
    changed_facts.canonical_command = "::llength".to_owned();
    assert!(original_ledger_subject(invoke, &changed_facts, &registry, Some(metadata)).is_none());
    let mut foreign = tcl_registry::CommandRegistry::build_default();
    foreign.insert(tcl_registry::CommandSpec {
        name: "llength",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    assert!(original_ledger_subject(invoke, facts, &foreign, Some(metadata)).is_none());
}

#[test]
fn original_alias_and_moved_slots_cannot_borrow_descriptor_pristine_subjects() {
    // naming.compiler.original-dispatch-ledger-subjects
    // docs/design/analysis/name-resolution-proofs/compiler-original-dispatch-ledger-subjects.md
    // Genuine authored source transitions, not edited binding rows.
    let registry = tcl_registry::CommandRegistry::build_default();
    for source in [
        "interp alias {} selectedAlias {} llength; selectedAlias {A B}",
        "rename llength selectedMove; selectedMove {A B}",
    ] {
        let unit = logical_unit(source, &registry);
        let invoke = last_invoke(&unit);
        let binding = invoke.source_binding.as_ref().unwrap();
        let reference = binding.evaluated_command_reference().unwrap();
        let direct_unmoved_builtin = matches!(reference.binding(),
                crate::command_binding::SourceCommandReferenceBinding::Direct {
                    kind: crate::command_binding::BindingKind::Builtin, identity,
                } if reference.slot() == Some(identity.origin.as_str()));
        assert!(!direct_unmoved_builtin, "{source}");
        let InvocationResolution::Resolved(facts) = &invoke.resolution else {
            panic!("selected original alias or moved builtin");
        };
        let metadata = unit
            .top_level
            .invocation_metadata_context(&registry)
            .unwrap();
        assert!(
            original_ledger_subject(invoke, facts, &registry, Some(metadata)).is_none(),
            "{source}"
        );
    }
}

#[test]
fn held_jim_entry_receives_no_authored_pristine_dispatch_proof() {
    // naming.compiler.original-dispatch-ledger-subjects
    // docs/design/analysis/name-resolution-proofs/compiler-original-dispatch-ledger-subjects.md
    // Genuine software VM entry; no external Jim execution or trace result.
    let profile = tcl_dialect::DialectProfile::find("jimtcl").unwrap();
    let context =
        tcl_registry::model::ingress::resolve_environment("jimtcl").default_context_registry();
    let registry = context.commands();
    let (_owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    assert!(captured.command_name_policy().is_none());
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_context_registry(
        "llength {A B}",
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect(profile.name),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
        registry,
        &unit.ir_module,
    )
    .unwrap();
    assert!(metadata.permits_logical_source_names());
    assert_eq!(
        unit.ir_module
            .original_invocation_source_purpose(registry, metadata),
        Some(crate::command_binding::OriginalInvocationSourcePurpose::Execution)
    );
    let executable = unit
        .top_level
        .semantic_facts
        .executable()
        .function()
        .unwrap();
    let proofs = analyse_original_dispatch_stability(
        executable,
        DispatchEntryAssumption::PristineRegistryWorld,
        registry,
        Some(metadata),
    );
    assert!(proofs.sites.is_empty());
}

#[test]
fn shared_written_ledger_geometry_preserves_trailing_separator_and_literal_colons() {
    // naming.compiler.original-dispatch-ledger-subjects
    // docs/design/analysis/name-resolution-proofs/compiler-original-dispatch-ledger-subjects.md
    // Authored written geometry only; selected descriptor bytes stay data.
    for (written, key) in [
        ("::N::::p", "N::p"),
        ("::N::", "N::"),
        ("::N:::p", "N::p"),
        (":::", ""),
        ("N:p", "N:p"),
    ] {
        assert_eq!(normalise_name(written), key);
    }
    assert_eq!(normalise_name("名字::p"), "名字::p");
    let descriptor = tcl_registry::CommandRegistry::build_default()
        .resolve_invocation("llength", &["A B"], None)
        .unwrap()
        .facts();
    assert_eq!(descriptor.descriptor_key().label(), "llength");
}

#[test]
fn authored_simulation_entry_has_layout_purpose_without_pristine_dispatch_coverage() {
    // naming.compiler.original-dispatch-ledger-subjects
    // docs/design/analysis/name-resolution-proofs/compiler-original-dispatch-ledger-subjects.md
    // Real authored entry policy; no Native interpreter or invocation grant.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let entry = crate::command_binding::SourceAnalysisEntry {
        metadata_context:
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &input,
            )),
        execution_name_policy: Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V9_0),
        )),
        invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_context_registry(
        "llength {A B}",
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
        registry,
        &unit.ir_module,
    )
    .unwrap();
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert_eq!(
        unit.ir_module
            .original_invocation_source_purpose(registry, metadata),
        Some(crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation)
    );
    let executable = unit
        .top_level
        .semantic_facts
        .executable()
        .function()
        .unwrap();
    assert!(
        analyse_original_dispatch_stability(
            executable,
            DispatchEntryAssumption::PristineRegistryWorld,
            registry,
            Some(metadata)
        )
        .sites
        .is_empty()
    );
}
