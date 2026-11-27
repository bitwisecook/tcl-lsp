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

use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationFrame};

fn analyse(
    source: &str,
    version: tcl_dialect::TclVersion,
    mode: NativeCompilationMode,
) -> SourceCommandBindings {
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        &tcl_registry::CommandRegistry::build_default(),
        SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: NativeCompilationContext {
                catch_depth: Some(0),
                mode,
                frame: NativeCompilationFrame::ScriptCode,
                loop_depth: 0,
            },
            ..SourceAnalysisOptions::default()
        },
    )
}

#[test]
fn original_literal_compiler_name_effects_remain_separate_from_unknown_admission() {
    // Implementation contract: naming.compiler.original-name-effect-separation (docs/design/analysis/name-resolution-proofs/original-compiler-name-effect-separation.md).
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let options = SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::Unknown,
                frame: NativeCompilationFrame::Unknown,
                catch_depth: None,
                loop_depth: 0,
            },
            ..SourceAnalysisOptions::default()
        };
        let analysis = SourceCommandBindings::analyse_with_options(
            "set checkpoint READY",
            config,
            &registry,
            options,
        );
        let point = analysis.points.iter().find(|point| point.dispatch).unwrap();
        let selected = &point.compiled_execution;
        assert!(selected.unknown);
        assert!(selected.compiler_preserves_names());
        let source = "set checkpoint READY";
        let segments =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segments[0],
        );
        analysis.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.original_compiler_names_preserved_for_tokens(&tokens));
        let mut changed = tokens.clone();
        let tcl_source::words::WordExpr::Literal { text, .. } = &mut changed.word_exprs[2] else {
            panic!("actual original literal");
        };
        *text = "foreign".into();
        assert!(!binding.original_compiler_names_preserved_for_tokens(&changed));
        assert_eq!(
            selected.admission,
            Some(NativeCompilationSelection::Unknown)
        );
        assert_eq!(
            selected.proofs,
            [] as [crate::command_binding::compiled_invocation::SourceCompiledInvocationProof; 0]
        );
        assert!(selected.admitted.is_none());
        assert_eq!(
            selected
                .stable_handler_after_original_arguments(
                    point.original_head_input.as_ref(),
                    &point.state,
                    &point.namespace_key
                )
                .unwrap()
                .command,
            "::set"
        );

        let nested = SourceCommandBindings::analyse_with_options(
            "set checkpoint [expr {rand()}]",
            config,
            &registry,
            options,
        );
        let nested_point = nested
            .points
            .iter()
            .find(|point| point.dispatch && point.offset == 0)
            .unwrap();
        assert!(!nested_point.compiled_execution.compiler_preserves_names());
        let nested_source = "set checkpoint [expr {rand()}]";
        let nested_segments =
            tcl_source::segment::segment_commands_with_offset_and_config(nested_source, 0, config);
        let mut nested_tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(nested_source),
            config,
            &nested_segments[0],
        );
        nested.stamp_original_tokens(&mut nested_tokens);
        let mut joined_binding = binding.clone();
        joined_binding.join(nested_tokens.source_binding.as_ref().unwrap());
        assert!(!joined_binding.original_compiler_names_preserved_for_tokens(&tokens));
        let mut joined = selected.clone();
        joined.join(&nested_point.compiled_execution);
        assert!(!joined.compiler_preserves_names());
        assert!(joined.unknown);
    }
}

#[test]
fn original_nested_compiler_effects_preserve_names_without_executing_children() {
    // Implementation contract: naming.compiler.original-preparation-name-effects (docs/design/analysis/name-resolution-proofs/original-preparation-name-effects.md).
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let options = SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: NativeCompilationContext::default(),
            ..Default::default()
        };
        for (source, closed) in [
            ("if {1} {package ifneeded p 1.0 {source p.tcl}}", true),
            ("if {0} {set x [expr {rand()}]}", true),
            ("if {1} {set x [expr {rand()}]}", false),
            ("set checkpoint [proc q {} {}]", true),
            ("if {$condition} {set x 1} else {set x 2}", true),
            ("expr {[expr {$x * 2}]}", true),
            ("expr {[expr {rand()}]}", false),
            ("expr {\"[expr {$x * 2}]\"}", true),
            ("expr {$a([expr {$x * 2}])}", true),
        ] {
            let analysis =
                SourceCommandBindings::analyse_with_options(source, config, &registry, options);
            let point = analysis
                .points
                .iter()
                .find(|point| point.dispatch && point.offset == 0)
                .unwrap();
            assert_eq!(
                point.compiled_execution.compiler_preserves_names(),
                closed,
                "{version:?}: {source}"
            );
            assert!(point.compiled_execution.unknown);
            assert_eq!(
                point.compiled_execution.admission,
                Some(NativeCompilationSelection::Unknown)
            );
            assert!(point.compiled_execution.admitted.is_none());
            assert_eq!(
                point.compiled_execution.proofs,
                [] as [crate::command_binding::compiled_invocation::SourceCompiledInvocationProof;
                    0]
            );
        }
    }
}

