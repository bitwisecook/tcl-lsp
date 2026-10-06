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

//! Original compiler admission and native index-coordinate chronology.
use super::*;
use tcl_registry::native_compilation::NativeCompilationFrame;
use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;

fn emit_original<'a>(
    registry: &'a tcl_registry::CommandRegistry,
    entry: Option<&'a tcl_runtime_api::NativeCompilationEntry>,
    profile: &'static tcl_dialect::DialectProfile,
    source: &[u8],
) -> CodegenCtx<'a> {
    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
    let parsed = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::native(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
    .unwrap();
    let mut context = CodegenCtx::new(true, &["x", "i"], registry);
    context.native_entry = entry;
    context.invocation_dialect = Some(dialect);
    context.source_string_protocol = entry.and_then(|entry| entry.source_string_protocol);
    context.compiled_variable_protocol = entry.and_then(|entry| entry.compiled_variable_protocol);
    context.native_compilation.frame = NativeCompilationFrame::ProcedureCode;
    context.emit_native_words(&parsed.commands[0].words);
    context
}
#[test]
fn original_list_index_emission_retains_native_coordinates_and_compiler_boundaries() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        for (source, immediate) in [
            (b"lindex $x 0".as_slice(), engine != "tcl8.4"),
            (
                b"lindex $x end-1",
                matches!(engine, "tcl8.6" | "tcl9.0" | "tcl9.1"),
            ),
            (b"lindex $x $i", false),
        ] {
            let context = emit_original(&registry, Some(&entry), profile, source);
            assert!(!context.native_dependency_refusal, "{engine}/{source:?}");
            assert_eq!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.native_list_index.is_some()),
                immediate,
                "{engine}/{source:?}"
            );
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.native_compiler_selection.is_some()),
                "{engine}/{source:?}"
            );
            if source == b"lindex $x end-1" && immediate {
                let coordinate = context
                    .instructions
                    .iter()
                    .find_map(|instruction| instruction.native_list_index)
                    .unwrap();
                assert_eq!(coordinate.encoded(), -3);
                assert!(
                    context
                        .literals
                        .entries()
                        .iter()
                        .all(|literal| literal.bytes() != b"end-1")
                );
            }
        }
    }
}
#[test]
fn original_list_index_withdraws_unknown_or_missing_entry_but_keeps_actual_decline() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let entry = crate::environment_ingress::captured_native_entry(profile);
    let context = emit_original(&registry, None, profile, b"lindex $x 0");
    assert!(context.native_dependency_refusal);
    for (hook, refused) in [
        (NativeCompilerHookPresence::Unknown, true),
        (NativeCompilerHookPresence::Absent, false),
    ] {
        let mut changed = entry.clone();
        let token = changed
            .lookup_command_bytes(changed.current_namespace, b"lindex")
            .unwrap()
            .unwrap()
            .token;
        let binding = changed
            .commands
            .iter_mut()
            .find(|binding| binding.token == token)
            .unwrap();
        binding.compiler_hook = hook;
        binding.compiler = None;
        let context = emit_original(&registry, Some(&changed), profile, b"lindex $x 0");
        assert_eq!(context.native_dependency_refusal, refused);
        assert!(
            context
                .instructions
                .iter()
                .all(|instruction| instruction.native_list_index.is_none())
        );
    }
}
