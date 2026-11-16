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

//! The closed generic byte-compilation certificate: absent hooks, search paths, bracket children and dynamic heads.

use super::*;

#[test]
fn original_generic_selection_precedes_inline_emitter_availability() {
    use tcl_runtime_api::native_compilation::NativeCommandCompiler;
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in tcl_dialect::TclVersion::ALL {
        let mut entry = entry(version);
        let mut target = binding(
            &entry.namespaces[0],
            b"original_worker",
            if version < tcl_dialect::TclVersion::V9_1 {
                NativeCompilerHookPresence::Absent
            } else {
                NativeCompilerHookPresence::Present
            },
        );
        target.compiler = Some(NativeCommandCompiler {
            registry_identity: "uplevel".into(),
            ensemble: None,
        });
        entry.commands.push(target);
        let module = source_module(b"original_worker 1 {error BODY}", &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original words");
        };
        let selected = native_byte_command_plan(words, &entry, &registry, context);
        if version < tcl_dialect::TclVersion::V9_1 {
            assert!(
                matches!(selected, Ok(NativeByteCommandPlan::Generic)),
                "{version:?}"
            );
        } else {
            let NativeByteCommandPlan::Registered(plan) = selected.unwrap() else {
                panic!("C9.1 original Uplevel compiler recipe");
            };
            assert_eq!(plan.binding, entry.commands[0]);
            assert_eq!(plan.context, context);
            assert_eq!(plan.operand_from, 1);
            assert_eq!(
                plan.dependencies,
                [] as [tcl_runtime_api::native_compilation::NativeCompilationBinding; 0]
            );
            assert!(
                plan.prerequisite.is_none(),
                "ordinary compiler has no delegated worker"
            );
            let original =
                NativeCompilerWords::capture(words, entry.source_string_protocol.unwrap()).unwrap();
            assert_eq!(
                tcl_registry::native_instruction_plan::native_instruction_plan(
                    plan.spec,
                    plan.selection,
                    &original,
                    plan.operand_from,
                    InvocationDialect::for_version(version),
                    context,
                )
                .unwrap(),
                tcl_registry::native_instruction_plan::NativeInstructionPlan::Uplevel(
                    tcl_registry::native_instruction_plan::NativeUplevelInstruction {
                        level_word: Some(1),
                        script_words: 2..3,
                    },
                ),
            );
        }
        if version == tcl_dialect::TclVersion::V9_1 {
            let script_context = NativeCompilationContext {
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..context
            };
            assert!(matches!(
                native_byte_command_plan(words, &entry, &registry, script_context),
                Ok(NativeByteCommandPlan::Generic)
            ));
            entry.commands[0].compiler = None;
            assert!(matches!(
                native_byte_command_plan(words, &entry, &registry, context),
                Err(NativeByteCommandUnavailable::Registration)
            ));
        }
    }
}