fn assert_native_fixed_lookup_resolver_scope(
    profile: &'static tcl_dialect::DialectProfile,
    registry: &tcl_registry::CommandRegistry,
    lookup: &tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    version: tcl_dialect::TclVersion,
) {
    use tcl_runtime_api::native_compilation::{
        NativeCommandResolverInventory, NativeCommandResolverPresence,
    };
    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
    let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let native_policy = entry.command_name_policy().unwrap();
    let native = ModuleCommandBindings::initial_with_options(
        registry,
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(dialect),
            ..Default::default()
        },
        Some(tcl_lexer::LexerConfig::from_grammar(profile.grammar)),
    );
    assert!(
        original_fixed_compiler_lookup_preserves_names(&native, lookup, Some(native_policy)),
        "{version:?}"
    );
    assert!(!original_fixed_compiler_lookup_preserves_names(
        &native,
        lookup,
        Some(policy)
    ));
    let resolver = |interpreter, epoch, presence| {
        Some(NativeCommandResolverInventory::captured(
            interpreter,
            epoch,
            presence,
        ))
    };
    let mut foreign = entry.interpreter;
    foreign.owner += 1;
    for inventory in [
        None,
        resolver(
            entry.interpreter,
            entry.epoch,
            NativeCommandResolverPresence::Present,
        ),
        resolver(
            entry.interpreter,
            entry.epoch,
            NativeCommandResolverPresence::Unknown,
        ),
        resolver(
            entry.interpreter,
            entry.epoch + 1,
            NativeCommandResolverPresence::Absent,
        ),
        resolver(foreign, entry.epoch, NativeCommandResolverPresence::Absent),
    ] {
        let mut changed = native.clone();
        Arc::make_mut(
            Arc::make_mut(&mut changed.baseline)
                .native_entry
                .as_mut()
                .unwrap(),
        )
        .command_resolvers = inventory;
        assert!(!original_fixed_compiler_lookup_preserves_names(
            &changed,
            lookup,
            Some(native_policy)
        ));
    }
}

#[test]
fn original_fixed_compiler_lookup_requires_current_authored_helper_and_resolver_scope() {
    // Implementation contract: naming.compiler.fixed-helper-name-effects (docs/design/analysis/name-resolution-proofs/fixed-helper-name-effects.md).
    // Implementation proof: naming.compiler.native-command-resolver-inventory
    // docs/design/analysis/name-resolution-proofs/native-command-resolver-inventory.md
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let options = SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            ..Default::default()
        };
        let state = ModuleCommandBindings::initial_with_options(
            &registry,
            options,
            Some(tcl_lexer::LexerConfig::from_grammar(profile.grammar)),
        );
        let spec = registry
            .native_compilation_for_registration("::tcl::namespace::exists", dialect)
            .unwrap();
        let lookup = spec
            .implementation_prerequisites(dialect)
            .unwrap()
            .pop()
            .unwrap();
        let policy = dialect.authored_name_policy().unwrap();
        assert!(
            original_fixed_compiler_lookup_preserves_names(&state, &lookup, Some(policy)),
            "{version:?}"
        );
        assert!(!original_fixed_compiler_lookup_preserves_names(
            &state, &lookup, None
        ));
        let root = state.source_root_namespace_key().unwrap();
        let key = state
            .original_registry_command_paths(&root, lookup.slot, policy)
            .unwrap()[0][0]
            .clone();
        for alternative in [MayBinding::Missing, MayBinding::Unknown] {
            let mut changed = state.clone();
            changed.replace(key.clone(), BTreeSet::from([alternative]));
            assert!(!original_fixed_compiler_lookup_preserves_names(
                &changed,
                &lookup,
                Some(policy)
            ));
        }
        let mut target = state
            .original_registry_metadata_target(&root, lookup.slot, policy)
            .unwrap();
        target.kind = BindingKind::Alias;
        let mut wrapper = state.clone();
        wrapper.replace(key, BTreeSet::from([MayBinding::Target(target)]));
        assert!(!original_fixed_compiler_lookup_preserves_names(
            &wrapper,
            &lookup,
            Some(policy)
        ));
        let mut open = state.clone();
        Arc::make_mut(&mut open.baseline).unknown_entry = true;
        assert!(!original_fixed_compiler_lookup_preserves_names(
            &open,
            &lookup,
            Some(policy)
        ));
        assert_native_fixed_lookup_resolver_scope(profile, &registry, &lookup, policy, version);
    }
}

