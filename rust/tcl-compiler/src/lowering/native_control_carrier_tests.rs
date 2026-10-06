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

//! Executable native control retains its compiler's original source vector.

use super::*;
use crate::compile_service::BytecodeCompileService;
use tcl_runtime_api::{CompileService, ScriptCompileTarget};

const ORIGINAL_MATHOP_TEST: &str = "if {![llength [info commands ::tcl::mathop::+]]} {set marker UNAVAILABLE} else {llength [info commands ::tcl::mathop::+]}";

fn native_control_module(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
    entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
) -> Module {
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let options =
        entry.map(|entry| BytecodeCompileService::native_entry_options(entry, Some(profile)));
    lower_script_module_for_bytecode_with_options(
        source,
        "",
        &registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        Some(profile),
        false,
        options,
    )
}

#[test]
fn original_control_compiler_enters_the_original_expression_worklist() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let module = native_control_module(ORIGINAL_MATHOP_TEST, profile, Some(&entry));
        let [
            Statement::Call {
                tokens: Some(tokens),
                ..
            },
        ] = module.top_level.statements.as_slice()
        else {
            panic!("{name}: retained control invocation");
        };
        assert!(matches!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_structured_compilation(tokens)
                .unwrap()
                .recipe(),
            tcl_registry::native_instruction_plan::NativeInstructionPlan::Control(_)
        ));
        let compiled = BytecodeCompileService::for_profile(profile)
            .compile_script_with_entry(
                ScriptCompileTarget {
                    source: ORIGINAL_MATHOP_TEST,
                    namespace: "",
                },
                profile,
                &entry,
            )
            .unwrap();
        assert_ne!(
            compiled.top_level.native_compilation_preflight,
            tcl_runtime_api::NativeCompilationPreflight::ProviderRequired
        );
        assert_original_control_guard(&compiled.top_level, &entry);
    }
}

fn assert_original_control_guard(
    compiled: &tcl_bytecode::FunctionAsm,
    entry: &tcl_runtime_api::NativeCompilationEntry,
) {
    use tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite;
    let root = entry
        .namespaces
        .iter()
        .find(|namespace| namespace.path == tcl_core_types::ByteNamespacePath::root())
        .expect("the actual native entry retains its root namespace owner");
    let original = entry
        .lookup_command_bytes(root.token, b"if")
        .unwrap()
        .unwrap();
    let retained = compiled
        .instructions
        .iter()
        .find_map(|instruction| {
            let selection = instruction.native_compiler_selection.as_ref()?;
            let NativeCompilerSelectionPrerequisite::Command(required) = &selection.prerequisite
            else {
                return None;
            };
            (required.invocation_word.as_bytes() == b"if").then_some(required)
        })
        .expect("original control keeps its instruction-time compiler guard");
    assert_eq!(retained.interpreter, entry.interpreter);
    assert_eq!(retained.token, original.token);
    assert_eq!(
        retained.implementation_generation,
        original.implementation_generation
    );
    assert_eq!(Some(&retained.compiler), original.compiler.as_ref());
}

#[test]
fn original_control_carrier_rejects_changed_words_and_absent_compiler_entry() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let entry = crate::environment_ingress::captured_native_entry(profile);
    let mut module = native_control_module(ORIGINAL_MATHOP_TEST, profile, Some(&entry));
    let [
        Statement::Call {
            tokens: Some(tokens),
            ..
        },
    ] = module.top_level.statements.as_mut_slice()
    else {
        panic!("retained control invocation");
    };
    let original = tokens.source_binding.clone().unwrap();
    tokens.argv_texts[1] = "1".to_owned();
    assert!(original.original_structured_compilation(tokens).is_none());
    let mut unknown = entry.clone();
    unknown.commands.clear();
    unknown.closed = false;
    let module = native_control_module(ORIGINAL_MATHOP_TEST, profile, Some(&unknown));
    assert!(module.top_level.statements.iter().all(|statement| {
        statement
            .tokens()
            .and_then(|tokens| {
                tokens
                    .source_binding
                    .as_ref()?
                    .original_structured_compilation(tokens)
            })
            .is_none()
    }));
}

#[test]
fn renamed_or_aliased_control_has_no_original_stock_compiler_carrier() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let entry = crate::environment_ingress::captured_native_entry(profile);
    for source in ["interp alias {} conditional {} if; conditional 1 {set x YES}"] {
        let module = native_control_module(source, profile, Some(&entry));
        let statement = module.top_level.statements.last().unwrap();
        assert!(statement.tokens().is_some());
        assert!(
            statement
                .tokens()
                .and_then(|tokens| tokens
                    .source_binding
                    .as_ref()?
                    .original_structured_compilation(tokens))
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn replaced_control_preserves_initial_compiler_selection_for_runtime_replay() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let entry = crate::environment_ingress::captured_native_entry(profile);
    let source = "rename if saved; proc if {args} {return CUSTOM}; if 1 {set x YES}";
    let module = native_control_module(source, profile, Some(&entry));
    let tokens = module
        .top_level
        .statements
        .last()
        .unwrap()
        .tokens()
        .unwrap();
    assert!(matches!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_structured_compilation(tokens)
            .unwrap()
            .recipe(),
        tcl_registry::native_instruction_plan::NativeInstructionPlan::Control(_)
    ));
    let compiled = BytecodeCompileService::for_profile(profile)
        .compile_script_with_entry(
            ScriptCompileTarget {
                source,
                namespace: "",
            },
            profile,
            &entry,
        )
        .unwrap();
    assert_original_control_guard(&compiled.top_level, &entry);
    assert!(compiled.top_level.instructions.iter().any(|instruction| {
        instruction.native_compiler_selection.is_some()
            && instruction.source_cmd_text.bytes() == b"if 1 {set x YES}"
    }));
}