#[test]
fn original_dynamic_expanded_head_keeps_generic_timing_and_literal_cpp_obligations() {
    // Native proof naming.command-head.literal-expansion-and-inline-replacement:
    // docs/design/analysis/name-resolution-proofs/command-head-literal-expansion-and-inline-replacement.md
    // Native proof naming.command-head.dynamic-expansion-after-replacement:
    // docs/design/analysis/name-resolution-proofs/command-head-dynamic-expansion-after-replacement.md
    // Native proof naming.command-head.empty-dynamic-expansion-exposes-written-head:
    // docs/design/analysis/name-resolution-proofs/command-head-empty-dynamic-expansion-exposes-written-head.md
    // Native six-engine control: tests/data/native_expanded_head/observations.tsv.
    // Modern C uses invokeExpanded for a dynamic head and storeScalar for
    // the literal parser-projected head, including command replacement.
    let observations = include_str!("../../../tests/data/native_expanded_head/observations.tsv");
    assert_eq!(observations.lines().count(), 6);
    assert!(
        observations
            .lines()
            .any(|row| row.starts_with("8.4|0|0|1|"))
    );
    assert!(
        observations
            .lines()
            .any(|row| row.starts_with("jim|0|0|0|A|1|"))
    );
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        let observation = observations
            .lines()
            .find(|row| row.starts_with(&format!("{}|", version.version_string())))
            .unwrap();
        let fields: Vec<_> = observation.split('|').collect();
        assert_eq!(
            &fields[1..3],
            &["1", "1"],
            "dynamic dispatch and literal inline CPP"
        );
        assert_eq!(
            &fields[9..13],
            &[
                "1",
                "{too many nested evaluations (infinite loop?)}",
                "0",
                "A"
            ]
        );
        let entry = entry(version);
        for source in [b"{*}$command VALUE".as_slice(), b"{*}$command set x A"] {
            let module = source_module(source, &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("original expanded command");
            };
            assert_eq!(
                native_byte_command_plan(words, &entry, &registry, context).unwrap(),
                NativeByteCommandPlan::Generic,
                "{version:?}"
            );
            assert!(closed_original_byte_compilation(
                &module, &entry, &registry, context
            ));
            assert!(closed_generic_byte_compilation(&module, &entry));
            let mut missing_point = entry.clone();
            missing_point.execution_point = None;
            assert_eq!(
                native_byte_command_plan(words, &missing_point, &registry, context),
                Err(NativeByteCommandUnavailable::Entry)
            );
            let mut unsupported = entry.clone();
            unsupported.execution_point = Some(tcl_dialect::model::DialectPoint::for_tcl_version(
                tcl_dialect::TclVersion::V8_4,
            ));
            unsupported.name_protocol = tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                unsupported.execution_point.unwrap(),
            );
            assert_eq!(
                native_byte_command_plan(words, &unsupported, &registry, context),
                Err(NativeByteCommandUnavailable::Recipe)
            );
        }
        for source in [b"{*}{set x} A".as_slice(), b"{*}{} set x A"] {
            let module = source_module(source, &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("original projected command");
            };
            assert_eq!(
                native_byte_command_plan(words, &entry, &registry, context),
                Err(NativeByteCommandUnavailable::Recipe)
            );
            assert!(!closed_generic_byte_compilation(&module, &entry));
        }
    }
}

#[test]
fn byte_generic_certificate_requires_original_absent_hook_or_closed_absence() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let module = source_module(b"opaque\xff ARG", &entry);
    assert!(closed_generic_byte_compilation(&module, &entry));
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"opaque\xff",
        NativeCompilerHookPresence::Absent,
    ));
    assert!(closed_generic_byte_compilation(&module, &entry));
    for hook in [
        NativeCompilerHookPresence::Present,
        NativeCompilerHookPresence::Unknown,
    ] {
        entry.commands[0].compiler_hook = hook;
        assert!(!closed_generic_byte_compilation(&module, &entry));
    }
    entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
    entry.closed = false;
    assert!(!closed_generic_byte_compilation(&module, &entry));
    entry.closed = true;
    entry.execution_point = None;
    assert!(!closed_generic_byte_compilation(&module, &entry));
}

#[test]
fn byte_generic_certificate_uses_search_path_and_retained_namespace_incarnations() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    entry.namespaces.push(namespace(
        2,
        ByteNamespacePath::from_segments([b"N".as_slice()]),
        false,
    ));
    entry.namespaces.push(namespace(
        3,
        ByteNamespacePath::from_segments([b"P".as_slice()]),
        true,
    ));
    entry.namespaces[1].command_path = vec![3];
    entry.current_namespace = 2;
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"worker\xff",
        NativeCompilerHookPresence::Absent,
    ));
    entry.commands.push(binding(
        &entry.namespaces[2],
        b"worker\xff",
        NativeCompilerHookPresence::Present,
    ));
    let module = source_module(b"worker\xff", &entry);
    assert!(!closed_generic_byte_compilation(&module, &entry));
    entry.commands[1].compiler_hook = NativeCompilerHookPresence::Absent;
    assert!(closed_generic_byte_compilation(&module, &entry));
    let descendant = source_module(b"child::worker\xff", &entry);
    assert!(!closed_generic_byte_compilation(&descendant, &entry));
    entry
        .namespaces
        .push(namespace(4, entry.namespaces[1].path.clone(), true));
    assert!(!closed_generic_byte_compilation(&descendant, &entry));
    entry.namespaces[1].command_path = vec![99];
    assert!(!closed_generic_byte_compilation(&module, &entry));
}