#[test]
fn original_null_namespace_compiler_does_not_borrow_worker_admission() {
    // Implementation contract: naming.compiler.fixed-helper-name-effects (docs/design/analysis/name-resolution-proofs/fixed-helper-name-effects.md).
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let analysis = SourceCommandBindings::analyse_with_options(
            "namespace eval A {proc p {} {}}",
            config,
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::Unknown,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let point = analysis
            .points
            .iter()
            .find(|point| point.dispatch && point.offset == 0)
            .unwrap();
        assert!(
            point.compiled_execution.compiler_preserves_names(),
            "{name}"
        );
        assert_eq!(
            point.compiled_execution.proofs,
            [] as [crate::command_binding::compiled_invocation::SourceCompiledInvocationProof; 0]
        );
        assert!(point.compiled_execution.admitted.is_none());
    }
}

#[test]
fn nested_generic_pid_keeps_the_original_list_length_admission() {
    let source = "set result [llength [pid]]";
    let analysis = analyse(
        source,
        tcl_dialect::TclVersion::V9_0,
        NativeCompilationMode::BytecodeObject,
    );
    for (head, offset) in [("set", 0), ("llength", 12), ("pid", 21)] {
        let binding = analysis.invocation_at_source(head, offset);
        assert_ne!(
            binding.native_compilation_admission_selection(),
            NativeCompilationSelection::Unknown,
            "{head}: runtime_unknown={} candidates={} reads_dynamic={}",
            binding.unknown,
            binding.compiled_candidates.len(),
            binding.variable_context.dynamic_bindings
        );
    }
    let segments = tcl_source::segment::segment_commands(source);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        tcl_lexer::LexerConfig::default(),
        &segments[0],
    );
    analysis.stamp_original_tokens(&mut tokens);
    let nested = tokens
        .nested_bindings
        .iter()
        .find(|(offset, _)| *offset == 12)
        .expect("exact original llength carrier");
    assert!(nested.1.admitted_inline_invocation().is_some());
}

#[test]
fn direct_procedure_slot_proof_keeps_alias_wrappers_distinct() {
    for (source, head, direct) in [
        ("proc p {} {return 1}; p", "p", true),
        (
            "proc p {} {return 1}; set target p; $target",
            "$target",
            true,
        ),
        (
            "proc p {} {return 1}; interp alias {} a {} p; set target a; $target",
            "$target",
            false,
        ),
        (
            "proc p {} {return 1}; interp alias {} a {} p; a",
            "a",
            false,
        ),
        (
            "proc p {} {return 1}; interp alias {} a {} p 7; a",
            "a",
            false,
        ),
    ] {
        let analysis = analyse(
            source,
            tcl_dialect::TclVersion::V9_1,
            NativeCompilationMode::Direct,
        );
        let offset = u32::try_from(source.rfind(head).unwrap()).unwrap();
        let binding = analysis.invocation_at_source(head, offset);
        assert_eq!(
            binding.called_slot_kind(),
            Some(if direct {
                BindingKind::Proc
            } else {
                BindingKind::Alias
            }),
            "{source}"
        );
        assert_eq!(
            binding.direct_procedure_target(head).is_some(),
            direct,
            "{source}"
        );
    }
}

