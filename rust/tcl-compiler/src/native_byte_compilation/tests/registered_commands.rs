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

//! Plans for registered byte compilers: shared recipes, registration guards, unsupported recipes and nested hook obligations.

use super::*;

#[test]
fn original_monolithic_match_retains_real_hook_and_registration_guards() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let source = b"string match * $subject";
    let parsed = native_script_words_in(
        SourceImage::native(source.as_slice()),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    let plan =
        native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context).unwrap();
    let NativeByteCommandPlan::Registered(selected) = plan else {
        panic!("original monolithic StringMatch compiler");
    };
    assert!(matches!(
        selected.spec.grammar,
        NativeCompilationGrammar::StringMatch(
            tcl_registry::native_string_compilation::NativeStringMatchScope::PublicMember
        )
    ));
    assert_eq!(
        selected.binding.compiler_hook,
        NativeCompilerHookPresence::Present
    );
    let original = entry
        .lookup_command_bytes(entry.current_namespace, b"string")
        .unwrap()
        .unwrap();
    assert_eq!(selected.binding.token, original.token);
    assert_eq!(selected.binding.compiler, original.compiler);
    for presence in [
        NativeCompilerHookPresence::Absent,
        NativeCompilerHookPresence::Unknown,
    ] {
        let mut changed = entry.clone();
        changed
            .commands
            .iter_mut()
            .find(|binding| binding.token == original.token)
            .unwrap()
            .compiler_hook = presence;
        let plan =
            native_byte_command_plan(&parsed.commands[0].words, &changed, &registry, context);
        if presence == NativeCompilerHookPresence::Absent {
            assert_eq!(plan.unwrap(), NativeByteCommandPlan::Generic);
        } else {
            assert!(plan.is_err());
        }
    }
}

#[test]
fn registered_worker_recipe_preserves_purpose_and_rejects_changed_original_basis() {
    use tcl_registry::native_compilation::NativeCompilationFrame;
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let entry: crate::environment_ingress::CapturedNativeEntry =
            crate::environment_ingress::captured_native_entry(profile);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, frame) in [
            (
                b"array set event_handlers {}".as_slice(),
                NativeCompilationFrame::ScriptCode,
            ),
            (
                b"array set event_handlers {name handler}".as_slice(),
                NativeCompilationFrame::ProcedureCode,
            ),
            (
                b"array exists event_handlers".as_slice(),
                NativeCompilationFrame::ScriptCode,
            ),
        ] {
            let parsed = native_script_words_in(
                SourceImage::native(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let words = &parsed.commands[0].words;
            let context = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame,
                ..Default::default()
            };
            let NativeByteCommandPlan::Registered(plan) =
                native_byte_command_plan(words, &entry, &registry, context).unwrap()
            else {
                panic!("{version:?}: actual Array worker compiler");
            };
            assert!(
                plan.prerequisite
                    .as_ref()
                    .unwrap()
                    .selected_worker
                    .is_some()
            );
            assert!(matches!(
                plan.original_instruction(words),
                Some(NativeInstructionPlan::Array(_))
            ));
            assert!(registered_operand_available(words, &entry, &plan));
            let changed_source = source.iter().copied().chain(*b" ").collect::<Vec<_>>();
            let changed = native_script_words_in(
                SourceImage::native(changed_source.as_slice()),
                Span::new(0, u32::try_from(changed_source.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            assert!(
                plan.original_instruction(&changed.commands[0].words)
                    .is_none()
            );
            assert!(!registered_operand_available(
                &changed.commands[0].words,
                &entry,
                &plan
            ));
            let mut changed_basis = (*plan).clone();
            changed_basis.operand_from += 1;
            assert!(changed_basis.original_instruction(words).is_none());
            changed_basis = (*plan).clone();
            changed_basis.context.frame = NativeCompilationFrame::Unknown;
            assert!(changed_basis.original_instruction(words).is_none());
        }
    }
}

#[test]
fn registered_concat_uses_shared_recipe_and_original_worker_authority() {
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let mut entry: crate::environment_ingress::CapturedNativeEntry =
            crate::environment_ingress::captured_native_entry(profile);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for source in [
            b"concat { A } { B }".as_slice(),
            b"concat $a $b",
            b"concat {*}{A B} $c",
        ] {
            let parsed = native_script_words_in(
                SourceImage::native(source),
                tcl_lexer::Span::new(
                    0,
                    u32::try_from(source.len()).expect("fixture source fits Span"),
                ),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            if version == tcl_dialect::TclVersion::V8_4 && source == b"concat {*}{A B} $c" {
                // The original C8.4 parser rejects the adjacent brace
                // groups before a compiler receives a command vector.
                assert!(parsed.fatal_tail.is_some());
                assert_eq!(
                    parsed.commands,
                    [] as [tcl_lexer::NativeScriptCommandWords; 0]
                );
                continue;
            }
            let plan =
                native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context)
                    .unwrap();
            if version < tcl_dialect::TclVersion::V8_6 {
                assert_eq!(plan, NativeByteCommandPlan::Generic);
            } else {
                assert!(
                    matches!(plan, NativeByteCommandPlan::Registered(_)),
                    "{version:?}: {source:?}"
                );
            }
        }
        let source = b"concat $a $b";
        let parsed = native_script_words_in(
            SourceImage::native(source.as_slice()),
            tcl_lexer::Span::new(
                0,
                u32::try_from(source.len()).expect("fixture source fits Span"),
            ),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let target = entry
            .commands
            .iter_mut()
            .find(|binding| binding.slot.simple.as_bytes() == b"concat")
            .unwrap();
        target.implementation_generation += 1;
        target.implementation = NativeCommandImplementation::Opaque;
        target.compiler_hook = NativeCompilerHookPresence::Absent;
        target.compiler = None;
        assert_eq!(
            native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
        target_unknown(&mut entry, b"concat");
        assert_eq!(
            native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context),
            Err(NativeByteCommandUnavailable::Registration)
        );
    }
}

fn target_unknown(entry: &mut NativeCompilationEntry, name: &[u8]) {
    entry
        .commands
        .iter_mut()
        .find(|binding| binding.slot.simple.as_bytes() == name)
        .unwrap()
        .compiler_hook = NativeCompilerHookPresence::Unknown;
}

#[test]
fn registered_recipe_without_shared_instruction_remains_unavailable() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let source = b"llength $items";
    let parsed = native_script_words_in(
        SourceImage::native(source.as_slice()),
        tcl_lexer::Span::new(
            0,
            u32::try_from(source.len()).expect("fixture source fits Span"),
        ),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    assert_eq!(
        native_byte_command_plan(
            &parsed.commands[0].words,
            &entry,
            &registry,
            NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                ..Default::default()
            }
        ),
        Err(NativeByteCommandUnavailable::Recipe)
    );
}

#[test]
fn byte_compiler_plan_retains_original_registration_for_opaque_ordinary_operands() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for (identity, source) in [
        ("set", b"renamed\xff x\xff\0tail VALUE".as_slice()),
        ("incr", b"renamed\xff x\xff\0tail 1".as_slice()),
        ("list", b"renamed\xff A\xff\0tail B".as_slice()),
        ("append", b"renamed\xff x\xff\0tail $value".as_slice()),
        ("lappend", b"renamed\xff x\xff\0tail $value".as_slice()),
        ("return", b"renamed\xff $value".as_slice()),
    ] {
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        let mut registration = binding(
            &entry.namespaces[0],
            b"renamed\xff",
            NativeCompilerHookPresence::Present,
        );
        registration.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: identity.into(),
            ensemble: None,
        });
        entry.commands.push(registration.clone());
        let module = source_module(source, &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("retained original word vector");
        };
        let plan = native_byte_command_plan(words, &entry, &registry, context).unwrap();
        let NativeByteCommandPlan::Registered(plan) = plan else {
            panic!("authenticated original compiler");
        };
        assert_eq!(plan.binding, registration);
        assert_eq!(plan.operand_from, 1);
        assert_eq!(
            plan.dependencies,
            [] as [tcl_runtime_api::native_compilation::NativeCompilationBinding; 0]
        );
        assert!(matches!(
            plan.selection,
            NativeCompilationSelection::Inline { .. }
        ));
        assert!(!closed_generic_byte_compilation(&module, &entry));
        assert!(closed_original_byte_compilation(
            &module, &entry, &registry, context
        ));
        entry.commands[0].compiler = None;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Err(NativeByteCommandUnavailable::Registration)
        );
        assert!(!closed_original_byte_compilation(
            &module, &entry, &registry, context
        ));
        entry.commands[0].compiler_hook = NativeCompilerHookPresence::Unknown;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Err(NativeByteCommandUnavailable::Registration)
        );
        entry.source_string_protocol = None;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Err(NativeByteCommandUnavailable::Entry)
        );
    }
}