#[test]
fn byte_generic_certificate_checks_every_original_bracket_child() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"compileMe",
        NativeCompilerHookPresence::Present,
    ));
    for source in [
        b"opaque\xff [compileMe]".as_slice(),
        b"$head [absent [compileMe]]",
        b"[compileMe] ARG",
        b"opaque\xff $array([compileMe])",
    ] {
        assert!(
            !closed_generic_byte_compilation(&source_module(source, &entry), &entry),
            "{source:?}"
        );
    }
    entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
    assert!(closed_generic_byte_compilation(
        &source_module(b"opaque\xff [absent [compileMe]]", &entry),
        &entry
    ));
    // An unresolved expanded head uses native invokeExpanded; it does not
    // choose a literal head's compiler. Its bracket children still require
    // the independently closed registrations checked above. The retained
    // native_expanded_head capture records the actual dynamic_bytecode.
    assert!(closed_generic_byte_compilation(
        &source_module(b"{*}$head ARG", &entry),
        &entry
    ));
}

#[test]
fn byte_generic_certificate_checks_bracket_hooks_beyond_advisory_index_depth() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"compileMe",
        NativeCompilerHookPresence::Present,
    ));
    let mut source = b"opaque\xff ".to_vec();
    for _ in 0..2000 {
        source.extend_from_slice(b"$a(");
    }
    source.extend_from_slice(b"[compileMe]");
    source.extend(std::iter::repeat_n(b')', 2000));
    let module = source_module(&source, &entry);
    assert!(!closed_generic_byte_compilation(&module, &entry));
    entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
    assert!(closed_generic_byte_compilation(&module, &entry));
}

#[test]
fn byte_generic_certificate_never_looks_up_a_dynamic_compiler_head() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    entry.closed = false;
    for source in [
        b"$head ARG".as_slice(),
        b"prefix${head} ARG",
        b"$array($key) ARG",
    ] {
        assert!(
            closed_generic_byte_compilation(&source_module(source, &entry), &entry),
            "{source:?}"
        );
    }
    assert!(!closed_generic_byte_compilation(
        &source_module(b"$head [literalChild]", &entry),
        &entry
    ));
    entry.closed = true;
    assert!(closed_generic_byte_compilation(
        &source_module(b"[absent] ARG", &entry),
        &entry
    ));
    entry.name_protocol = None;
    assert!(!closed_generic_byte_compilation(
        &source_module(b"$head ARG", &entry),
        &entry
    ));
}

#[test]
fn byte_generic_certificate_keeps_native_constant_head_rules_by_release() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = version.dialect_profile_name();
        let mut entry = entry(version);
        entry.commands.push(binding(
            &entry.namespaces[0],
            b"worker",
            NativeCompilerHookPresence::Present,
        ));
        for source in [b"{worker}".as_slice(), b"\"worker\""] {
            assert!(
                !closed_generic_byte_compilation(&source_module(source, &entry), &entry),
                "{dialect} {source:?}"
            );
        }
        let escaped = source_module(b"wor\\ker", &entry);
        assert_eq!(
            closed_generic_byte_compilation(&escaped, &entry),
            dialect == "tcl8.4" || dialect == "tcl8.5",
            "{dialect}"
        );
    }
}
