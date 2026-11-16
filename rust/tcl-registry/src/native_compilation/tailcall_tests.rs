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

#[test]
fn native_tailcall_selection_and_namespace_stack_follow_the_actual_release() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::Tailcall,
        operation: SemanticOperationId::Invoke,
        body: NativeBodyCompilation::Inherit,
    };
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let arguments = ["target"];
        let words = InvocationWords::literals("tailcall", &arguments);
        let selected = spec.select(
            words,
            &[NativeCompilationWordShape::Literal],
            Some(dialect),
            context,
        );
        assert_eq!(
            matches!(selected, NativeCompilationSelection::Inline { .. }),
            version >= TclVersion::V8_6
        );
        let script = spec.select(
            words,
            &[NativeCompilationWordShape::Literal],
            Some(dialect),
            NativeCompilationContext {
                frame: NativeCompilationFrame::ScriptCode,
                ..context
            },
        );
        assert_eq!(script, NativeCompilationSelection::Generic);
        let expanded = spec.select(
            words,
            &[NativeCompilationWordShape::Expanded],
            Some(dialect),
            context,
        );
        assert_eq!(
            matches!(expanded, NativeCompilationSelection::Inline { .. }),
            version == TclVersion::V9_1
        );
        let empty = spec.select(
            InvocationWords::literals("tailcall", &[]),
            &[],
            Some(dialect),
            context,
        );
        assert_eq!(
            matches!(empty, NativeCompilationSelection::Inline { .. }),
            version == TclVersion::V9_1
        );
        let stack = NativeTailcallStack::for_invocation(dialect, 1, false);
        assert_eq!(
            stack,
            match version {
                TclVersion::V8_4 | TclVersion::V8_5 => None,
                TclVersion::V8_6 | TclVersion::V9_0 =>
                    Some(NativeTailcallStack::NamespaceAfterArguments),
                TclVersion::V9_1 => Some(NativeTailcallStack::NamespaceBeforeArguments),
            }
        );
    }
    assert_eq!(
        NativeTailcallStack::for_invocation(
            InvocationDialect::for_version(TclVersion::V9_1),
            0,
            false
        ),
        Some(NativeTailcallStack::NamespacePrefixedList)
    );
}
#[test]
fn nested_named_invocation_retains_only_written_selectors() {
    static LOOKUP: NativeCompilerImplementationLookup = NativeCompilerImplementationLookup {
        ensemble: "::tcl::binary::encode",
        member: "hex",
        slot: "::tcl::binary::encode::hex",
        command: "binary",
        prepended: &["encode", "hex"],
    };
    for (_head, args, from) in [
        ("binary", vec!["encode", "hex", "DATA"], 2),
        ("::tcl::binary::encode", vec!["hex", "DATA"], 1),
        ("::tcl::binary::encode::hex", vec!["DATA"], 0),
    ] {
        let shapes = vec![NativeCompilationWordShape::Literal; args.len()];
        assert!(matches!(
            named_ensemble_grammar(&LOOKUP, TclVersion::V8_6, crate::Arity::exact(1),
                NamedCompilerOperandView::OriginalArguments { selectors_retained: from }, &shapes, TclVersion::V8_6),
            NativeCompilationSelection::NamedInvocation { arguments_from, .. } if arguments_from == from
        ));
        assert_eq!(
            named_ensemble_grammar(
                &LOOKUP,
                TclVersion::V8_6,
                crate::Arity::exact(1),
                NamedCompilerOperandView::OriginalArguments {
                    selectors_retained: from
                },
                &shapes,
                TclVersion::V8_5
            ),
            NativeCompilationSelection::Generic
        );
        assert_eq!(
            named_ensemble_grammar(
                &LOOKUP,
                TclVersion::V8_6,
                crate::Arity::exact(1),
                NamedCompilerOperandView::OriginalArguments {
                    selectors_retained: from
                },
                &shapes[..shapes.len() - 1],
                TclVersion::V8_6
            ),
            NativeCompilationSelection::Generic
        );
    }
}
