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

use super::{
    NativeClockReading as Reading, NativeCompilationWordShape as Shape, clock_reading_value,
};
use tcl_dialect::TclVersion;

#[test]
fn clock_reading_selects_only_proven_forms_in_tcl_8_6_and_9_0() {
    for version in [TclVersion::V8_6, TclVersion::V9_0] {
        for (reading, selector) in [
            (Reading::Clicks, 0),
            (Reading::Microseconds, 1),
            (Reading::Milliseconds, 2),
            (Reading::Seconds, 3),
        ] {
            assert_eq!(
                clock_reading_value(reading, None, &[], version),
                Some(selector)
            );
            assert_eq!(
                clock_reading_value(reading, None, &[], TclVersion::V9_1),
                None
            );
            assert_eq!(
                clock_reading_value(reading, None, &[], TclVersion::V8_5),
                None
            );
        }
        for (word, selector) in [
            (b"-mic".as_slice(), 1),
            (b"-mil".as_slice(), 2),
            (b"-microseconds".as_slice(), 1),
        ] {
            assert_eq!(
                clock_reading_value(Reading::Clicks, Some(word), &[Shape::Literal], version),
                Some(selector)
            );
        }
        for (word, shape) in [
            (b"-mi".as_slice(), Shape::Literal),
            (b"-mic".as_slice(), Shape::BackslashLiteral),
            (b"-other".as_slice(), Shape::Literal),
        ] {
            assert_eq!(
                clock_reading_value(Reading::Clicks, Some(word), &[shape], version),
                None
            );
        }
    }
}
#[test]
fn registered_clock_worker_emits_the_pinned_c90_selectors() {
    use crate::native_compiler_words::NativeCompilerWords;
    use crate::native_instruction_plan::{
        NativeInstructionPlan, native_registered_worker_instruction_plan,
    };
    let version = TclVersion::V9_0;
    let dialect = crate::InvocationDialect::for_version(version);
    let registry = crate::CommandRegistry::build_default();
    let context = super::NativeCompilationContext {
        mode: super::NativeCompilationMode::BytecodeObject,
        frame: super::NativeCompilationFrame::ProcedureCode,
        ..super::NativeCompilationContext::default()
    };
    for (source, member, which) in [
        ("clock clicks -mic", "clicks", 1),
        ("clock clicks -mil", "clicks", 2),
        ("clock seconds", "seconds", 3),
    ] {
        let source_image = tcl_lexer::SourceImage::native(source.as_bytes());
        let parsed = tcl_lexer::native_script_words_in(
            source_image,
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
            .native_compilation_for_registration(&format!("::tcl::clock::{member}"), dialect)
            .unwrap();
        let selection =
            spec.select_registered_worker_native_words(&words, 2, Some(dialect), context);
        assert!(
            matches!(selection, super::NativeCompilationSelection::Inline { .. }),
            "{source}: {selection:?}"
        );
        assert_eq!(
            native_registered_worker_instruction_plan(spec, selection, &words, 2, dialect, context),
            Ok(NativeInstructionPlan::ClockRead(which))
        );
    }
}