#[test]
fn direct_registry_slot_proof_uses_captured_heads_and_rejects_wrappers() {
    for (source, head, direct) in [
        ("puts hi", "puts", true),
        ("set target puts; $target hi", "$target", true),
        ("namespace eval N {puts hi}", "puts", true),
        ("interp alias {} a {} puts; a hi", "a", false),
        (
            "interp alias {} a {} puts; set target a; $target hi",
            "$target",
            false,
        ),
    ] {
        let analysis = analyse(
            source,
            tcl_dialect::TclVersion::V9_1,
            NativeCompilationMode::Direct,
        );
        let offset = u32::try_from(source.rfind(head).unwrap()).unwrap();
        let binding = analysis.invocation_at_source(head, offset);
        assert_eq!(
            binding.called_slot_kind(),
            Some(if direct {
                BindingKind::Builtin
            } else {
                BindingKind::Alias
            }),
            "{source}"
        );
        assert_eq!(
            binding.direct_registry_target(head).is_some(),
            direct,
            "{source}"
        );
    }
}

#[test]
fn qualified_increment_preserves_the_next_procedure_command() {
    let source = "set ::count 0; proc operand {} {incr ::count; return x}; operand";
    let analysis = analyse(
        source,
        tcl_dialect::TclVersion::V8_5,
        NativeCompilationMode::BytecodeObject,
    );
    let offset = u32::try_from(source.find("return x").unwrap()).unwrap();
    let binding = analysis.invocation_at_source("return", offset);
    assert!(
        binding.proved_execution_target().is_some(),
        "return unknown={} absent={} dynamic_bindings={} dynamic_traces={} compiler={:?}",
        binding.unknown,
        binding.may_be_absent,
        binding.variable_context.dynamic_bindings,
        binding.variable_context.dynamic_traces,
        binding.native_compilation_selection()
    );
}

#[test]
fn string_equal_public_replacement_follows_its_original_compiler_shape() {
    // Implementation contract: naming.compiler.original-compiler-operand-coordinates
    // docs/design/analysis/name-resolution-proofs/compiler-original-operand-coordinates.md
    // The native scalar argv windows independently establish member shape;
    // this source-model control checks the replacement horizon, not a native run.
    for version in tcl_dialect::TclVersion::ALL {
        for member in ["equal", "eq", "\"equal\"", "{equal}", "equal -nocase"] {
            let source =
                format!("string {member} a [proc string {{args}} {{return CUSTOM}};set v a]");
            let analysis = analyse(&source, version, NativeCompilationMode::BytecodeObject);
            let binding = analysis.invocation_at_source("string", 0);
            let generic = if member == "equal -nocase" {
                version < tcl_dialect::TclVersion::V8_6
            } else {
                version == tcl_dialect::TclVersion::V8_4
                    && matches!(member, "\"equal\"" | "{equal}")
            };
            assert_eq!(
                binding.native_compilation_selection() == NativeCompilationSelection::Generic,
                generic,
                "{version:?}: {member}",
            );
            assert_eq!(
                binding
                    .proved_execution_target()
                    .is_some_and(|target| target.registry_backed),
                !generic,
                "{version:?}: {member}",
            );
        }
    }
}

#[test]
fn public_info_replacement_during_argv_preserves_selected_exists_operation() {
    let source = "info exists [proc info {args} {return CUSTOM}]";
    for version in tcl_dialect::TclVersion::ALL {
        let analysis = analyse(source, version, NativeCompilationMode::BytecodeObject);
        let binding = analysis.invocation_at_source("info", 0);
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(
                binding.compiled_candidates,
                [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
            );
            assert!(
                binding
                    .proved_execution_target()
                    .is_some_and(|target| !target.registry_backed)
            );
        } else {
            let [proof] = binding.compiled_candidates.as_slice() else {
                panic!("expected selected original info exists on {version:?}");
            };
            assert_eq!(
                proof.operation,
                tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::InfoExists)
            );
            assert_eq!(proof.certainty, CompiledExecutionCertainty::Must);
            assert!(
                binding
                    .proved_execution_target()
                    .is_some_and(|target| target.registry_backed)
            );
            assert!(
                binding
                    .proved_target()
                    .is_some_and(|target| !target.registry_backed)
            );
        }
    }
}

