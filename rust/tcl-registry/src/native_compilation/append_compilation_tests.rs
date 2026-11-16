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

fn selection(
    kind: NativeAppendKind,
    version: TclVersion,
    frame: NativeCompilationFrame,
    arguments: &[crate::InvocationWord<'_>],
    shapes: &[NativeCompilationWordShape],
) -> NativeCompilationSelection {
    NativeCompilationSpec {
        grammar: NativeCompilationGrammar::VariableAppend(kind),
        operation: SemanticOperationId::StructuredLowering(
            crate::hooks::LoweringHookId::AppendOrLappend,
        ),
        body: NativeBodyCompilation::Inherit,
    }
    .select(
        InvocationWords::structured(crate::InvocationWord::Literal("selected"), arguments),
        shapes,
        Some(InvocationDialect::for_version(version)),
        NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame,
            loop_depth: 0,
            catch_depth: Some(0),
        },
    )
}

#[test]
fn string_and_list_append_compilers_preserve_native_release_and_frame_rules() {
    use crate::InvocationWord::{Dynamic, Literal as Word};
    use NativeCompilationFrame::{ProcedureCode, ScriptCode};
    use NativeCompilationSelection::{Generic, Inline};
    use NativeCompilationWordShape::{Literal, Substituted};
    for (version, modern, latest) in [
        (TclVersion::V8_4, false, false),
        (TclVersion::V8_5, false, false),
        (TclVersion::V8_6, true, false),
        (TclVersion::V9_0, true, false),
        (TclVersion::V9_1, true, true),
    ] {
        let inline = Inline {
            operation: SemanticOperationId::StructuredLowering(
                crate::hooks::LoweringHookId::AppendOrLappend,
            ),
            guard: NativeCompilationGuard::for_c_version(version),
        };
        for kind in [NativeAppendKind::String, NativeAppendKind::List] {
            assert_eq!(
                selection(
                    kind,
                    version,
                    ProcedureCode,
                    &[Word("x"), Dynamic],
                    &[Literal, Substituted]
                ),
                inline
            );
            assert_eq!(
                selection(
                    kind,
                    version,
                    ProcedureCode,
                    &[Word("x"), Dynamic, Word("B")],
                    &[Literal, Substituted, Literal]
                ),
                if modern { inline } else { Generic }
            );
        }
        assert_eq!(
            selection(
                NativeAppendKind::String,
                version,
                ScriptCode,
                &[Word("x"), Dynamic],
                &[Literal, Substituted]
            ),
            inline
        );
        assert_eq!(
            selection(
                NativeAppendKind::String,
                version,
                ProcedureCode,
                &[Word("::x"), Dynamic, Word("B")],
                &[Literal, Substituted, Literal]
            ),
            Generic
        );
        assert_eq!(
            selection(
                NativeAppendKind::String,
                version,
                ProcedureCode,
                &[Dynamic, Dynamic, Word("B")],
                &[Substituted, Substituted, Literal]
            ),
            Generic
        );
        assert_eq!(
            selection(
                NativeAppendKind::List,
                version,
                ScriptCode,
                &[Word("::a(k)"), Dynamic, Word("B")],
                &[Literal, Substituted, Literal]
            ),
            if modern { inline } else { Generic }
        );
        assert_eq!(
            selection(
                NativeAppendKind::List,
                version,
                ProcedureCode,
                &[Word("x")],
                &[Literal]
            ),
            if latest { inline } else { Generic }
        );
    }
}

#[test]
fn expanded_list_append_requires_the_actual_tcl91_native_protocol() {
    use crate::InvocationWord::{Expanded, Literal};
    let value_shapes = [
        NativeCompilationWordShape::Literal,
        NativeCompilationWordShape::Expanded,
    ];
    assert!(matches!(
        selection(
            NativeAppendKind::List,
            TclVersion::V9_1,
            NativeCompilationFrame::ScriptCode,
            &[Literal("x"), Expanded],
            &value_shapes
        ),
        NativeCompilationSelection::Inline { .. }
    ));
    assert_eq!(
        selection(
            NativeAppendKind::List,
            TclVersion::V9_0,
            NativeCompilationFrame::ProcedureCode,
            &[Literal("x"), Expanded],
            &value_shapes
        ),
        NativeCompilationSelection::Generic
    );
    assert_eq!(
        selection(
            NativeAppendKind::String,
            TclVersion::V9_1,
            NativeCompilationFrame::ProcedureCode,
            &[Literal("x"), Expanded],
            &value_shapes
        ),
        NativeCompilationSelection::Generic
    );
    assert_eq!(
        selection(
            NativeAppendKind::List,
            TclVersion::V9_1,
            NativeCompilationFrame::ProcedureCode,
            &[Expanded, Literal("x")],
            &[
                NativeCompilationWordShape::Expanded,
                NativeCompilationWordShape::Literal
            ]
        ),
        NativeCompilationSelection::Generic
    );
}