#[test]
fn byte_compiler_plan_refuses_unsupported_registered_recipes_and_nested_hooks() {
    let registry = CommandRegistry::build_default();
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut registration = binding(
        &entry.namespaces[0],
        b"worker",
        NativeCompilerHookPresence::Present,
    );
    registration.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "expr".into(),
        ensemble: None,
    });
    entry.commands.push(registration);
    let module = source_module(b"worker $x", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
        panic!("original words");
    };
    assert_eq!(
        native_byte_command_plan(
            words,
            &entry,
            &registry,
            NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..Default::default()
            }
        ),
        Err(NativeByteCommandUnavailable::Recipe)
    );
    assert!(!closed_generic_byte_compilation(
        &source_module(b"opaque\xff $a([worker])", &entry),
        &entry
    ));
    entry.source_string_protocol = None;
    assert!(!closed_generic_byte_compilation(
        &source_module(b"opaque\xff", &entry),
        &entry
    ));
}

#[test]
fn registered_certificate_keeps_nested_original_hook_obligations() {
    let registry = CommandRegistry::build_default();
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut registration = binding(
        &entry.namespaces[0],
        b"set",
        NativeCompilerHookPresence::Present,
    );
    registration.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "set".into(),
        ensemble: None,
    });
    entry.commands.push(registration);
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"child",
        NativeCompilerHookPresence::Present,
    ));
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let module = source_module(b"set a\xff([child]) VALUE", &entry);
    assert!(!closed_original_byte_compilation(
        &module, &entry, &registry, context
    ));
    entry.commands[1].compiler_hook = NativeCompilerHookPresence::Absent;
    assert!(closed_original_byte_compilation(
        &module, &entry, &registry, context
    ));
    let malformed = source_module(b"set a\xff([child {a}b]) VALUE", &entry);
    assert!(!closed_original_byte_compilation(
        &malformed, &entry, &registry, context
    ));
    entry.source_string_protocol = None;
    assert!(!closed_original_byte_compilation(
        &module, &entry, &registry, context
    ));
}