#[test]
fn literal_list_operands_retain_the_actual_native_compiler_target() {
    for version in tcl_dialect::TclVersion::ALL {
        for (source, head, inline) in [
            (
                "lrange [proc lrange args {return CUSTOM}; set v {a b}] 0 end",
                "lrange",
                version >= tcl_dialect::TclVersion::V8_6,
            ),
            (
                "lassign [proc lassign args {return CUSTOM}; set v {a b}] x",
                "lassign",
                version >= tcl_dialect::TclVersion::V8_5,
            ),
        ] {
            let analysis = analyse(source, version, NativeCompilationMode::BytecodeObject);
            let binding = analysis.invocation_at_source(head, 0);
            assert_eq!(
                matches!(
                    binding.native_compilation_selection(),
                    NativeCompilationSelection::Inline { .. }
                ),
                inline,
                "{version:?}: {head}"
            );
            assert_eq!(
                binding
                    .proved_execution_target()
                    .is_some_and(|target| target.registry_backed),
                inline,
                "{version:?}: {head}"
            );
        }
    }
}

#[test]
fn escaped_head_compilation_uses_the_native_release_boundary() {
    let source = r"se\x74 x [rename set saved;proc set {args} {return CUSTOM};saved value 7]";
    for version in tcl_dialect::TclVersion::ALL {
        let analysis = analyse(source, version, NativeCompilationMode::BytecodeObject);
        let binding = analysis.invocation_at_source("set", 0);
        if version < tcl_dialect::TclVersion::V8_6 {
            assert_eq!(
                binding.compiled_candidates,
                [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
            );
            assert!(
                binding
                    .proved_execution_target()
                    .is_some_and(|target| !target.registry_backed)
            );
        } else {
            assert!(
                binding
                    .proved_execution_target()
                    .is_some_and(|target| target.registry_backed && target.command == "::set")
            );
            assert!(
                binding
                    .proved_target()
                    .is_some_and(|target| !target.registry_backed)
            );
        }
    }
}

#[test]
fn ensemble_compiler_selection_requires_its_original_private_implementation() {
    let source = "proc ::tcl::info::exists {args} {return PRIVATE}; info exists x";
    let analysis = analyse(
        source,
        tcl_dialect::TclVersion::V8_6,
        NativeCompilationMode::BytecodeObject,
    );
    let offset = u32::try_from(source.rfind("info exists").unwrap()).unwrap();
    let binding = analysis.invocation_at_source("info", offset);
    assert_eq!(
        binding.compiled_candidates,
        [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
    );
    let lookup = tcl_registry::native_compilation::NativeCompilationSpec {
        grammar: tcl_registry::native_compilation::NativeCompilationGrammar::InfoExists,
        operation: tcl_registry::SemanticOperationId::Intrinsic(
            tcl_registry::IntrinsicId::InfoExists,
        ),
        body: tcl_registry::native_compilation::NativeBodyCompilation::Inherit,
    }
    .implementation_lookup(tcl_registry::InvocationDialect::for_version(
        tcl_dialect::TclVersion::V8_6,
    ))
    .unwrap();
    assert!(!binding.proves_native_implementation_lookup(&lookup));
}

#[test]
fn default_execution_retains_live_dispatch() {
    assert!(SourceInvocationBinding::default().may_use_live_dispatch);
}

#[test]
fn unknown_compiler_protocol_can_retain_handler_identity_without_an_opcode() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let analysis = SourceCommandBindings::analyse_with_options(
        "set x 1",
        tcl_lexer::LexerConfig::for_dialect("f5-irules"),
        registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                registry.profile().expect("selected F5 profile"),
            )),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                catch_depth: Some(0),
                loop_depth: 0,
            },
            ..SourceAnalysisOptions::default()
        },
    );
    let point = analysis.points.iter().find(|point| point.dispatch).unwrap();
    let selection = &point.compiled_execution;
    assert!(selection.unknown);
    assert_eq!(
        selection.proofs,
        [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
    );
    let target = selection
        .stable_handler_after_original_arguments(
            point.original_head_input.as_ref(),
            &point.state,
            &point.namespace_key,
        )
        .expect("unchanged native handler under uncertain compilation");
    assert_eq!(target.command, "::set");
    assert!(target.registry_backed);
    let proof = analysis.invocation_at_source("set", 0);
    assert!(proof.compiled_execution_unknown());
    assert!(proof.proved_execution_target().is_none());
    let mut joined = selection.clone();
    joined.join(&CompiledInvocationSelection::default());
    assert!(joined.stable_handler.is_none());
}

