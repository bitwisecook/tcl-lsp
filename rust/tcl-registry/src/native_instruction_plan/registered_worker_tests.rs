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

//! Private-worker operand selection remains distinct from a public selector.

use super::*;
use crate::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_dialect::TclVersion;

#[test]
fn registered_array_worker_recipe_preserves_original_operands_and_selection_purpose() {
    for (version, engine) in [
        (TclVersion::V8_6, "tcl8.6"),
        (TclVersion::V9_0, "tcl9.0"),
        (TclVersion::V9_1, "tcl9.1"),
    ] {
        let dialect = InvocationDialect::for_version(version);
        let registry = crate::model::ingress::static_context_for_profile(
            crate::model::ingress::resolve_environment(engine).unit_profile(),
        )
        .commands();
        let context = NativeCompilationContext {
            mode: crate::native_compilation::NativeCompilationMode::BytecodeObject,
            frame: crate::native_compilation::NativeCompilationFrame::ProcedureCode,
            loop_depth: 0,
            catch_depth: Some(0),
        };
        for (source, identity) in [
            (b"array exists a".as_slice(), "tcl::array::exists"),
            (b"array set a {}".as_slice(), "tcl::array::set"),
            (b"array unset a".as_slice(), "tcl::array::unset"),
        ] {
            let parsed = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::native(source),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            )
            .unwrap();
            let words = NativeCompilerWords::capture(
                &parsed.commands[0].words,
                dialect.native_string_protocol().unwrap(),
            )
            .unwrap();
            let spec = registry
                .native_compilation_for_registration(identity, dialect)
                .unwrap();
            let selection =
                spec.select_registered_worker_native_words(&words, 2, Some(dialect), context);
            assert!(matches!(
                selection,
                NativeCompilationSelection::Inline { .. }
            ));
            assert_eq!(
                native_instruction_plan(spec, selection, &words, 2, dialect, context),
                Err(NativeInstructionPlanUnavailable::Selection)
            );
            let NativeInstructionPlan::Array(array) = native_registered_worker_instruction_plan(
                spec, selection, &words, 2, dialect, context,
            )
            .unwrap() else {
                panic!("original selected Array worker")
            };
            assert_eq!(
                array.instruction.unwrap().operand,
                NativeCompilerWordOperand::Original(2)
            );
            assert_eq!(
                native_registered_worker_instruction_plan(
                    spec,
                    NativeCompilationSelection::Unknown,
                    &words,
                    2,
                    dialect,
                    context
                ),
                Err(NativeInstructionPlanUnavailable::Selection)
            );
            assert_eq!(
                native_registered_worker_instruction_plan(
                    spec,
                    selection,
                    &words,
                    2,
                    dialect,
                    NativeCompilationContext {
                        mode: crate::native_compilation::NativeCompilationMode::Direct,
                        ..context
                    }
                ),
                Err(NativeInstructionPlanUnavailable::Selection)
            );
        }
    }
}