#[test]
fn unknown_compiler_keeps_possible_body_inventory_and_may_stores() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let source = "if {[IP::client_addr] eq {x}} {set flag 1}; list final";
    let analysis = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::for_dialect("f5-irules"),
        registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                registry.profile().unwrap(),
            )),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                catch_depth: Some(0),
                loop_depth: 0,
            },
            ..SourceAnalysisOptions::default()
        },
    );
    let body = u32::try_from(source.find("set flag").unwrap()).unwrap();
    let binding = analysis.invocation_at_source("set", body);
    assert!(
        binding
            .proved_handler_target()
            .is_some_and(|target| target.registry_backed)
    );
    assert!(binding.proved_execution_target().is_none());
    let flag = crate::var_resolve::canonical_literal_variable_key(
        "flag",
        &binding.variable_context,
        registry,
    )
    .expect("original setter operand selects its own namespace cell");
    let after = u32::try_from(source.find("list final").unwrap()).unwrap();
    let binding = analysis.invocation_at_source("list", after);
    assert!(binding.proved_handler_target().is_some());
    assert!(!binding.existing_namespace_cells.contains(&flag));
}

#[test]
fn native_selection_precedes_mutations_in_argv() {
    let source = "set x [proc set {args} {return CUSTOM}]; puts finished";
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let analysis = analyse(source, version, NativeCompilationMode::BytecodeObject);
        let proof = analysis.invocation_at_source("set", 0);
        assert!(
            proof
                .proved_target()
                .is_some_and(|target| !target.registry_backed),
            "live: {proof:#?}"
        );
        assert!(
            proof
                .proved_execution_target()
                .is_some_and(|target| target.registry_backed),
            "execution: {proof:#?}"
        );
        assert!(!proof.may_use_live_dispatch);
        let later = analysis
            .invocation_at_source("puts", u32::try_from(source.find("puts").unwrap()).unwrap());
        assert!(
            later.namespace_cell_presence.present.contains("::x"),
            "{later:#?}"
        );
    }
}

#[test]
fn direct_entry_uses_late_lookup_after_argv() {
    let source = "set x [proc set {args} {return CUSTOM}]";
    let analysis = analyse(
        source,
        tcl_dialect::TclVersion::V8_6,
        NativeCompilationMode::Direct,
    );
    let proof = analysis.invocation_at_source("set", 0);
    assert_eq!(
        proof.compiled_candidates,
        [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
    );
    assert!(
        proof
            .proved_execution_target()
            .is_some_and(|target| !target.registry_backed)
    );
}

#[test]
fn legacy_chunk_entry_and_modern_before_argv_guards_differ() {
    let source = "proc set {args} {return CUSTOM}; set x 1";
    let offset = u32::try_from(source.find("set x").unwrap()).unwrap();
    let legacy = analyse(
        source,
        tcl_dialect::TclVersion::V8_4,
        NativeCompilationMode::BytecodeObject,
    )
    .invocation_at_source("set", offset);
    assert!(
        legacy
            .proved_execution_target()
            .is_some_and(|target| target.registry_backed),
        "{legacy:#?}"
    );
    let modern = analyse(
        source,
        tcl_dialect::TclVersion::V8_6,
        NativeCompilationMode::BytecodeObject,
    )
    .invocation_at_source("set", offset);
    assert!(
        modern
            .proved_execution_target()
            .is_some_and(|target| !target.registry_backed),
        "{modern:#?}"
    );
}

#[test]
fn computed_head_does_not_acquire_a_compiler_hook_from_its_value() {
    let source = "set c set; $c x [proc set {args} {return CUSTOM}]";
    let offset = u32::try_from(source.find("$c x").unwrap()).unwrap();
    let proof = analyse(
        source,
        tcl_dialect::TclVersion::V8_6,
        NativeCompilationMode::BytecodeObject,
    )
    .invocation_at_source("$c", offset);
    assert_eq!(
        proof.compiled_candidates,
        [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
    );
    assert!(proof.may_use_live_dispatch);
    assert!(
        proof
            .proved_execution_target()
            .is_some_and(|target| !target.registry_backed),
        "{proof:#?}"
    );
}

#[test]
fn original_chunk_compiler_selection_survives_unknown_runtime_occupancy() {
    // Implementation proof: naming.compiler.chunk-selection-runtime-occupancy
    // docs/design/analysis/name-resolution-proofs/chunk-selection-runtime-occupancy.md
    let source = "proc opaque {} {unknown}; opaque; set x VALUE";
    let offset = u32::try_from(source.find("set x").unwrap()).unwrap();
    for version in tcl_dialect::TclVersion::ALL {
        let analysis = analyse(source, version, NativeCompilationMode::BytecodeObject);
        let point = analysis
            .points
            .iter()
            .find(|point| point.dispatch && point.offset == offset)
            .unwrap();
        assert!(point.state.has_opaque_domain(), "{version:?}");
        assert!(
            point.compiled_execution.stable_handler.is_none(),
            "{version:?}"
        );
        assert_ne!(
            point.compiled_execution.admission,
            Some(NativeCompilationSelection::Unknown),
            "{version:?}"
        );
        assert!(
            point.compiled_execution.original_words.is_some(),
            "{version:?}"
        );
        let unknown = analyse(source, version, NativeCompilationMode::Unknown);
        let binding = unknown.invocation_at_source("set", offset);
        assert!(binding.native_compiler_admission.is_none());
        assert_eq!(
            binding.native_compilation_admission_selection(),
            NativeCompilationSelection::Unknown
        );
    }
}

#[test]
fn procedure_compilation_is_distinct_from_direct_file_entry() {
    let source = "proc p {} {set x [proc set {args} {return CUSTOM}]}; p";
    let offset = u32::try_from(source.find("set x").unwrap()).unwrap();
    let proof = analyse(
        source,
        tcl_dialect::TclVersion::V8_6,
        NativeCompilationMode::Direct,
    )
    .invocation_at_source("set", offset);
    assert!(
        proof
            .proved_execution_target()
            .is_some_and(|target| target.registry_backed),
        "{proof:#?}"
    );
}

#[test]
fn compiler_hook_lookup_rejects_name_aliases() {
    let source = "interp alias {} named {} set; named x 1";
    let analysis = analyse(
        source,
        tcl_dialect::TclVersion::V8_6,
        NativeCompilationMode::Direct,
    );
    let point = analysis.invocation_at_source(
        "named",
        u32::try_from(source.find("named x").unwrap()).unwrap(),
    );
    let snapshot = point
        .lookup_state
        .as_ref()
        .unwrap()
        .state
        .native_compilation_snapshot();
    let targets = snapshot.compiler_targets("named", "::");
    assert_eq!(
        targets.targets,
        [] as [crate::command_binding::SourceCommandTarget; 0]
    );
    assert!(targets.may_be_generic);
}
#[test]
fn nested_normal_handler_path_requires_each_live_worker() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let arguments = [
        tcl_registry::InvocationWord::Literal("encode"),
        tcl_registry::InvocationWord::Literal("hex"),
        tcl_registry::InvocationWord::Literal("VALUE"),
    ];
    let words = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal("binary"),
        &arguments,
    )
    .with_dialect(tcl_registry::InvocationDialect::of_profile(profile));
    let facts = super::super::resolve_source_invocation_facts(
        &registry,
        None,
        words,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .unwrap();
    let tcl_registry::native_compilation::NormalHandlerImplementationLookup::RequiredPath(paths) =
        facts.normal_handler_implementation_lookup(words.arguments().dialect())
    else {
        panic!("selected nested normal handler path");
    };
    for (prefix, expected) in [
        ("", true),
        (
            "rename ::tcl::binary::encode::hex saved; proc ::tcl::binary::encode::hex args {return CUSTOM}; ",
            false,
        ),
        (
            "rename ::tcl::binary::encode saved; proc ::tcl::binary::encode args {return CUSTOM}; ",
            false,
        ),
    ] {
        let source = format!("{prefix}binary encode hex VALUE");
        let analysis = analyse(
            &source,
            tcl_dialect::TclVersion::V8_6,
            NativeCompilationMode::Direct,
        );
        let offset = u32::try_from(source.rfind("binary encode").unwrap()).unwrap();
        let binding = analysis.invocation_at_source("binary", offset);
        assert_eq!(
            binding.proves_native_handler_path(paths, words.arguments(), facts.argument_offset),
            expected,
            "{source}"
        );
    }
}
